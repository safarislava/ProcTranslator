use std::collections::HashMap;
use crate::common::BoxError;
use crate::ast::{AST, ASN};
use crate::expression::Expression;

struct SemanticTable {
    scopes: Vec<HashMap<String, String>>,
    functions: HashMap<String, (String, Vec<String>)>,
}

impl SemanticTable {
    pub fn new() -> Self {
        SemanticTable { scopes: vec![], functions: HashMap::new() }
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
        for child in ast.children.iter() {
            match &child.node {
                ASN::Declaration { typ, name, expression } => {
                    if let Some(var) = self.find_var(name) {
                        return Err("Variable with this name already exists".into());
                    }
                    if let Some(expression) = expression  {
                        let expr_typ = self.check_type_expression(expression)?;
                        if expr_typ != *typ {
                            return Err("Type mismatch".into());
                        }
                    }
                    if let Some(vars) = self.scopes.last_mut() {
                        vars.insert(name.clone(), typ.clone());
                    }
                    else {
                        return Err("Scope doesn't exist".into());
                    }
                },
                ASN::Expression { expression } => {

                },
                _ => continue,
            }
        }
        Ok(())
    }

    fn check_type_expression(&self, expression: &Expression) -> Result<String, BoxError> {
        match expression {
            Expression::Literal { value } => {
                Self::get_literal_type(value)
            }
            Expression::BinaryOp { left, op, right } => {
                let left_typ = self.check_type_expression(left)?;
                let right_typ = self.check_type_expression(right)?;
                if left_typ == right_typ {
                    Ok(left_typ)
                }
                else {
                    Err("Unsupported binary operator".into())
                }
            }
            Expression::FunctionCall { name, arguments } => {
                if let Some((result_type, arguments_type)) = self.functions.get(name) {
                    if arguments.len() != arguments_type.len() {
                        return Err("Function call argument mismatch".into());
                    }
                    for i in 0..arguments.len() {
                        if arguments_type[i] != self.check_type_expression(&arguments[i])? {
                            return Err("Function call argument mismatch".into());
                        }
                    }
                    Ok(result_type.clone())
                }
                else {
                    Err("Function with this name doesn't exist".into())
                }
            }
            Expression::Assign { .. } => {
                Err("Assignment doesn't have type".into())
            }
            Expression::Variable { name } |
            Expression::Increment { name, .. } |
            Expression::Decrement { name, .. } => {
                if let Some(typ) = self.find_var(name) {
                    Ok(typ.clone())
                }
                else {
                    Err("Variable with this name doesn't exist".into())
                }
            }
        }
    }

    fn get_literal_type(value: &str) -> Result<String, BoxError> {
        if Self::is_int_literal(value) {
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
}

pub fn semantic_analyze(ast: &AST) -> Result<(), BoxError> {
    let mut table = SemanticTable::new();
    table.check(ast)?;
    Ok(())
}
