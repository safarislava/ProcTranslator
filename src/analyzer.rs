use std::collections::HashMap;
use crate::common::BoxError;
use crate::ast::{AST, ASN, Initializer};
use crate::expression::Expression;

struct SemanticTable {
    scopes: Vec<HashMap<String, String>>,
    functions: HashMap<String, (String, Vec<String>)>,
    context_stack: Vec<ASN>,
}

impl SemanticTable {
    pub fn new() -> Self {
        SemanticTable { scopes: vec![], functions: HashMap::new(), context_stack: vec![],}
    }

    fn find_var(&self, name: &str) -> Option<&String> {
        for scope in self.scopes.iter().rev() {
            if let Some(typ) = scope.get(name) {
                return Some(typ);
            }
        }
        None
    }
    fn check(&mut self, ast: &AST) -> Result<(), BoxError> {
        match &ast.node {
            ASN::If { condition } |
            ASN::ElseIf { condition } |
            ASN::While { condition } => {
                let expression_type = self.check_expression(condition)?;
                if expression_type != "bool" {
                    return Err("Condition must be bool".into());
                }
                self.context_stack.push(ast.node.clone());
                self.scopes.push(HashMap::new());
            }
            ASN::Else => {
                self.context_stack.push(ast.node.clone());
                self.scopes.push(HashMap::new());
            }
            ASN::For { initializer, condition, increment } => {
                self.context_stack.push(ast.node.clone());
                self.scopes.push(HashMap::new());

                if let Some(init) = initializer {
                    match init {
                        Initializer::Declaration { typ, name, value: option_expr } => {
                            self.check_declaration(typ, name, option_expr)?;
                        }
                        Initializer::Expression { value } => {
                            self.check_expression(value)?;
                        }
                    }
                }
                if let Some(condition) = condition {
                    let condition_type = self.check_expression(condition)?;
                    if condition_type != "bool" {
                        return Err("Condition must be bool".into());
                    }
                }
                if let Some(increment) = increment {
                    self.check_expression(increment)?;
                }
            }
            ASN::Function { result_type, name, arguments } => {
                match self.context_stack.last() {
                    Some(ASN::File) | Some(ASN::Class { .. }) => {}
                    _ => return Err("Function can only be declared inside file or class".into())
                }
                if self.functions.contains_key(name) {
                    return Err("Function already exists".into());
                }
                let arg_types = arguments.iter().map(|v| v.typ.clone()).collect();
                self.functions.insert(name.clone(), (result_type.clone(), arg_types));

                self.context_stack.push(ast.node.clone());
                self.scopes.push(HashMap::new());

                for arg in arguments {
                    self.scopes.last_mut().unwrap().insert(arg.name.clone(), arg.typ.clone());
                }
            }
            ASN::Class { .. } => {
                if !matches!(self.context_stack.last(), Some(ASN::File)) {
                    return Err("Class can only be declared inside file".into());
                }
                self.context_stack.push(ast.node.clone());
                self.scopes.push(HashMap::new());
            }
            ASN::Expression { expression } => {
                self.check_expression(expression)?;
            }
            ASN::Declaration { typ, name, expression: option_expr } => {
                self.check_declaration(typ, name, option_expr)?;
            }
            ASN::Return { value } => {
                let function = self.context_stack.iter().rev().find(|ctx| matches!(ctx, ASN::Function { .. }));
                let result_type = match function {
                    Some(ASN::Function { result_type, .. }) => result_type,
                    _ => return Err("Return outside of function".into()),
                };

                match value {
                    Some(expression) => {
                        let expression_type = self.check_expression(expression)?;
                        if expression_type != *result_type {
                            return Err("Return type mismatch".into());
                        }
                    }
                    None => {
                        if result_type != "void" {
                            return Err("Return value expected".into());
                        }
                    }
                }
            }
            ASN::Break => {
                if !self.context_stack.iter().rev().any(|n| matches!(n, ASN::While { .. } | ASN::For { .. })) {
                    return Err("Break can only be declared inside loop".into());
                }
            }
            ASN::Continue => {
                if !self.context_stack.iter().rev().any(|n| matches!(n, ASN::While { .. } | ASN::For { .. })) {
                    return Err("Continue can only be declared inside loop".into());
                }
            }
            ASN::Scope => {
                if matches!(self.context_stack.last(), Some(ASN::File)) ||
                    matches!(self.context_stack.last(), Some(ASN::Class { .. })) {
                    return Err("Class cannot be declared into file and class".into());
                }
                self.context_stack.push(ast.node.clone());
                self.scopes.push(HashMap::new());
            }
            ASN::File => {
                self.context_stack.push(ast.node.clone());
                self.scopes.push(HashMap::new());
            }
        }

        for (i, child) in ast.children.iter().enumerate() {
            match &child.node {
                ASN::ElseIf { .. } | ASN::Else => {
                    if i == 0 {
                        return Err("else/else if must follow if or else if".into());
                    }
                    let prev = &ast.children[i - 1].node;
                    match prev {
                        ASN::If { .. } | ASN::ElseIf { .. } => {}
                        _ => return Err("else/else if must follow if or else if".into())
                    }
                }
                _ => {}
            }
            self.check(child)?;
        }

        match ast.node {
            ASN::File
            | ASN::Class { .. }
            | ASN::Function { .. }
            | ASN::Scope
            | ASN::If { .. }
            | ASN::ElseIf { .. }
            | ASN::Else
            | ASN::While { .. }
            | ASN::For { .. } => {
                self.context_stack.pop();
                self.scopes.pop();
            }
            _ => {}
        }

        Ok(())
    }

    fn check_declaration(&mut self, typ: &String, name: &String, option_expr: &Option<Expression>) -> Result<(), BoxError> {
        if self.scopes.last().unwrap().contains_key(name) {
            return Err("Variable already declared in this scope".into());
        }
        if let Some(expression) = option_expr {
            let expression_type = self.check_expression(expression)?;
            if expression_type != *typ {
                return Err("Type mismatch in declaration".into());
            }
        }
        self.scopes.last_mut().unwrap().insert(name.clone(), typ.clone());
        Ok(())
    }

    fn check_expression(&self, expression: &Expression) -> Result<String, BoxError> {
        match expression {
            Expression::Literal { value } => {
                Self::get_literal_type(value)
            }
            Expression::BinaryOp { left, op, right } => {
                let left_typ = self.check_expression(left)?;
                let right_typ = self.check_expression(right)?;
                if left_typ != right_typ {
                    return Err("Unsupported binary operator".into());
                }
                if Self::is_compering_binary_op(op) {
                    Ok("bool".to_string())
                }
                else {
                    Ok(left_typ)
                }
            }
            Expression::FunctionCall { name, arguments } => {
                if let Some((result_type, arguments_type)) = self.functions.get(name) {
                    if arguments.len() != arguments_type.len() {
                        return Err("Function call argument mismatch".into());
                    }
                    for i in 0..arguments.len() {
                        if arguments_type[i] != self.check_expression(&arguments[i])? {
                            return Err("Function call argument mismatch".into());
                        }
                    }
                    Ok(result_type.clone())
                }
                else {
                    Err("Function with this name doesn't exist".into())
                }
            }
            Expression::Assign { value, .. } => {
                self.check_expression(value)?;
                Ok("void".into())
            }
            Expression::Variable { name } |
            Expression::Increment { name, .. } |
            Expression::Decrement { name, .. } => {
                if let Some(typ) = self.find_var(name) {
                    Ok(typ.clone())
                }
                else {
                    Err(format!("Variable with name {name} doesn't exist").into())
                }
            }
        }
    }

    fn get_literal_type(value: &str) -> Result<String, BoxError> {
        if Self::is_bool_literal(value) {
            Ok("bool".to_string())
        }
        else if Self::is_int_literal(value) {
            Ok("int".to_string())
        }
        else if Self::is_float_literal(value) {
            Ok("float".to_string())
        }
        else if Self::is_string_literal(value) {
            Ok("string".to_string())
        }
        else {
            Err("Unsupported type of literal".into())
        }
    }

    fn is_string_literal(value: &str) -> bool {
        value.starts_with('"') && value.ends_with('"')
    }

    fn is_int_literal(value: &str) -> bool {
        value.parse::<i64>().is_ok()
    }

    fn is_float_literal(value: &str) -> bool {
        value.parse::<f64>().is_ok()
    }

    fn is_bool_literal(value: &str) -> bool {
        value.parse::<bool>().is_ok()
    }

    fn is_compering_binary_op(op: &str) -> bool {
        op == "==" || op == "!=" || op == "<" || op == "<=" || op == ">" || op == ">="
    }
}

pub fn semantic_analyze(ast: &AST) -> Result<(), BoxError> {
    let mut table = SemanticTable::new();
    table.check(ast)?;
    Ok(())
}
