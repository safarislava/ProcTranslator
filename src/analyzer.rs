use std::collections::HashMap;
use crate::common::BoxError;
use crate::ast::{AST, ASN, Initializer, Type};
use crate::expression::{Expression, BinaryOperator};

type FunctionMap = HashMap<String, (Type, Vec<Type>)>;

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
    functions: FunctionMap,
    classes: HashMap<String, ClassInfo>,
    stacktrace: Vec<ASN>,
}

impl SemanticTable {
    pub fn new() -> Self {
        SemanticTable {
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
            classes: HashMap::new(),
            stacktrace: vec![],
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
            if let ASN::Class { name } = node {
                return Some(name.clone());
            }
        }
        None
    }

    fn collect_definitions(&mut self, ast: &AST) -> Result<(), BoxError> {
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

    fn check(&mut self, ast: &AST) -> Result<(), BoxError> {
        self.stacktrace.push(ast.node.clone());

        match &ast.node {
            ASN::If { condition } | ASN::ElseIf { condition } | ASN::While { condition } => {
                if self.check_expression(condition)? != Type::Bool {
                    return Err("Condition must be bool".into());
                }
                self.scopes.push(HashMap::new());
                self.check_children(ast)?;
                self.scopes.pop();
            }
            ASN::For { initializer, condition, increment } => {
                self.scopes.push(HashMap::new());
                if let Some(init) = initializer {
                    match init {
                        Initializer::Declaration { typ, name, expression } => self.check_declaration(typ, name, expression)?,
                        Initializer::Expression { expression } => { self.check_expression(expression)?; }
                    }
                }
                if let Some(cond) = condition && self.check_expression(cond)? != Type::Bool {
                    return Err("For condition must be bool".into());
                }
                if let Some(inc) = increment { self.check_expression(inc)?; }
                self.check_children(ast)?;
                self.scopes.pop();
            }
            ASN::Callable { arguments, .. } => {
                self.scopes.push(HashMap::new());
                for arg in arguments {
                    self.scopes.last_mut().unwrap().insert(arg.name.clone(), arg.typ.clone());
                }
                self.check_children(ast)?;
                self.scopes.pop();
            }
            ASN::Class { .. } | ASN::File | ASN::Scope | ASN::Else => {
                self.scopes.push(HashMap::new());
                self.check_children(ast)?;
                self.scopes.pop();
            }
            ASN::Expression { expression } => { self.check_expression(expression)?; }
            ASN::Declaration { typ, name, expression } => {
                if !matches!(self.stacktrace.get(self.stacktrace.len() - 2), Some(ASN::Class { .. })) {
                    self.check_declaration(typ, name, expression)?;
                }
            }
            ASN::Return { value } => {
                let func = self.stacktrace.iter().rev().find_map(|n| if let ASN::Callable { result_type, .. } = n { Some(result_type) } else { None })
                    .ok_or("Return outside function")?;
                let val_type = if let Some(expr) = value { self.check_expression(expr)? } else { Type::Void };
                if val_type != *func { return Err("Return type mismatch".into()); }
            }
            ASN::Break | ASN::Continue => {
                if !self.stacktrace.iter().any(|n| matches!(n, ASN::While { .. } | ASN::For { .. })) {
                    return Err("Jump outside loop".into());
                }
            }
        }

        self.stacktrace.pop();
        Ok(())
    }

    fn check_children(&mut self, ast: &AST) -> Result<(), BoxError> {
        for child in &ast.children {
            self.check(child)?;
        }
        Ok(())
    }

    fn check_declaration(&mut self, typ: &Type, name: &str, expression: &Option<Expression>) -> Result<(), BoxError> {
        if let Type::Class(c) = typ && !self.classes.contains_key(c) {
            return Err(format!("Unknown type {}", c).into());
        }
        if let Some(expr) = expression && self.check_expression(expr)? != *typ {
            return Err("Declaration type mismatch".into());
        }
        self.scopes.last_mut().unwrap().insert(name.to_owned(), typ.clone());
        Ok(())
    }

    fn check_expression(&self, expression: &Expression) -> Result<Type, BoxError> {
        match expression {
            Expression::Literal(v) => Self::get_literal_type(v),
            Expression::Variable { name } => self.find_var(name).cloned().ok_or(format!("Undefined: {}", name).into()),
            Expression::BinaryOp { left, op, right } => {
                let lt = self.check_expression(left)?;
                let rt = self.check_expression(right)?;
                if lt != rt { return Err("Binary op type mismatch".into()); }
                Ok(if Self::is_compering_binary_op(op) { Type::Bool } else { lt })
            }
            Expression::FunctionCall { name, arguments } => {
                let (ret, params) = self.functions.get(name).ok_or(format!("Func {} not found", name))?;
                self.check_args(params, arguments)?;
                Ok(ret.clone())
            }
            Expression::MethodCall { object, name: method, arguments } => {
                let obj_type = self.check_expression(object)?;
                if let Type::Class(c_name) = obj_type {
                    let class = self.classes.get(&c_name).ok_or("Class not found")?;
                    let (ret, params) = class.methods.get(method).ok_or(format!("Method {} not found in {}", method, c_name))?;
                    self.check_args(params, arguments)?;
                    Ok(ret.clone())
                } else { Err("Method call on non-object".into()) }
            }
            Expression::Field { object, name: member } => {
                let obj_type = self.check_expression(object)?;
                if let Type::Class(c_name) = obj_type {
                    let class = self.classes.get(&c_name).ok_or("Class not found")?;
                    class.fields.get(member).cloned().ok_or(format!("Field {} not found", member).into())
                } else { Err("Field access on non-object".into()) }
            }
            Expression::Assign { name, value } => {
                let var_t = self.find_var(name).ok_or(format!("Undefined {}", name))?.clone();
                if var_t != self.check_expression(value)? { return Err("Assign type mismatch".into()); }
                Ok(var_t)
            }
            Expression::AssignField { object, name: member, value } => {
                let obj_t = self.check_expression(object)?;
                if let Type::Class(c) = obj_t {
                    let field_t = self.classes.get(&c).unwrap().fields.get(member).ok_or("Field not found")?.clone();
                    if field_t != self.check_expression(value)? { return Err("Field assign mismatch".into()); }
                    Ok(field_t)
                } else { Err("Not an object".into()) }
            }
            Expression::Increment { expression , ..} |
            Expression::Decrement { expression, .. } => {
                match **expression {
                    Expression::Variable { .. } | Expression::Field { .. } => {}
                    _ => return Err("Increment/Decrement can only be applied to a variable or field".into())
                }

                let typ = self.check_expression(expression)?;
                if typ != Type::Int && typ != Type::Float {
                    return Err(format!("Operator ++/-- cannot be applied to type {:?}", typ).into());
                }
                Ok(typ.clone())
            }
            Expression::Negate { expression } => {
                let t = self.check_expression(expression)?;
                if t != Type::Int && t != Type::Float { return Err("Need numeric for minus".into()); }
                Ok(t)
            }
            Expression::Not { expression } => {
                if self.check_expression(expression)? != Type::Bool { return Err("Need bool for !".into()); }
                Ok(Type::Bool)
            }
            Expression::New { class_name, .. } => {
                if !self.classes.contains_key(class_name) { return Err("Unknown class".into()); }
                Ok(Type::Class(class_name.clone()))
            }
            Expression::This => {
                let c = self.current_class_context().ok_or("'this' outside class")?;
                Ok(Type::Class(c))
            }
        }
    }

    fn check_args(&self, params: &[Type], args: &[Expression]) -> Result<(), BoxError> {
        if params.len() != args.len() {
            return Err("Arg count mismatch".into());
        }
        for (p, a) in params.iter().zip(args) {
            if *p != self.check_expression(a)? {
                return Err("Arg type mismatch".into());
            }
        }
        Ok(())
    }

    fn get_literal_type(value: &str) -> Result<Type, BoxError> {
        if value.parse::<bool>().is_ok() {
            Ok(Type::Bool)
        } else if value.parse::<i64>().is_ok() {
            Ok(Type::Int)
        } else if value.parse::<f64>().is_ok() {
            Ok(Type::Float)
        }
        else if value.starts_with('"') && value.ends_with('"') {
            Ok(Type::Str)
        }
        else {
            Err("Unknown literal type".into())
        }
    }

    fn is_compering_binary_op(op: &BinaryOperator) -> bool {
        matches!(op, BinaryOperator::Equal | BinaryOperator::NotEqual | BinaryOperator::Less | BinaryOperator::LessEqual | BinaryOperator::Greater | BinaryOperator::GreaterEqual)
    }
}

pub fn semantic_analyze(ast: &AST) -> Result<(), BoxError> {
    let mut table = SemanticTable::new();
    table.collect_definitions(ast)?;
    table.check(ast)?;
    Ok(())
}