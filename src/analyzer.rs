use std::collections::HashMap;
use crate::common::{BoxError, RawAST, RawExpression, Type, TypedAST, TypedExpression, ASN};
use crate::expression::{BinaryOperator, Expression};

type FunctionInfo = (Type, Vec<Type>);

#[derive(Debug, Clone)]
struct ClassInfo {
    fields: HashMap<String, Type>,
    methods: HashMap<String, (Type, Vec<Type>)>,
}

impl ClassInfo {
    fn new() -> ClassInfo {
        ClassInfo { fields: HashMap::new(), methods: HashMap::new() }
    }
}

struct SemanticTable {
    scopes: Vec<HashMap<String, Type>>,
    stacktrace: Vec<ASN<RawExpression>>,
    functions: HashMap<String, FunctionInfo>,
    classes: HashMap<String, ClassInfo>,
}

impl SemanticTable {
    pub fn new() -> Self {
        SemanticTable {
            scopes: vec![HashMap::new()],
            stacktrace: vec![],
            functions: HashMap::new(),
            classes: HashMap::new(),
        }
    }

    fn find_var(&self, name: &str) -> Option<&Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(typ) = scope.get(name) { return Some(typ); }
        }
        None
    }

    fn current_class_context(&self) -> Option<String> {
        for node in self.stacktrace.iter().rev() {
            if let ASN::Class { name } = node {
                return Some(name.clone());
            }
        }
        None
    }

    fn collect_definitions(&mut self, ast: &RawAST) -> Result<(), BoxError> {
        match &ast.node {
            ASN::Class { name } => {
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
            ASN::Callable { result_type, name, arguments } => {
                let arg_types: Vec<Type> = arguments.iter().map(|v| v.typ.clone()).collect();
                if let Some(class_name) = self.current_class_context() {
                    let class_info = self.classes.get_mut(&class_name).unwrap();
                    class_info.methods.insert(name.clone(), (result_type.clone(), arg_types));
                } else {
                    self.functions.insert(name.clone(), (result_type.clone(), arg_types));
                }
            }
            ASN::Declaration { typ, name, .. } => {
                if let Some(class_name) = self.current_class_context() &&
                    matches!(self.stacktrace.last(), Some(ASN::Class { .. })) {
                    let class_info = self.classes.get_mut(&class_name).unwrap();
                    class_info.fields.insert(name.clone(), typ.clone());
                }
            }
            ASN::File | ASN::Scope => {
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

    pub fn analyze(&mut self, ast: &RawAST) -> Result<TypedAST, BoxError> {
        self.stacktrace.push(ast.node.clone());

        let (typed_node, typed_children) = match &ast.node {
            ASN::If { condition } => {
                let typed_condition = self.analyze_expression(condition)?;
                if typed_condition.get_type() != Type::Bool { return Err("Condition must be bool".into()); }
                self.scopes.push(HashMap::new());
                let children = self.analyze_children(&ast.children)?;
                self.scopes.pop();
                (ASN::If { condition: typed_condition }, children)
            }
            ASN::While { condition } => {
                let typed_condition = self.analyze_expression(condition)?;
                if typed_condition.get_type() != Type::Bool { return Err("Condition must be bool".into()); }
                self.scopes.push(HashMap::new());
                let children = self.analyze_children(&ast.children)?;
                self.scopes.pop();
                (ASN::While { condition: typed_condition }, children)
            }
            ASN::Callable { result_type, name, arguments } => {
                self.scopes.push(HashMap::new());
                for arg in arguments {
                    self.scopes.last_mut().unwrap().insert(arg.name.clone(), arg.typ.clone());
                }
                let children = self.analyze_children(&ast.children)?;
                self.scopes.pop();

                if !self.statements_guarantee_return(&children) {
                    return Err(format!("Not all code paths return a value in function '{}'", name).into());
                }
                (ASN::Callable { result_type: result_type.clone(), name: name.clone(), arguments: arguments.clone() }, children)
            }
            ASN::Class { name } => {
                self.scopes.push(HashMap::new());
                for child in &ast.children {
                    if let ASN::Declaration { typ, name, expression } = &child.node {
                        let typed_expr = match expression {
                            Some(e) => Some(self.analyze_expression(e)?),
                            None => None,
                        };
                        self.analyze_declaration(typ, name, &typed_expr)?;
                    }
                }

                let mut typed_children = vec![];
                for child in &ast.children {
                    if !matches!(child.node, ASN::Declaration { .. }) {
                        typed_children.push(self.analyze(child)?);
                    } else if let ASN::Declaration { typ, name, expression } = &child.node {
                        let typed_expr = match expression {
                            Some(e) => Some(self.analyze_expression(e)?),
                            None => None,
                        };
                        typed_children.push(TypedAST::new(ASN::Declaration { typ: typ.clone(), name: name.clone(), expression: typed_expr }));
                    }
                }
                self.scopes.pop();
                (ASN::Class { name: name.clone() }, typed_children)
            }
            ASN::File | ASN::Scope | ASN::Else => {
                self.scopes.push(HashMap::new());
                let children = self.analyze_children(&ast.children)?;
                self.scopes.pop();
                (ast.node.clone().map_expr(|_| unreachable!()), children)
            }
            ASN::Expression { expression } => {
                let typed_expr = self.analyze_expression(expression)?;
                (ASN::Expression { expression: typed_expr }, vec![])
            }
            ASN::Declaration { typ, name, expression } => {
                let typed_expr = match expression {
                    Some(e) => Some(self.analyze_expression(e)?),
                    None => None,
                };
                if !matches!(self.stacktrace.get(self.stacktrace.len() - 2), Some(ASN::Class { .. })) {
                    self.analyze_declaration(typ, name, &typed_expr)?;
                }
                (ASN::Declaration { typ: typ.clone(), name: name.clone(), expression: typed_expr }, vec![])
            }
            ASN::Return { value } => {
                let func_type = self.stacktrace.iter().rev().find_map(|n|
                    if let ASN::Callable { result_type, .. } = n { Some(result_type) } else { None }).ok_or("Return outside function")?;
                let typed_value = match value {
                    Some(expr) => Some(self.analyze_expression(expr)?),
                    None => None
                };
                let val_type = typed_value.as_ref().map_or(Type::Void, |v| v.get_type());
                if val_type != *func_type { return Err("Return type mismatch".into()); }
                (ASN::Return { value: typed_value }, vec![])
            }
            ASN::Break | ASN::Continue => {
                if !self.stacktrace.iter().any(|n| matches!(n, ASN::While { .. } | ASN::For { .. })) {
                    return Err("Jump outside loop".into());
                }
                (ast.node.clone().map_expr(|_| unreachable!()), vec![])
            }
            _ => unreachable!()
        };

        self.stacktrace.pop();
        Ok(TypedAST::with_children(typed_node, typed_children))
    }

    fn analyze_children(&mut self, children: &[RawAST]) -> Result<Vec<TypedAST>, BoxError> {
        let mut typed_children = vec![];
        for child in children {
            typed_children.push(self.analyze(child)?);
        }
        Ok(typed_children)
    }

    fn analyze_declaration(&mut self, typ: &Type, name: &str, expression: &Option<TypedExpression>) -> Result<(), BoxError> {
        if let Type::Class(c) = typ && !self.classes.contains_key(c) {
            return Err(format!("Unknown type {}", c).into());
        }
        if let Some(expr) = expression && expr.get_type() != *typ {
            return Err("Declaration type mismatch".into());
        }
        self.scopes.last_mut().unwrap().insert(name.to_owned(), typ.clone());
        Ok(())
    }

    fn analyze_expression(&self, expression: &RawExpression) -> Result<TypedExpression, BoxError> {
        match expression {
            Expression::Literal { value, .. } => {
                let typ = get_literal_type(value)?;
                Ok(Expression::Literal { typ, value: value.clone() })
            }
            Expression::Variable { name, .. } => {
                let typ = self.find_var(name).cloned().ok_or(format!("Undefined: {}", name))?;
                Ok(Expression::Variable { typ, name: name.clone() })
            }
            Expression::BinaryOp { left, op, right, .. } => {
                let typed_left = self.analyze_expression(left)?;
                let typed_right = self.analyze_expression(right)?;
                let left_type = typed_left.get_type();
                let right_type = typed_right.get_type();
                if left_type != right_type { return Err("Binary op type mismatch".into()); }
                let result_type = if Self::is_compering_binary_op(op) { Type::Bool } else { left_type };
                Ok(Expression::BinaryOp { typ: result_type, left: Box::new(typed_left), op: op.clone(), right: Box::new(typed_right) })
            }
            Expression::FunctionCall { name, arguments, .. } => {
                let (ret, params) = self.functions.get(name).ok_or(format!("Func {} not found", name))?;
                let typed_args = self.analyze_args(params, arguments)?;
                Ok(Expression::FunctionCall { typ: ret.clone(), name: name.clone(), arguments: typed_args })
            }
            Expression::MethodCall { object, name: method, arguments, .. } => {
                let typed_object = self.analyze_expression(object)?;
                if let Type::Class(c_name) = typed_object.get_type() {
                    let class = self.classes.get(&c_name).ok_or("Class not found")?;
                    let (ret, params) = class.methods.get(method).ok_or(format!("Method {} not found in {}", method, c_name))?;
                    let typed_args = self.analyze_args(params, arguments)?;
                    Ok(Expression::MethodCall { typ: ret.clone(), object: Box::new(typed_object), name: method.clone(), arguments: typed_args })
                } else { Err("Method call on non-object".into()) }
            }
            Expression::Field { object, name: member, .. } => {
                let typed_object = self.analyze_expression(object)?;
                if let Type::Class(c_name) = typed_object.get_type() {
                    let class = self.classes.get(&c_name).ok_or("Class not found")?;
                    let field_type = class.fields.get(member).cloned().ok_or(format!("Field {} not found", member))?;
                    Ok(Expression::Field { typ: field_type, object: Box::new(typed_object), name: member.clone() })
                } else { Err("Field access on non-object".into()) }
            }
            Expression::Assign { name, value, .. } => {
                let var_type = self.find_var(name).ok_or(format!("Undefined {}", name))?.clone();
                let typed_value = self.analyze_expression(value)?;
                if var_type != typed_value.get_type() { return Err("Assign type mismatch".into()); }
                Ok(Expression::Assign { typ: var_type, name: name.clone(), value: Box::new(typed_value) })
            }
            Expression::AssignField { object, name: member, value, .. } => {
                let typed_object = self.analyze_expression(object)?;
                let typed_value = self.analyze_expression(value)?;
                if let Type::Class(c) = typed_object.get_type() {
                    let field_type = self.classes.get(&c).unwrap().fields.get(member).ok_or("Field not found")?.clone();
                    if field_type != typed_value.get_type() { return Err("Field assign mismatch".into()); }
                    Ok(Expression::AssignField { typ: field_type, object: Box::new(typed_object), name: member.clone(), value: Box::new(typed_value) })
                } else { Err("Not an object".into()) }
            }
            Expression::Increment { expression, postfix, ..} |
            Expression::Decrement { expression, postfix, .. } => {
                let typed_expr = self.analyze_expression(expression)?;
                if !matches!(typed_expr, Expression::Variable { .. } | Expression::Field { .. }) {
                    return Err("Increment/Decrement can only be applied to a variable or field".into());
                }
                let typ = typed_expr.get_type();
                if typ != Type::Int && typ != Type::Float { return Err(format!("Operator ++/-- cannot be applied to type {:?}", typ).into()); }

                if matches!(expression.as_ref(), Expression::Increment { .. }) {
                    Ok(Expression::Increment { typ: typ.clone(), expression: Box::new(typed_expr), postfix: *postfix })
                } else {
                    Ok(Expression::Decrement { typ: typ.clone(), expression: Box::new(typed_expr), postfix: *postfix })
                }
            }
            Expression::Negate { expression, .. } => {
                let typed_expr = self.analyze_expression(expression)?;
                let t = typed_expr.get_type();
                if t != Type::Int && t != Type::Float { return Err("Need numeric for minus".into()); }
                Ok(Expression::Negate { typ: t, expression: Box::new(typed_expr) })
            }
            Expression::Not { expression, .. } => {
                let typed_expr = self.analyze_expression(expression)?;
                if typed_expr.get_type() != Type::Bool { return Err("Need bool for !".into()); }
                Ok(Expression::Not { typ: Type::Bool, expression: Box::new(typed_expr) })
            }
            Expression::New { class_name, .. } => {
                if !self.classes.contains_key(class_name) { return Err("Unknown class".into()); }
                Ok(Expression::New { typ: Type::Class(class_name.clone()), class_name: class_name.clone() })
            }
            Expression::This { .. } => {
                let class_name = self.current_class_context().ok_or("'this' outside class")?;
                Ok(Expression::This { typ: Type::Class(class_name) })
            }
        }
    }

    fn analyze_args(&self, params: &[Type], args: &[RawExpression]) -> Result<Vec<TypedExpression>, BoxError> {
        if params.len() != args.len() { return Err("Arg count mismatch".into()); }
        let mut typed_args = Vec::new();
        for (p, a) in params.iter().zip(args) {
            let typed_a = self.analyze_expression(a)?;
            if *p != typed_a.get_type() { return Err("Arg type mismatch".into()); }
            typed_args.push(typed_a);
        }
        Ok(typed_args)
    }

    fn is_compering_binary_op(op: &BinaryOperator) -> bool {
        matches!(op, BinaryOperator::Equal | BinaryOperator::NotEqual | BinaryOperator::Less | BinaryOperator::LessEqual | BinaryOperator::Greater | BinaryOperator::GreaterEqual)
    }
}

impl SemanticTable {
    fn statements_guarantee_return(&self, children: &[TypedAST]) -> bool {
        if let Some(last_statement) = children.last() {
            match &last_statement.node {
                ASN::Return { .. } => true,
                ASN::Scope => self.statements_guarantee_return(&last_statement.children),
                ASN::If { .. } => {
                    let else_node = last_statement.children.iter().find(|c| matches!(c.node, ASN::Else));
                    if let Some(else_node) = else_node {
                        let then_children: Vec<_> = last_statement.children
                            .iter().filter(|c| !matches!(c.node, ASN::Else)).cloned().collect();
                        self.statements_guarantee_return(&then_children) && self.statements_guarantee_return(&else_node.children)
                    } else { false }
                }
                _ => false,
            }
        } else { false }
    }
}


impl<E> ASN<E> {
    fn map_expr<F, T>(self, f: F) -> ASN<T>
    where F: Fn(E) -> T + Copy {
        match self {
            ASN::If { condition } => ASN::If { condition: f(condition) },
            ASN::ElseIf { condition } => ASN::ElseIf { condition: f(condition) },
            ASN::Else => ASN::Else,
            ASN::While { condition } => ASN::While { condition: f(condition) },
            ASN::Expression { expression } => ASN::Expression { expression: f(expression) },
            ASN::Declaration { typ, name, expression } => ASN::Declaration {
                typ,
                name,
                expression: expression.map(f)
            },
            ASN::Return { value } => ASN::Return { value: value.map(f) },
            ASN::Break => ASN::Break,
            ASN::Continue => ASN::Continue,
            ASN::Scope => ASN::Scope,
            ASN::File => ASN::File,
            ASN::For { initializer, condition, increment } => ASN::For {
                initializer: initializer.map(|asn| Box::new((*asn).map_expr(f))),
                condition: condition.map(f),
                increment: increment.map(f)
            },
            ASN::Callable { result_type, name, arguments } => ASN::Callable {
                result_type,
                name,
                arguments
            },
            ASN::Class { name } => ASN::Class { name },
        }
    }
}

pub fn get_literal_type(value: &str) -> Result<Type, BoxError> {
    if value.parse::<bool>().is_ok() { Ok(Type::Bool) }
    else if value.parse::<i64>().is_ok() { Ok(Type::Int) }
    else if value.parse::<f64>().is_ok() { Ok(Type::Float) }
    else if value.starts_with('"') && value.ends_with('"') { Ok(Type::Str) }
    else { Err(format!("Unknown literal type for value: {}", value).into()) }
}

pub fn semantic_analyze(ast: RawAST) -> Result<TypedAST, BoxError> {
    let mut table = SemanticTable::new();
    table.collect_definitions(&ast)?;
    table.analyze(&ast)
}