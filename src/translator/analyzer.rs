use crate::translator::ast::{RawAbstractSyntaxNode, RawAbstractSyntaxTree};
use crate::translator::common::{RawExpression, ResBox, Type, TypedExpression, Variable};
use crate::translator::expression::{
    Expression, is_arithmetic_binary_op, is_compering_binary_op,
    is_logical_binary_op, is_relational_binary_op,
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum TypedAbstractSyntaxNode {
    If {
        condition: TypedExpression,
    },
    ElseIf {
        condition: TypedExpression,
    },
    Else,
    While {
        condition: TypedExpression,
    },
    For {
        initializer: Option<Box<TypedAbstractSyntaxNode>>,
        condition: Option<TypedExpression>,
        increment: Option<TypedExpression>,
    },
    Callable {
        result_type: Type,
        name: String,
        arguments: Vec<Variable>,
    },
    Class {
        name: String,
    },
    Expression {
        expression: TypedExpression,
    },
    Declaration {
        typ: Type,
        name: String,
        expression: Option<TypedExpression>,
    },
    Return {
        value: Option<TypedExpression>,
    },
    Break,
    Continue,
    Scope,
    File,
}

#[derive(Debug, Clone)]
pub struct TypedAbstractSyntaxTree {
    pub node: TypedAbstractSyntaxNode,
    pub children: Vec<TypedAbstractSyntaxTree>,
}

impl TypedAbstractSyntaxTree {
    pub fn new(node: TypedAbstractSyntaxNode) -> Self {
        Self {
            node,
            children: vec![],
        }
    }
    pub fn with_children(
        node: TypedAbstractSyntaxNode,
        children: Vec<TypedAbstractSyntaxTree>,
    ) -> Self {
        Self { node, children }
    }
}

type FunctionInfo = (Type, Vec<Type>);

#[derive(Debug, Clone)]
struct ClassInfo {
    fields: HashMap<String, Type>,
    methods: HashMap<String, (Type, Vec<Type>)>,
}

impl ClassInfo {
    fn new() -> ClassInfo {
        ClassInfo {
            fields: HashMap::new(),
            methods: HashMap::new(),
        }
    }
}

struct SemanticTable {
    scopes: Vec<HashMap<String, Type>>,
    stacktrace: Vec<RawAbstractSyntaxNode>,
    functions: HashMap<String, FunctionInfo>,
    classes: HashMap<String, ClassInfo>,
}

impl SemanticTable {
    fn new() -> Self {
        SemanticTable {
            scopes: vec![HashMap::new()],
            stacktrace: vec![],
            functions: HashMap::new(),
            classes: HashMap::new(),
        }
    }

    fn find_var(&self, name: &str) -> Option<&Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(typ) = scope.get(name) {
                return Some(typ);
            }
        }
        None
    }

    fn current_class_context(&self) -> Option<String> {
        for node in self.stacktrace.iter().rev() {
            if let RawAbstractSyntaxNode::Class { name } = node {
                return Some(name.clone());
            }
        }
        None
    }

    fn collect_definitions(&mut self, ast: &RawAbstractSyntaxTree) -> ResBox<()> {
        match &ast.node {
            RawAbstractSyntaxNode::Class { name } => {
                if self.classes.contains_key(name) {
                    return Err(format!("Class '{}' already exists", name).into());
                }
                self.classes.insert(name.clone(), ClassInfo::new());
                self.stacktrace.push(ast.node.clone());
                for child in &ast.children {
                    self.collect_definitions(child)?;
                }
                self.stacktrace.pop();
            }
            RawAbstractSyntaxNode::Callable {
                result_type,
                name,
                arguments,
            } => {
                let arg_types: Vec<Type> = arguments.iter().map(|v| v.typ.clone()).collect();
                if let Some(class_name) = self.current_class_context() {
                    let class_info = self.classes.get_mut(&class_name).unwrap();
                    if class_info.methods.contains_key(name) {
                        return Err(format!("Function '{}' already exists", name).into());
                    }
                    class_info
                        .methods
                        .insert(name.clone(), (result_type.clone(), arg_types));
                } else {
                    if self.functions.contains_key(name) {
                        return Err(format!("Function '{}' already exists", name).into());
                    }
                    self.functions
                        .insert(name.clone(), (result_type.clone(), arg_types));
                }
            }
            RawAbstractSyntaxNode::Declaration { typ, name, .. } => {
                if let Some(class_name) = self.current_class_context()
                    && matches!(
                        self.stacktrace.last(),
                        Some(RawAbstractSyntaxNode::Class { .. })
                    )
                {
                    let class_info = self.classes.get_mut(&class_name).unwrap();
                    class_info.fields.insert(name.clone(), typ.clone());
                }
            }
            RawAbstractSyntaxNode::File | RawAbstractSyntaxNode::Scope => {
                self.stacktrace.push(ast.node.clone());
                for child in &ast.children {
                    self.collect_definitions(child)?;
                }
                self.stacktrace.pop();
            }
            _ => {}
        }
        Ok(())
    }

    fn analyze(&mut self, ast: &RawAbstractSyntaxTree) -> ResBox<TypedAbstractSyntaxTree> {
        self.stacktrace.push(ast.node.clone());

        let (typed_node, typed_children) = match &ast.node {
            RawAbstractSyntaxNode::If { condition } => {
                let typed_condition = self.analyze_expression(condition)?;
                if typed_condition.get_type() != Type::Bool {
                    return Err("Condition must be bool".into());
                }
                self.scopes.push(HashMap::new());
                let children = self.analyze_children(&ast.children)?;
                self.scopes.pop();
                (
                    TypedAbstractSyntaxNode::If {
                        condition: typed_condition,
                    },
                    children,
                )
            }
            RawAbstractSyntaxNode::While { condition } => {
                let typed_condition = self.analyze_expression(condition)?;
                if typed_condition.get_type() != Type::Bool {
                    return Err("Condition must be bool".into());
                }
                self.scopes.push(HashMap::new());
                let children = self.analyze_children(&ast.children)?;
                self.scopes.pop();
                (
                    TypedAbstractSyntaxNode::While {
                        condition: typed_condition,
                    },
                    children,
                )
            }
            RawAbstractSyntaxNode::Callable {
                result_type,
                name,
                arguments,
            } => {
                self.scopes.push(HashMap::new());
                for arg in arguments {
                    self.scopes
                        .last_mut()
                        .unwrap()
                        .insert(arg.name.clone(), arg.typ.clone());
                }
                let children = self.analyze_children(&ast.children)?;
                self.scopes.pop();

                if !self.node_guarantees_return(ast) {
                    return Err(format!(
                        "Not all code paths return a value in function '{}'",
                        name
                    )
                    .into());
                }

                (
                    TypedAbstractSyntaxNode::Callable {
                        result_type: result_type.clone(),
                        name: name.clone(),
                        arguments: arguments.clone(),
                    },
                    children,
                )
            }
            RawAbstractSyntaxNode::Class { name } => {
                self.scopes.push(HashMap::new());
                for child in &ast.children {
                    if let RawAbstractSyntaxNode::Declaration {
                        typ,
                        name,
                        expression,
                    } = &child.node
                    {
                        let typed_expr = match expression {
                            Some(e) => Some(self.analyze_expression(e)?),
                            None => None,
                        };
                        self.analyze_declaration(typ, name, &typed_expr)?;
                    }
                }

                let mut typed_children = vec![];
                for child in &ast.children {
                    if !matches!(child.node, RawAbstractSyntaxNode::Declaration { .. }) {
                        typed_children.push(self.analyze(child)?);
                    } else if let RawAbstractSyntaxNode::Declaration {
                        typ,
                        name,
                        expression,
                    } = &child.node
                    {
                        let typed_expr = match expression {
                            Some(e) => Some(self.analyze_expression(e)?),
                            None => None,
                        };
                        typed_children.push(TypedAbstractSyntaxTree::new(
                            TypedAbstractSyntaxNode::Declaration {
                                typ: typ.clone(),
                                name: name.clone(),
                                expression: typed_expr,
                            },
                        ));
                    }
                }
                self.scopes.pop();
                (
                    TypedAbstractSyntaxNode::Class { name: name.clone() },
                    typed_children,
                )
            }
            RawAbstractSyntaxNode::File
            | RawAbstractSyntaxNode::Scope
            | RawAbstractSyntaxNode::Else => {
                self.scopes.push(HashMap::new());
                let children = self.analyze_children(&ast.children)?;
                self.scopes.pop();
                let typed_node = match &ast.node {
                    RawAbstractSyntaxNode::File => TypedAbstractSyntaxNode::File,
                    RawAbstractSyntaxNode::Scope => TypedAbstractSyntaxNode::Scope,
                    RawAbstractSyntaxNode::Else => TypedAbstractSyntaxNode::Else,
                    _ => unreachable!(),
                };
                (typed_node, children)
            }
            RawAbstractSyntaxNode::Expression { expression } => {
                let typed_expr = self.analyze_expression(expression)?;
                (
                    TypedAbstractSyntaxNode::Expression {
                        expression: typed_expr,
                    },
                    vec![],
                )
            }
            RawAbstractSyntaxNode::Declaration {
                typ,
                name,
                expression,
            } => {
                let typed_expr = match expression {
                    Some(e) => Some(self.analyze_expression(e)?),
                    None => None,
                };
                if !matches!(
                    self.stacktrace.get(self.stacktrace.len() - 2),
                    Some(RawAbstractSyntaxNode::Class { .. })
                ) {
                    self.analyze_declaration(typ, name, &typed_expr)?;
                }
                (
                    TypedAbstractSyntaxNode::Declaration {
                        typ: typ.clone(),
                        name: name.clone(),
                        expression: typed_expr,
                    },
                    vec![],
                )
            }
            RawAbstractSyntaxNode::Return { value } => {
                let function_type = self
                    .stacktrace
                    .iter()
                    .rev()
                    .find_map(|n| {
                        if let RawAbstractSyntaxNode::Callable { result_type, .. } = n {
                            Some(result_type)
                        } else {
                            None
                        }
                    })
                    .ok_or("Return outside function")?;
                let typed_value = match value {
                    Some(expression) => Some(self.analyze_expression(expression)?),
                    None => None,
                };
                let typ = typed_value.as_ref().map_or(Type::Void, |v| v.get_type());
                if typ != *function_type {
                    return Err("Return type mismatch".into());
                }
                (
                    TypedAbstractSyntaxNode::Return { value: typed_value },
                    vec![],
                )
            }
            RawAbstractSyntaxNode::Break | RawAbstractSyntaxNode::Continue => {
                if !self.stacktrace.iter().any(|n| {
                    matches!(
                        n,
                        RawAbstractSyntaxNode::While { .. } | RawAbstractSyntaxNode::For { .. }
                    )
                }) {
                    return Err("Jump outside loop".into());
                }
                let typed_node = match &ast.node {
                    RawAbstractSyntaxNode::Break => TypedAbstractSyntaxNode::Break,
                    RawAbstractSyntaxNode::Continue => TypedAbstractSyntaxNode::Continue,
                    _ => unreachable!(),
                };
                (typed_node, vec![])
            }
            _ => unreachable!(),
        };

        self.stacktrace.pop();
        Ok(TypedAbstractSyntaxTree::with_children(
            typed_node,
            typed_children,
        ))
    }

    fn analyze_children(
        &mut self,
        children: &[RawAbstractSyntaxTree],
    ) -> ResBox<Vec<TypedAbstractSyntaxTree>> {
        let mut typed_children = vec![];
        for child in children {
            typed_children.push(self.analyze(child)?);
        }
        Ok(typed_children)
    }

    fn is_valid_type(&self, typ: &Type) -> bool {
        match typ {
            Type::Class(c) => self.classes.contains_key(c),
            Type::Array(inner, _) => self.is_valid_type(inner),
            _ => true,
        }
    }

    fn analyze_declaration(
        &mut self,
        typ: &Type,
        name: &str,
        expression: &Option<TypedExpression>,
    ) -> ResBox<()> {
        if let Type::Class(c) = typ
            && !self.classes.contains_key(c)
        {
            return Err(format!("Unknown type {}", c).into());
        }
        if let Some(expression) = expression
            && expression.get_type() != *typ
        {
            return Err("Declaration type mismatch".into());
        }
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name.to_owned(), typ.clone());
        Ok(())
    }

    fn analyze_expression(&self, expression: &RawExpression) -> ResBox<TypedExpression> {
        match expression {
            Expression::Literal { value, .. } => {
                let typ = get_literal_type(value)?;
                Ok(Expression::Literal {
                    typ,
                    value: value.clone(),
                })
            }
            Expression::Variable { name, .. } => {
                let typ = self
                    .find_var(name)
                    .cloned()
                    .ok_or(format!("Undefined: {}", name))?;
                Ok(Expression::Variable {
                    typ,
                    name: name.clone(),
                })
            }
            Expression::BinaryOperator {
                left,
                operator,
                right,
                ..
            } => {
                let typed_left = self.analyze_expression(left)?;
                let typed_right = self.analyze_expression(right)?;
                let left_type = typed_left.get_type();
                let right_type = typed_right.get_type();

                if left_type != right_type {
                    return Err("Binary operator type mismatch".into());
                }

                if is_arithmetic_binary_op(operator)
                    && left_type != Type::Int
                    && left_type != Type::Array(Box::new(Type::Int), 4)
                {
                    return Err(format!(
                        "Arithmetic operations can only be applied to type int or array[4], found {:?}",
                        left_type
                    )
                    .into());
                }

                if is_logical_binary_op(operator) && left_type != Type::Bool {
                    return Err(
                        "Logical operations (&&, ||) can only be applied to type bool".into(),
                    );
                }

                if is_relational_binary_op(operator) && left_type != Type::Int {
                    return Err(
                        "Relational operations (<, >, <=, >=) can only be applied to type int"
                            .into(),
                    );
                }

                let result_type = if is_compering_binary_op(operator) {
                    Type::Bool
                } else {
                    left_type
                };

                Ok(Expression::BinaryOperator {
                    typ: result_type,
                    left: Box::new(typed_left),
                    operator: operator.clone(),
                    right: Box::new(typed_right),
                })
            }
            Expression::FunctionCall {
                name, arguments, ..
            } => {
                let (ret, parameters) = self
                    .functions
                    .get(name)
                    .ok_or(format!("Function {} not found", name))?;
                let typed_args = self.analyze_arguments(parameters, arguments)?;
                Ok(Expression::FunctionCall {
                    typ: ret.clone(),
                    name: name.clone(),
                    arguments: typed_args,
                })
            }
            Expression::MethodCall {
                object,
                name: method,
                arguments,
                ..
            } => {
                let typed_object = self.analyze_expression(object)?;
                if let Type::Class(class_name) = typed_object.get_type() {
                    let class = self.classes.get(&class_name).ok_or("Class not found")?;
                    let (ret, parameters) = class
                        .methods
                        .get(method)
                        .ok_or(format!("Method {} not found in {}", method, class_name))?;
                    let typed_arguments = self.analyze_arguments(parameters, arguments)?;
                    Ok(Expression::MethodCall {
                        typ: ret.clone(),
                        object: Box::new(typed_object),
                        name: method.clone(),
                        arguments: typed_arguments,
                    })
                } else {
                    Err("Method call on non-object".into())
                }
            }
            Expression::Field {
                object,
                name: member,
                ..
            } => {
                let typed_object = self.analyze_expression(object)?;
                if let Type::Class(class_name) = typed_object.get_type() {
                    let class = self.classes.get(&class_name).ok_or("Class not found")?;
                    let field_type = class
                        .fields
                        .get(member)
                        .cloned()
                        .ok_or(format!("Field {} not found in {}", member, class_name))?;
                    Ok(Expression::Field {
                        typ: field_type,
                        object: Box::new(typed_object),
                        name: member.clone(),
                    })
                } else {
                    Err("Field access on non-object".into())
                }
            }
            Expression::Index {
                expression, index, ..
            } => {
                let typed_expression = self.analyze_expression(expression)?;
                let typed_index = self.analyze_expression(index)?;
                if typed_index.get_type() != Type::Int {
                    return Err("Array index must be int".into());
                }

                if let Expression::Variable { name, typ } = &typed_expression
                    && let Expression::Literal { value, .. } = &typed_index
                    && let Ok(index) = value.parse::<i64>()
                    && let Type::Array(_, arr_size) = typ
                    && (index < 0 || index >= *arr_size as i64)
                {
                    return Err(format!(
                        "Index out of bounds: array '{}' has size {}, but accessed at index {}",
                        name, arr_size, index
                    )
                    .into());
                }

                if let Type::Array(inner_type, _) = typed_expression.get_type() {
                    Ok(Expression::Index {
                        typ: *inner_type,
                        expression: Box::new(typed_expression),
                        index: Box::new(typed_index),
                    })
                } else {
                    Err("Indexing non-array".into())
                }
            }
            Expression::Assign { name, value, .. } => {
                let variable_type = self
                    .find_var(name)
                    .ok_or(format!("Undefined {}", name))?
                    .clone();
                let typed_value = self.analyze_expression(value)?;
                if variable_type != typed_value.get_type() {
                    return Err("Assign type mismatch".into());
                }
                Ok(Expression::Assign {
                    typ: variable_type,
                    name: name.clone(),
                    value: Box::new(typed_value),
                })
            }
            Expression::AssignField {
                object,
                name: member,
                value,
                ..
            } => {
                let typed_object = self.analyze_expression(object)?;
                let typed_value = self.analyze_expression(value)?;
                if let Type::Class(c) = typed_object.get_type() {
                    let field_type = self
                        .classes
                        .get(&c)
                        .unwrap()
                        .fields
                        .get(member)
                        .ok_or("Field not found")?
                        .clone();
                    if field_type != typed_value.get_type() {
                        return Err("Field assign mismatch".into());
                    }
                    Ok(Expression::AssignField {
                        typ: field_type,
                        object: Box::new(typed_object),
                        name: member.clone(),
                        value: Box::new(typed_value),
                    })
                } else {
                    Err("Not an object".into())
                }
            }
            Expression::AssignIndex {
                expression,
                index,
                value,
                ..
            } => {
                let typed_expression = self.analyze_expression(expression)?;
                let typed_index = self.analyze_expression(index)?;
                let typed_value = self.analyze_expression(value)?;
                if typed_index.get_type() != Type::Int {
                    return Err("Array index must be int".into());
                }

                if let Expression::Variable { name, typ } = &typed_expression
                    && let Expression::Literal { value, .. } = &typed_index
                    && let Ok(index) = value.parse::<i64>()
                    && let Type::Array(_, arr_size) = typ
                    && (index < 0 || index >= *arr_size as i64)
                {
                    return Err(format!(
                        "Index out of bounds: array '{}' has size {}, but accessed at index {}",
                        name, arr_size, index
                    )
                    .into());
                }

                if let Type::Array(inner_type, _) = typed_expression.get_type() {
                    if *inner_type != typed_value.get_type() {
                        return Err("Array assign type mismatch".into());
                    }
                    Ok(Expression::AssignIndex {
                        typ: *inner_type,
                        expression: Box::new(typed_expression),
                        index: Box::new(typed_index),
                        value: Box::new(typed_value),
                    })
                } else {
                    Err("Indexing non-array".into())
                }
            }
            Expression::Increment {
                expression,
                postfix,
                ..
            } => {
                let typed_expression = self.analyze_expression(expression)?;
                if !matches!(
                    typed_expression,
                    Expression::Variable { .. }
                        | Expression::Field { .. }
                        | Expression::Index { .. }
                ) {
                    return Err(
                        "Increment/Decrement can only be applied to a variable or field".into(),
                    );
                }
                let typ = typed_expression.get_type();
                if typ != Type::Int {
                    return Err(
                        format!("Operator ++/-- cannot be applied to type {:?}", typ).into(),
                    );
                }
                Ok(Expression::Increment {
                    typ: typ.clone(),
                    expression: Box::new(typed_expression),
                    postfix: *postfix,
                })
            }
            Expression::Decrement {
                expression,
                postfix,
                ..
            } => {
                let typed_expression = self.analyze_expression(expression)?;
                if !matches!(
                    typed_expression,
                    Expression::Variable { .. }
                        | Expression::Field { .. }
                        | Expression::Index { .. }
                ) {
                    return Err(
                        "Increment/Decrement can only be applied to a variable or field".into(),
                    );
                }
                let typ = typed_expression.get_type();
                if typ != Type::Int {
                    return Err(
                        format!("Operator ++/-- cannot be applied to type {:?}", typ).into(),
                    );
                }
                Ok(Expression::Decrement {
                    typ: typ.clone(),
                    expression: Box::new(typed_expression),
                    postfix: *postfix,
                })
            }
            Expression::Negate { expression, .. } => {
                let typed_expr = self.analyze_expression(expression)?;
                let typ = typed_expr.get_type();
                if typ != Type::Int {
                    return Err("Need numeric for minus".into());
                }
                Ok(Expression::Negate {
                    typ,
                    expression: Box::new(typed_expr),
                })
            }
            Expression::Not { expression, .. } => {
                let typed_expression = self.analyze_expression(expression)?;
                if typed_expression.get_type() != Type::Bool {
                    return Err("Need bool for !".into());
                }
                Ok(Expression::Not {
                    typ: Type::Bool,
                    expression: Box::new(typed_expression),
                })
            }
            Expression::New { class_name, .. } => {
                if !self.classes.contains_key(class_name) {
                    return Err("Unknown class".into());
                }
                Ok(Expression::New {
                    typ: Type::Class(class_name.clone()),
                    class_name: class_name.clone(),
                })
            }
            Expression::NewArray {
                element_type, size, ..
            } => {
                if !self.is_valid_type(element_type) {
                    return Err(format!("Unknown element type {:?}", element_type).into());
                }
                Ok(Expression::NewArray {
                    typ: Type::Array(Box::new(element_type.clone()), *size),
                    element_type: element_type.clone(),
                    size: *size,
                })
            }
            Expression::This { .. } => {
                let class_name = self.current_class_context().ok_or("'this' outside class")?;
                Ok(Expression::This {
                    typ: Type::Class(class_name),
                })
            }
        }
    }

    fn analyze_arguments(
        &self,
        parameters: &[Type],
        arguments: &[RawExpression],
    ) -> ResBox<Vec<TypedExpression>> {
        if parameters.len() != arguments.len() {
            return Err("Arg count mismatch".into());
        }
        let mut typed_arguments = Vec::new();
        for (parameter, argument) in parameters.iter().zip(arguments) {
            let typed_argument = self.analyze_expression(argument)?;
            if *parameter != typed_argument.get_type() {
                return Err("Arg type mismatch".into());
            }
            typed_arguments.push(typed_argument);
        }
        Ok(typed_arguments)
    }
}

impl SemanticTable {
    fn node_guarantees_return(&self, node: &RawAbstractSyntaxTree) -> bool {
        match &node.node {
            RawAbstractSyntaxNode::Return { .. } => true,
            RawAbstractSyntaxNode::If { .. } => self.if_guarantees_return(&node.children),
            RawAbstractSyntaxNode::Scope
            | RawAbstractSyntaxNode::While { .. }
            | RawAbstractSyntaxNode::Callable { .. } => {
                node.children.iter().any(|c| self.node_guarantees_return(c))
            }
            _ => false,
        }
    }

    fn if_guarantees_return(&self, children: &[RawAbstractSyntaxTree]) -> bool {
        let else_node = children
            .iter()
            .find(|c| matches!(c.node, RawAbstractSyntaxNode::Else));

        if else_node.is_none() {
            return false;
        }

        let if_guarantees = children.iter().any(|c| {
            !matches!(c.node, RawAbstractSyntaxNode::Else) && self.node_guarantees_return(c)
        });
        let else_guarantees = self.else_guarantees_return(&else_node.unwrap().children);

        if_guarantees && else_guarantees
    }

    fn else_guarantees_return(&self, children: &[RawAbstractSyntaxTree]) -> bool {
        let else_node = children
            .iter()
            .find(|c| matches!(c.node, RawAbstractSyntaxNode::Else));

        match else_node {
            Some(else_node) => self.else_guarantees_return(&else_node.children),
            None => children.iter().any(|c| self.node_guarantees_return(c)),
        }
    }
}

fn get_literal_type(value: &str) -> ResBox<Type> {
    if value == "true" || value == "false" {
        Ok(Type::Bool)
    } else if value.parse::<i64>().is_ok() {
        Ok(Type::Int)
    } else if value.starts_with('\'') && value.ends_with('\'') {
        Ok(Type::Char)
    } else {
        Err(format!("Unknown literal type for value: {}", value).into())
    }
}

pub fn semantic_analyze(ast: RawAbstractSyntaxTree) -> ResBox<TypedAbstractSyntaxTree> {
    let mut table = SemanticTable::new();
    table.collect_definitions(&ast)?;

    let has_main = table.functions.contains_key("Main")
        || table
            .classes
            .iter()
            .any(|(_, info)| info.methods.contains_key("Main"));

    if !has_main {
        return Err("Entry point 'Main' function not found".into());
    }

    table.analyze(&ast)
}
