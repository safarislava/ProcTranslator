use crate::translator::common::{RawExpression, ResBox, Type};
use std::iter::Peekable;
use std::str::Chars;
use std::vec::IntoIter;

fn parse_type_expr(s: &str) -> Type {
    match s {
        "void" => Type::Void,
        "int" => Type::Int,
        "char" => Type::Char,
        "bool" => Type::Bool,
        _ => Type::Class(s.to_string()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionBinaryOperator {
    Assign,
    AssignAdd,
    AssignSub,
    AssignMul,
    AssignDiv,
    AssignAnd,
    AssignOr,
    AssignXor,
    Or,
    And,
    BitwiseOr,
    BitwiseXor,
    BitwiseAnd,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Add,
    Sub,
    Multiply,
    Divide,
    Remainder,
    LeftShift,
    RightShift,
}

pub fn is_arithmetic_binary_op(operator: &ExpressionBinaryOperator) -> bool {
    matches!(
        operator,
        ExpressionBinaryOperator::Add
            | ExpressionBinaryOperator::Sub
            | ExpressionBinaryOperator::Multiply
            | ExpressionBinaryOperator::Divide
            | ExpressionBinaryOperator::Remainder
            | ExpressionBinaryOperator::LeftShift
            | ExpressionBinaryOperator::RightShift
            | ExpressionBinaryOperator::BitwiseAnd
            | ExpressionBinaryOperator::BitwiseOr
            | ExpressionBinaryOperator::BitwiseXor
    )
}

pub fn is_logical_binary_op(operator: &ExpressionBinaryOperator) -> bool {
    matches!(
        operator,
        ExpressionBinaryOperator::And | ExpressionBinaryOperator::Or
    )
}

pub fn is_relational_binary_op(operator: &ExpressionBinaryOperator) -> bool {
    matches!(
        operator,
        ExpressionBinaryOperator::Less
            | ExpressionBinaryOperator::LessEqual
            | ExpressionBinaryOperator::Greater
            | ExpressionBinaryOperator::GreaterEqual
    )
}

pub fn is_compering_binary_op(operator: &ExpressionBinaryOperator) -> bool {
    matches!(
        operator,
        ExpressionBinaryOperator::Equal
            | ExpressionBinaryOperator::NotEqual
            | ExpressionBinaryOperator::Less
            | ExpressionBinaryOperator::LessEqual
            | ExpressionBinaryOperator::Greater
            | ExpressionBinaryOperator::GreaterEqual
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expression<T> {
    Literal {
        typ: T,
        value: String,
    },
    Variable {
        typ: T,
        name: String,
    },
    BinaryOperator {
        typ: T,
        left: Box<Expression<T>>,
        operator: ExpressionBinaryOperator,
        right: Box<Expression<T>>,
    },
    FunctionCall {
        typ: T,
        name: String,
        arguments: Vec<Expression<T>>,
    },
    MethodCall {
        typ: T,
        object: Box<Expression<T>>,
        name: String,
        arguments: Vec<Expression<T>>,
    },
    Assign {
        typ: T,
        name: String,
        value: Box<Expression<T>>,
    },
    AssignField {
        typ: T,
        object: Box<Expression<T>>,
        name: String,
        value: Box<Expression<T>>,
    },
    AssignIndex {
        typ: T,
        expression: Box<Expression<T>>,
        index: Box<Expression<T>>,
        value: Box<Expression<T>>,
    },
    Increment {
        typ: T,
        expression: Box<Expression<T>>,
        postfix: bool,
    },
    Decrement {
        typ: T,
        expression: Box<Expression<T>>,
        postfix: bool,
    },
    Negate {
        typ: T,
        expression: Box<Expression<T>>,
    },
    Not {
        typ: T,
        expression: Box<Expression<T>>,
    },
    BitwiseNot {
        typ: T,
        expression: Box<Expression<T>>,
    },
    New {
        typ: T,
        class_name: String,
    },
    NewArray {
        typ: T,
        element_type: Type,
        size: u64,
    },
    Field {
        typ: T,
        object: Box<Expression<T>>,
        name: String,
    },
    Index {
        typ: T,
        expression: Box<Expression<T>>,
        index: Box<Expression<T>>,
    },
    AssignSlice {
        typ: T,
        expression: Box<Expression<T>>,
        start: Box<Expression<T>>,
        size: u64,
        value: Box<Expression<T>>,
    },
    Slice {
        typ: T,
        expression: Box<Expression<T>>,
        start: Box<Expression<T>>,
        size: u64,
    },
    This {
        typ: T,
    },
}

impl<T: Clone> Expression<T> {
    pub fn get_type(&self) -> T {
        match self {
            Expression::Literal { typ, .. } => typ.clone(),
            Expression::Variable { typ, .. } => typ.clone(),
            Expression::BinaryOperator { typ, .. } => typ.clone(),
            Expression::FunctionCall { typ, .. } => typ.clone(),
            Expression::MethodCall { typ, .. } => typ.clone(),
            Expression::Assign { typ, .. } => typ.clone(),
            Expression::AssignField { typ, .. } => typ.clone(),
            Expression::AssignIndex { typ, .. } => typ.clone(),
            Expression::Increment { typ, .. } => typ.clone(),
            Expression::Decrement { typ, .. } => typ.clone(),
            Expression::Negate { typ, .. } => typ.clone(),
            Expression::Not { typ, .. } => typ.clone(),
            Expression::New { typ, .. } => typ.clone(),
            Expression::NewArray { typ, .. } => typ.clone(),
            Expression::Field { typ, .. } => typ.clone(),
            Expression::Index { typ, .. } => typ.clone(),
            Expression::AssignSlice { typ, .. } => typ.clone(),
            Expression::Slice { typ, .. } => typ.clone(),
            Expression::This { typ } => typ.clone(),
            Expression::BitwiseNot { typ, .. } => typ.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(u64),
    String(String),
    Char(String),
    Bool(bool),
    Id(String),
    Operator(String),
    LeftBracket,
    RightBracket,
    LeftSquareBracket,
    RightSquareBracket,
    Comma,
    Dot,
    Colon,
}

struct Tokenizer<'a> {
    chars: Peekable<Chars<'a>>,
}

impl<'a> Tokenizer<'a> {
    fn tokenize(&mut self) -> ResBox<Vec<Token>> {
        let mut tokens = Vec::new();

        while let Some(&c) = self.chars.peek() {
            if c.is_whitespace() {
                self.chars.next();
            } else if c.is_alphabetic() || c == '_' {
                let mut id = String::new();
                while let Some(&ch) = self.chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' {
                        id.push(self.chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                match id.as_str() {
                    "true" => tokens.push(Token::Bool(true)),
                    "false" => tokens.push(Token::Bool(false)),
                    _ => tokens.push(Token::Id(id)),
                }
            } else if c.is_ascii_digit() {
                let mut num = String::new();
                while let Some(&ch) = self.chars.peek() {
                    if ch.is_ascii_digit() {
                        num.push(self.chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Number(num.parse()?));
            } else if c == '"' {
                self.chars.next();
                let mut s = String::new();
                while let Some(ch) = self.chars.next() {
                    if ch == '\\' {
                        s.push('\\');
                        if let Some(escaped) = self.chars.next() {
                            s.push(escaped);
                        } else {
                            return Err("Unexpected EOF in string literal escape".into());
                        }
                    } else if ch == '"' {
                        break;
                    } else {
                        s.push(ch);
                    }
                }
                tokens.push(Token::String(s));
            } else if c == '\'' {
                self.chars.next();
                let mut s = String::new();
                while let Some(ch) = self.chars.next() {
                    if ch == '\\' {
                        s.push('\\');
                        if let Some(escaped) = self.chars.next() {
                            s.push(escaped);
                        } else {
                            return Err("Unexpected EOF in char literal escape".into());
                        }
                    } else if ch == '\'' {
                        break;
                    } else {
                        s.push(ch);
                    }
                }
                tokens.push(Token::Char(s));
            } else {
                let next_char = self.chars.next().unwrap();
                match next_char {
                    '(' => tokens.push(Token::LeftBracket),
                    ')' => tokens.push(Token::RightBracket),
                    '[' => tokens.push(Token::LeftSquareBracket),
                    ']' => tokens.push(Token::RightSquareBracket),
                    ',' => tokens.push(Token::Comma),
                    '.' => tokens.push(Token::Dot),
                    ':' => tokens.push(Token::Colon),
                    _ => {
                        let mut op = next_char.to_string();
                        if let Some(&p) = self.chars.peek() {
                            let is_double = matches!(
                                (next_char, p),
                                ('+', '+')
                                    | ('-', '-')
                                    | ('&', '&')
                                    | ('|', '|')
                                    | ('+', '=')
                                    | ('-', '=')
                                    | ('*', '=')
                                    | ('/', '=')
                                    | ('=', '=')
                                    | ('!', '=')
                                    | ('<', '=')
                                    | ('>', '=')
                                    | ('<', '<')
                                    | ('>', '>')
                                    | ('&', '=')
                                    | ('|', '=')
                                    | ('^', '=')
                            );
                            if is_double {
                                op.push(self.chars.next().unwrap());
                            }
                        }
                        tokens.push(Token::Operator(op));
                    }
                }
            }
        }
        Ok(tokens)
    }
}

struct Parser {
    tokens: Peekable<IntoIter<Token>>,
}

impl Parser {
    fn link_binary_operator(operator: &str) -> ResBox<ExpressionBinaryOperator> {
        match operator {
            "=" => Ok(ExpressionBinaryOperator::Assign),
            "+=" => Ok(ExpressionBinaryOperator::AssignAdd),
            "-=" => Ok(ExpressionBinaryOperator::AssignSub),
            "*=" => Ok(ExpressionBinaryOperator::AssignMul),
            "/=" => Ok(ExpressionBinaryOperator::AssignDiv),
            "||" => Ok(ExpressionBinaryOperator::Or),
            "&&" => Ok(ExpressionBinaryOperator::And),
            "==" => Ok(ExpressionBinaryOperator::Equal),
            "!=" => Ok(ExpressionBinaryOperator::NotEqual),
            "<" => Ok(ExpressionBinaryOperator::Less),
            "<=" => Ok(ExpressionBinaryOperator::LessEqual),
            ">" => Ok(ExpressionBinaryOperator::Greater),
            ">=" => Ok(ExpressionBinaryOperator::GreaterEqual),
            "+" => Ok(ExpressionBinaryOperator::Add),
            "-" => Ok(ExpressionBinaryOperator::Sub),
            "*" => Ok(ExpressionBinaryOperator::Multiply),
            "/" => Ok(ExpressionBinaryOperator::Divide),
            "%" => Ok(ExpressionBinaryOperator::Remainder),
            "<<" => Ok(ExpressionBinaryOperator::LeftShift),
            ">>" => Ok(ExpressionBinaryOperator::RightShift),
            "&" => Ok(ExpressionBinaryOperator::BitwiseAnd),
            "|" => Ok(ExpressionBinaryOperator::BitwiseOr),
            "^" => Ok(ExpressionBinaryOperator::BitwiseXor),
            "&=" => Ok(ExpressionBinaryOperator::AssignAnd),
            "|=" => Ok(ExpressionBinaryOperator::AssignOr),
            "^=" => Ok(ExpressionBinaryOperator::AssignXor),
            _ => Err(format!("Unknown operator: {operator}").into()),
        }
    }

    fn binding_order(operator: &ExpressionBinaryOperator) -> (u8, u8) {
        match operator {
            ExpressionBinaryOperator::Assign
            | ExpressionBinaryOperator::AssignAdd
            | ExpressionBinaryOperator::AssignSub
            | ExpressionBinaryOperator::AssignMul
            | ExpressionBinaryOperator::AssignDiv
            | ExpressionBinaryOperator::AssignAnd
            | ExpressionBinaryOperator::AssignOr
            | ExpressionBinaryOperator::AssignXor => (1, 2),
            ExpressionBinaryOperator::Or => (3, 4),
            ExpressionBinaryOperator::And => (5, 6),
            ExpressionBinaryOperator::BitwiseOr => (7, 8),
            ExpressionBinaryOperator::BitwiseXor => (9, 10),
            ExpressionBinaryOperator::BitwiseAnd => (11, 12),
            ExpressionBinaryOperator::Equal | ExpressionBinaryOperator::NotEqual => (13, 14),
            ExpressionBinaryOperator::Less
            | ExpressionBinaryOperator::LessEqual
            | ExpressionBinaryOperator::Greater
            | ExpressionBinaryOperator::GreaterEqual => (15, 16),
            ExpressionBinaryOperator::LeftShift | ExpressionBinaryOperator::RightShift => (17, 18),
            ExpressionBinaryOperator::Add | ExpressionBinaryOperator::Sub => (19, 20),
            ExpressionBinaryOperator::Multiply
            | ExpressionBinaryOperator::Divide
            | ExpressionBinaryOperator::Remainder => (21, 22),
        }
    }

    fn postfix_order(token: &Token) -> Option<(u8, ())> {
        match token {
            Token::LeftBracket | Token::LeftSquareBracket | Token::Dot => Some((25, ())),
            Token::Operator(operator) if operator == "++" || operator == "--" => Some((25, ())),
            _ => None,
        }
    }

    fn parse_expression(&mut self, min_order: u8) -> ResBox<RawExpression> {
        let token = self.tokens.next().ok_or("Unexpected EOF")?;
        let mut left = self.parse_prefix(token)?;

        loop {
            let Some(operator) = self.tokens.peek().cloned() else {
                break;
            };

            if let Some((left_order, ())) = Self::postfix_order(&operator) {
                if left_order < min_order {
                    break;
                }
                self.tokens.next();
                left = self.parse_postfix(left, operator)?;
                continue;
            }

            if let Token::Operator(ref operator) = operator
                && let Ok(operator) = Self::link_binary_operator(operator)
            {
                let (left_order, right_order) = Self::binding_order(&operator);
                if left_order < min_order {
                    break;
                }
                self.tokens.next();
                let right = self.parse_expression(right_order)?;

                left = if operator == ExpressionBinaryOperator::Assign {
                    match left {
                        RawExpression::Variable { name, .. } => RawExpression::Assign {
                            typ: Default::default(),
                            name,
                            value: Box::new(right),
                        },
                        RawExpression::Field { object, name, .. } => RawExpression::AssignField {
                            typ: Default::default(),
                            object,
                            name,
                            value: Box::new(right),
                        },
                        RawExpression::Index {
                            expression, index, ..
                        } => RawExpression::AssignIndex {
                            typ: Default::default(),
                            expression,
                            index,
                            value: Box::new(right),
                        },
                        RawExpression::Slice {
                            expression,
                            start,
                            size,
                            ..
                        } => RawExpression::AssignSlice {
                            typ: Default::default(),
                            expression,
                            start,
                            size,
                            value: Box::new(right),
                        },
                        _ => return Err("Invalid assignment target".into()),
                    }
                } else {
                    RawExpression::BinaryOperator {
                        typ: Default::default(),
                        left: Box::new(left),
                        operator,
                        right: Box::new(right),
                    }
                };
                continue;
            }
            break;
        }
        Ok(left)
    }

    fn parse_prefix(&mut self, token: Token) -> ResBox<RawExpression> {
        match token {
            Token::Number(n) => Ok(RawExpression::Literal {
                typ: Default::default(),
                value: n.to_string(),
            }),
            Token::String(s) => Ok(RawExpression::Literal {
                typ: Default::default(),
                value: format!("\"{}\"", s),
            }),
            Token::Char(c) => Ok(RawExpression::Literal {
                typ: Default::default(),
                value: format!("'{}'", c),
            }),
            Token::Bool(b) => Ok(RawExpression::Literal {
                typ: Default::default(),
                value: b.to_string(),
            }),
            Token::Id(id) => {
                if id == "this" {
                    Ok(RawExpression::This {
                        typ: Default::default(),
                    })
                } else if id == "new" {
                    let type_name = match self.tokens.next() {
                        Some(Token::Id(name)) => name,
                        _ => return Err("Expected type name after 'new'".into()),
                    };

                    match self.tokens.peek() {
                        Some(Token::LeftSquareBracket) => {
                            self.tokens.next();
                            if let Token::Number(size) = self.tokens.next().unwrap() {
                                if self.tokens.next() != Some(Token::RightSquareBracket) {
                                    return Err("Expected ']' after array size".into());
                                }
                                Ok(RawExpression::NewArray {
                                    typ: Default::default(),
                                    element_type: parse_type_expr(&type_name),
                                    size,
                                })
                            } else {
                                Err("Array size must be constant".into())
                            }
                        }
                        Some(Token::LeftBracket) => {
                            self.tokens.next();
                            if self.tokens.next() != Some(Token::RightBracket) {
                                return Err(
                                    "Arguments in 'new Class()' are not supported yet".into()
                                );
                            }
                            Ok(RawExpression::New {
                                typ: Default::default(),
                                class_name: type_name,
                            })
                        }
                        _ => Err("Expected '(' or '[' after type name in 'new'".into()),
                    }
                } else {
                    Ok(RawExpression::Variable {
                        typ: Default::default(),
                        name: id,
                    })
                }
            }
            Token::LeftBracket => {
                let expr = self.parse_expression(0)?;
                if self.tokens.next() != Some(Token::RightBracket) {
                    return Err("Expected ')'".into());
                }
                Ok(expr)
            }
            Token::Operator(operator) => {
                let right_order = 23;
                if operator == "~" {
                    Ok(RawExpression::BitwiseNot {
                        typ: Default::default(),
                        expression: Box::new(self.parse_expression(right_order)?),
                    })
                } else if operator == "-" {
                    if let Some(Token::Number(n)) = self.tokens.peek() {
                        let value = format!("-{}", n);
                        self.tokens.next();
                        Ok(RawExpression::Literal {
                            typ: Default::default(),
                            value,
                        })
                    } else {
                        Ok(RawExpression::Negate {
                            typ: Default::default(),
                            expression: Box::new(self.parse_expression(right_order)?),
                        })
                    }
                } else if operator == "!" {
                    Ok(RawExpression::Not {
                        typ: Default::default(),
                        expression: Box::new(self.parse_expression(right_order)?),
                    })
                } else if operator == "++" {
                    Ok(RawExpression::Increment {
                        typ: Default::default(),
                        expression: Box::new(self.parse_expression(right_order)?),
                        postfix: false,
                    })
                } else if operator == "--" {
                    Ok(RawExpression::Decrement {
                        typ: Default::default(),
                        expression: Box::new(self.parse_expression(right_order)?),
                        postfix: false,
                    })
                } else {
                    Err(format!("Invalid prefix operator: {}", operator).into())
                }
            }
            _ => Err(format!("Unexpected token in prefix position: {:?}", token).into()),
        }
    }

    fn parse_postfix(&mut self, left: RawExpression, token: Token) -> ResBox<RawExpression> {
        match token {
            Token::Operator(op) if op == "++" => Ok(RawExpression::Increment {
                typ: Default::default(),
                expression: Box::new(left),
                postfix: true,
            }),
            Token::Operator(operator) if operator == "--" => Ok(RawExpression::Decrement {
                typ: Default::default(),
                expression: Box::new(left),
                postfix: true,
            }),
            Token::Dot => {
                let name = match self.tokens.next() {
                    Some(Token::Id(id)) => id,
                    _ => return Err("Expected identifier after '.'".into()),
                };
                if let Some(Token::LeftBracket) = self.tokens.peek() {
                    self.tokens.next();
                    let arguments = self.parse_arguments()?;
                    Ok(RawExpression::MethodCall {
                        typ: Default::default(),
                        object: Box::new(left),
                        name,
                        arguments,
                    })
                } else {
                    Ok(RawExpression::Field {
                        typ: Default::default(),
                        object: Box::new(left),
                        name,
                    })
                }
            }
            Token::LeftBracket => {
                let arguments = self.parse_arguments()?;
                if let RawExpression::Variable { name, .. } = left {
                    Ok(RawExpression::FunctionCall {
                        typ: Default::default(),
                        name,
                        arguments,
                    })
                } else {
                    Err("Invalid target for function call".into())
                }
            }
            Token::LeftSquareBracket => {
                let start_or_index = self.parse_expression(0)?;

                if let Some(Token::Colon) = self.tokens.peek() {
                    self.tokens.next();

                    if let Token::Number(size) = self.tokens.next().unwrap() {
                        if self.tokens.next() != Some(Token::RightSquareBracket) {
                            return Err("Expected ']' after slice range".into());
                        }

                        Ok(RawExpression::Slice {
                            typ: Default::default(),
                            expression: Box::new(left),
                            start: Box::new(start_or_index),
                            size,
                        })
                    } else {
                        Err("Slice must have constant size".into())
                    }
                } else {
                    if self.tokens.next() != Some(Token::RightSquareBracket) {
                        return Err("Expected ']'".into());
                    }

                    Ok(RawExpression::Index {
                        typ: Default::default(),
                        expression: Box::new(left),
                        index: Box::new(start_or_index),
                    })
                }
            }
            _ => Err("Invalid postfix token".into()),
        }
    }

    fn parse_arguments(&mut self) -> ResBox<Vec<RawExpression>> {
        let mut arguments = Vec::new();
        if let Some(Token::RightBracket) = self.tokens.peek() {
            self.tokens.next();
            return Ok(arguments);
        }
        loop {
            arguments.push(self.parse_expression(0)?);
            match self.tokens.next() {
                Some(Token::Comma) => continue,
                Some(Token::RightBracket) => break,
                _ => return Err("Expected ',' or ')' in arguments list".into()),
            }
        }
        Ok(arguments)
    }
}

pub fn parse_expression(code: &str) -> ResBox<RawExpression> {
    let mut tokenizer = Tokenizer {
        chars: code.chars().peekable(),
    };
    let tokens = tokenizer.tokenize()?;

    if tokens.is_empty() {
        return Err("Empty expression".into());
    }

    let mut parser = Parser {
        tokens: tokens.into_iter().peekable(),
    };
    let expression = parser.parse_expression(0)?;

    if parser.tokens.peek().is_some() {
        return Err(format!("Unexpected tokens remaining after parsing expression: {code}").into());
    }

    Ok(expression)
}
