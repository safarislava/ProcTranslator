use crate::translator::common::{RawExpression, ResBox};
use std::iter::Peekable;
use std::str::Chars;
use std::vec::IntoIter;

#[derive(Debug, Clone, PartialEq)]
pub enum ExpressionBinaryOperator {
    Assign,
    AssignAdd,
    AssignSub,
    AssignMul,
    AssignDiv,
    Or,
    And,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
}

#[derive(Debug, Clone)]
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
    New {
        typ: T,
        class_name: String,
    },
    Field {
        typ: T,
        object: Box<Expression<T>>,
        name: String,
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
            Expression::Increment { typ, .. } => typ.clone(),
            Expression::Decrement { typ, .. } => typ.clone(),
            Expression::Negate { typ, .. } => typ.clone(),
            Expression::Not { typ, .. } => typ.clone(),
            Expression::New { typ, .. } => typ.clone(),
            Expression::Field { typ, .. } => typ.clone(),
            Expression::This { typ } => typ.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(String),
    String(String),
    Bool(String),
    Id(String),
    Operator(String),
    LeftBracket,
    RightBracket,
    Comma,
    Dot,
}

struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.chars.peek() {
            if c.is_whitespace() {
                self.chars.next();
            } else {
                break;
            }
        }
    }

    fn read_while<F>(&mut self, cond: F) -> String
    where
        F: Fn(char) -> bool,
    {
        let mut s = String::new();
        while let Some(&c) = self.chars.peek() {
            if cond(c) {
                s.push(c);
                self.chars.next();
            } else {
                break;
            }
        }
        s
    }

    fn next_token(&mut self) -> ResBox<Option<Token>> {
        self.skip_whitespace();
        let &c = match self.chars.peek() {
            Some(c) => c,
            None => return Ok(None),
        };

        let token = match c {
            c if c.is_ascii_digit() => {
                Token::Number(self.read_while(|c| c.is_ascii_digit() || c == '.'))
            }
            '"' => {
                self.chars.next();
                let s = self.read_while(|c| c != '"');
                self.chars.next();
                Token::String(s)
            }
            c if c.is_alphabetic() || c == '_' => {
                let id = self.read_while(|c| c.is_alphanumeric() || c == '_');
                match id.as_str() {
                    "true" | "false" => Token::Bool(id),
                    _ => Token::Id(id),
                }
            }
            '(' => {
                self.chars.next();
                Token::LeftBracket
            }
            ')' => {
                self.chars.next();
                Token::RightBracket
            }
            ',' => {
                self.chars.next();
                Token::Comma
            }
            '.' => {
                self.chars.next();
                Token::Dot
            }
            _ => {
                let op = self.read_while(|c| "+-*/%=<>!&|".contains(c));
                if op.is_empty() {
                    self.chars.next();
                    return self.next_token();
                }
                Token::Operator(op)
            }
        };

        Ok(Some(token))
    }

    fn tokenize(&mut self) -> ResBox<Vec<Token>> {
        let mut tokens = vec![];
        while let Some(token) = self.next_token()? {
            tokens.push(token);
        }
        Ok(tokens)
    }
}

struct Parser {
    tokens: Peekable<IntoIter<Token>>,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens: tokens.into_iter().peekable(),
        }
    }

    fn is_changeable(expr: &RawExpression) -> bool {
        matches!(expr, Expression::Variable { .. } | Expression::Field { .. })
    }

    fn parse_expression(&mut self, min_order: u8) -> ResBox<RawExpression> {
        let mut left = self.parse_primary()?;

        loop {
            let next = self.tokens.peek().cloned();

            if let Some(Token::Dot) = next {
                self.tokens.next();
                let member = match self.tokens.next() {
                    Some(Token::Id(id)) => id,
                    _ => return Err("Expected identifier after '.'".into()),
                };

                if let Some(Token::LeftBracket) = self.tokens.peek() {
                    self.tokens.next();
                    let args = self.parse_arguments()?;
                    left = Expression::MethodCall {
                        typ: (),
                        object: Box::new(left),
                        name: member,
                        arguments: args,
                    };
                } else {
                    left = Expression::Field {
                        typ: (),
                        object: Box::new(left),
                        name: member,
                    };
                }
                continue;
            }

            if let Some(Token::Operator(op)) = next.as_ref()
                && (op == "++" || op == "--")
            {
                let op_str = op.clone();
                self.tokens.next();

                if Self::is_changeable(&left) {
                    left = if op_str == "++" {
                        Expression::Increment {
                            typ: (),
                            expression: Box::new(left),
                            postfix: true,
                        }
                    } else {
                        Expression::Decrement {
                            typ: (),
                            expression: Box::new(left),
                            postfix: true,
                        }
                    };
                } else {
                    return Err(
                        format!("Operator '{}' can only be applied to a variable", op_str).into(),
                    );
                }
                continue;
            }

            if let Some(Token::Operator(op_str)) = next {
                let operator = Self::link_binary_operator(&op_str)?;
                let (left_order, right_order) = Self::binding_order(&operator);
                if left_order < min_order {
                    break;
                }
                self.tokens.next();

                let right = self.parse_expression(right_order)?;

                left = match operator {
                    ExpressionBinaryOperator::Assign => {
                        if !Self::is_changeable(&left) {
                            return Err("Invalid assignment target".into());
                        }
                        match left {
                            Expression::Variable { name, .. } => Expression::Assign {
                                typ: (),
                                name,
                                value: Box::new(right),
                            },
                            Expression::Field {
                                object,
                                name: member,
                                ..
                            } => Expression::AssignField {
                                typ: (),
                                object,
                                name: member,
                                value: Box::new(right),
                            },
                            _ => unreachable!(),
                        }
                    }
                    _ => Expression::BinaryOperator {
                        typ: (),
                        left: Box::new(left),
                        operator,
                        right: Box::new(right),
                    },
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_primary(&mut self) -> ResBox<RawExpression> {
        let token = self.tokens.next().ok_or("Unexpected end of expression")?;
        match token {
            Token::Number(value) => Ok(Expression::Literal { typ: (), value }),
            Token::String(value) => Ok(Expression::Literal {
                typ: (),
                value: format!("\"{}\"", value),
            }),
            Token::Bool(value) => Ok(Expression::Literal { typ: (), value }),
            Token::Id(id) => match id.as_str() {
                "this" => Ok(Expression::This { typ: () }),
                "new" => {
                    let class_name = match self.tokens.next() {
                        Some(Token::Id(name)) => name,
                        _ => return Err("Expected class name after 'new'".into()),
                    };
                    Ok(Expression::New {
                        typ: (),
                        class_name,
                    })
                }
                name => {
                    if let Some(Token::LeftBracket) = self.tokens.peek() {
                        self.tokens.next();
                        let args = self.parse_arguments()?;
                        Ok(Expression::FunctionCall {
                            typ: (),
                            name: name.to_string(),
                            arguments: args,
                        })
                    } else {
                        Ok(Expression::Variable {
                            typ: (),
                            name: name.to_string(),
                        })
                    }
                }
            },
            Token::LeftBracket => {
                let expression = self.parse_expression(0)?;
                if self.tokens.next() != Some(Token::RightBracket) {
                    return Err("Expected ')'".into());
                }
                Ok(expression)
            }
            Token::Operator(operator) => {
                match operator.as_str() {
                    "++" | "--" => {
                        let target = self.parse_primary()?;
                        if Self::is_changeable(&target) {
                            Ok(if operator == "++" {
                                Expression::Increment {
                                    typ: (),
                                    expression: Box::new(target),
                                    postfix: false,
                                }
                            } else {
                                Expression::Decrement {
                                    typ: (),
                                    expression: Box::new(target),
                                    postfix: false,
                                }
                            })
                        } else {
                            Err(format!(
                                "Operator '{}' can only be applied to a variable or field",
                                operator
                            )
                            .into())
                        }
                    }
                    "-" => {
                        let expression = self.parse_expression(15)?; // Унарный минус имеет высокий приоритет
                        Ok(Expression::Negate {
                            typ: (),
                            expression: Box::new(expression),
                        })
                    }
                    "!" => {
                        let expression = self.parse_expression(15)?; // Унарное отрицание
                        Ok(Expression::Not {
                            typ: (),
                            expression: Box::new(expression),
                        })
                    }
                    _ => Err(format!("Unexpected unary operator: {}", operator).into()),
                }
            }
            _ => Err(format!("Unexpected token: {:?}", token).into()),
        }
    }

    fn parse_arguments(&mut self) -> ResBox<Vec<RawExpression>> {
        let mut arguments = vec![];
        if let Some(Token::RightBracket) = self.tokens.peek() {
            self.tokens.next();
            return Ok(arguments);
        }
        loop {
            arguments.push(self.parse_expression(0)?);
            match self.tokens.next() {
                Some(Token::Comma) => continue,
                Some(Token::RightBracket) => break,
                _ => return Err("Expected ',' or ')' in arguments".into()),
            }
        }
        Ok(arguments)
    }

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
            "+" => Ok(ExpressionBinaryOperator::Plus),
            "-" => Ok(ExpressionBinaryOperator::Minus),
            "*" => Ok(ExpressionBinaryOperator::Multiply),
            "/" => Ok(ExpressionBinaryOperator::Divide),
            "%" => Ok(ExpressionBinaryOperator::Modulo),
            _ => Err("Unknown binary operator".into()),
        }
    }

    fn binding_order(operator: &ExpressionBinaryOperator) -> (u8, u8) {
        match operator {
            ExpressionBinaryOperator::Assign
            | ExpressionBinaryOperator::AssignAdd
            | ExpressionBinaryOperator::AssignSub
            | ExpressionBinaryOperator::AssignMul
            | ExpressionBinaryOperator::AssignDiv => (1, 2),
            ExpressionBinaryOperator::Or => (3, 4),
            ExpressionBinaryOperator::And => (5, 6),
            ExpressionBinaryOperator::Equal | ExpressionBinaryOperator::NotEqual => (7, 8),
            ExpressionBinaryOperator::Less
            | ExpressionBinaryOperator::LessEqual
            | ExpressionBinaryOperator::Greater
            | ExpressionBinaryOperator::GreaterEqual => (9, 10),
            ExpressionBinaryOperator::Plus | ExpressionBinaryOperator::Minus => (11, 12),
            ExpressionBinaryOperator::Multiply
            | ExpressionBinaryOperator::Divide
            | ExpressionBinaryOperator::Modulo => (13, 14),
        }
    }
}

pub fn parse_expression(code: &str) -> ResBox<RawExpression> {
    let trimmed = code.trim().trim_end_matches(';');
    if trimmed.is_empty() {
        return Err("Empty expression".into());
    }
    let mut lexer = Lexer::new(trimmed);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    parser.parse_expression(0)
}
