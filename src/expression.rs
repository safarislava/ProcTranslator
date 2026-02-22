use std::iter::Peekable;
use std::str::Chars;
use std::vec::IntoIter;
use crate::common::BoxError;

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
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
pub enum Expression {
    Literal(String),
    Variable { name: String },
    BinaryOp { left: Box<Expression>, op: BinaryOperator, right: Box<Expression> },
    FunctionCall { name: String, arguments: Vec<Expression> },
    MethodCall { object: Box<Expression>, name: String, arguments: Vec<Expression> },
    Assign { name: String, value: Box<Expression> },
    AssignField { object: Box<Expression>, name: String, value: Box<Expression> },
    Increment { expression: Box<Expression>, postfix: bool },
    Decrement { expression: Box<Expression>, postfix: bool },
    Negate { expression: Box<Expression> },
    Not { expression: Box<Expression> },
    New { class_name: String, arguments: Vec<Expression> },
    Field { object: Box<Expression>, name: String },
    This,
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Num(String),
    Str(String),
    Bool(String),
    Id(String),
    Op(String),
    LParen,
    RParen,
    Comma,
    Dot
}

struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self { Self { chars: input.chars().peekable() } }

    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.chars.peek() {
            if c.is_whitespace() {
                self.chars.next();
            } else {
                break;
            }
        }
    }

    fn read_while<F>(&mut self, cond: F) -> String where F: Fn(char) -> bool {
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

    fn next_token(&mut self) -> Result<Option<Token>, BoxError> {
        self.skip_whitespace();
        let &c = match self.chars.peek() {
            Some(c) => c,
            None => return Ok(None),
        };

        let token = match c {
            c if c.is_ascii_digit() =>
                Token::Num(self.read_while(|c| c.is_ascii_digit() || c == '.')),
            '"' => {
                self.chars.next();
                let s = self.read_while(|c| c != '"');
                self.chars.next();
                Token::Str(s)
            }
            c if c.is_alphabetic() || c == '_' => {
                let id = self.read_while(|c| c.is_alphanumeric() || c == '_');
                match id.as_str() {
                    "true" | "false" => Token::Bool(id),
                    _ => Token::Id(id),
                }
            }
            '(' => { self.chars.next(); Token::LParen }
            ')' => { self.chars.next(); Token::RParen }
            ',' => { self.chars.next(); Token::Comma }
            '.' => { self.chars.next(); Token::Dot }
            _ => {
                let op = self.read_while(|c| "+-*/%=<>!&|".contains(c));
                if op.is_empty() {
                    self.chars.next();
                    return self.next_token();
                }
                Token::Op(op)
            }
        };

        Ok(Some(token))
    }

    fn tokenize(&mut self) -> Result<Vec<Token>, BoxError> {
        let mut tokens = vec![];
        while let Some(token) = self.next_token()? { tokens.push(token); }
        Ok(tokens)
    }
}

struct Parser {
    tokens: Peekable<IntoIter<Token>>,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens: tokens.into_iter().peekable() }
    }

    fn is_changeable(expr: &Expression) -> bool {
        matches!(expr, Expression::Variable { .. } | Expression::Field { .. })
    }

    fn parse_expression(&mut self, min_order: u8) -> Result<Expression, BoxError> {
        let mut left = self.parse_primary()?;

        loop {
            let next = self.tokens.peek().cloned();

            if let Some(Token::Dot) = next {
                self.tokens.next();
                let member = match self.tokens.next() {
                    Some(Token::Id(id)) => id,
                    _ => return Err("Expected identifier after '.'".into()),
                };

                if let Some(Token::LParen) = self.tokens.peek() {
                    self.tokens.next();
                    let args = self.parse_arguments()?;
                    left = Expression::MethodCall { object: Box::new(left), name: member, arguments: args };
                } else {
                    left = Expression::Field { object: Box::new(left), name: member };
                }
                continue;
            }

            if let Some(Token::Op(op)) = next.as_ref() && (op == "++" || op == "--") {
                let op_str = op.clone();
                self.tokens.next();

                if Self::is_changeable(&left) {
                    left = if op_str == "++" {
                        Expression::Increment { expression: Box::new(left), postfix: true }
                    } else {
                        Expression::Decrement { expression: Box::new(left), postfix: true }
                    };
                } else {
                    return Err(format!("Operator '{}' can only be applied to a variable", op_str).into());
                }
                continue;
            }

            if let Some(Token::Op(op_str)) = next {
                let op = Self::link_binary_operator(&op_str)?;
                let (left_order, right_order) = Self::binding_order(&op);
                if left_order < min_order { 
                    break; 
                }
                self.tokens.next();

                let right = self.parse_expression(right_order)?;

                left = match op {
                    BinaryOperator::Assign => {
                        if !Self::is_changeable(&left) { return Err("Invalid assignment target".into()); }
                        match left {
                            Expression::Variable { name } =>
                                Expression::Assign { name, value: Box::new(right) },
                            Expression::Field { object, name: member } =>
                                Expression::AssignField { object, name: member, value: Box::new(right) },
                            _ => unreachable!()
                        }
                    },
                    _ => Expression::BinaryOp { left: Box::new(left), op, right: Box::new(right) },
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expression, BoxError> {
        let token = self.tokens.next().ok_or("Unexpected end of expression")?;
        match token {
            Token::Num(v) => Ok(Expression::Literal(v)),
            Token::Str(v) => Ok(Expression::Literal(format!("\"{}\"", v))),
            Token::Bool(v) => Ok(Expression::Literal(v)),
            Token::Id(id) => match id.as_str() {
                "this" => Ok(Expression::This),
                "new" => {
                    let class_name = match self.tokens.next() {
                        Some(Token::Id(name)) => name,
                        _ => return Err("Expected class name after 'new'".into()),
                    };
                    let arguments = if let Some(Token::LParen) = self.tokens.peek() {
                        self.tokens.next();
                        self.parse_arguments()?
                    } else { 
                        vec![] 
                    };
                    Ok(Expression::New { class_name, arguments })
                }
                name => {
                    if let Some(Token::LParen) = self.tokens.peek() {
                        self.tokens.next();
                        let args = self.parse_arguments()?;
                        Ok(Expression::FunctionCall { name: name.to_string(), arguments: args })
                    } else {
                        Ok(Expression::Variable { name: name.to_string() })
                    }
                }
            },
            Token::LParen => {
                let expr = self.parse_expression(0)?;
                if self.tokens.next() != Some(Token::RParen) { return Err("Expected ')'".into()); }
                Ok(expr)
            }
            Token::Op(op) if op == "++" || op == "--" => {
                let target = self.parse_primary()?;
                if Self::is_changeable(&target) {
                    Ok(if op == "++" {
                        Expression::Increment { expression: Box::new(target), postfix: false }
                    } else {
                        Expression::Decrement { expression: Box::new(target), postfix: false }
                    })
                } else {
                    Err(format!("Operator '{}' can only be applied to a variable or field", op).into())
                }
            }
            _ => Err(format!("Unexpected token: {:?}", token).into()),
        }
    }

    fn parse_arguments(&mut self) -> Result<Vec<Expression>, BoxError> {
        let mut args = vec![];
        if let Some(Token::RParen) = self.tokens.peek() {
            self.tokens.next();
            return Ok(args);
        }
        loop {
            args.push(self.parse_expression(0)?);
            match self.tokens.next() {
                Some(Token::Comma) => continue,
                Some(Token::RParen) => break,
                _ => return Err("Expected ',' or ')' in arguments".into()),
            }
        }
        Ok(args)
    }
    
    fn link_binary_operator(op: &str) -> Result<BinaryOperator, BoxError> {
        match op {
            "=" => Ok(BinaryOperator::Assign),
            "+=" => Ok(BinaryOperator::AssignAdd),
            "-=" => Ok(BinaryOperator::AssignSub),
            "*=" => Ok(BinaryOperator::AssignMul),
            "/=" => Ok(BinaryOperator::AssignDiv),
            "||" => Ok(BinaryOperator::Or),
            "&&" => Ok(BinaryOperator::And),
            "==" => Ok(BinaryOperator::Equal),
            "!=" => Ok(BinaryOperator::NotEqual),
            "<" => Ok(BinaryOperator::Less),
            "<=" => Ok(BinaryOperator::LessEqual),
            ">" => Ok(BinaryOperator::Greater),
            ">=" => Ok(BinaryOperator::GreaterEqual),
            "+" => Ok(BinaryOperator::Plus),
            "-" => Ok(BinaryOperator::Minus),
            "*" => Ok(BinaryOperator::Multiply),
            "/" => Ok(BinaryOperator::Divide),
            "%" => Ok(BinaryOperator::Modulo),
            _ => Err("Unknown binary operator".into()),
        }
    }

    fn binding_order(op: &BinaryOperator) -> (u8, u8) {
        match op {
            BinaryOperator::Assign | 
            BinaryOperator::AssignAdd | BinaryOperator::AssignSub | 
            BinaryOperator::AssignMul | BinaryOperator::AssignDiv => (1, 2),
            BinaryOperator::Or => (3, 4),
            BinaryOperator::And => (5, 6),
            BinaryOperator::Equal | BinaryOperator::NotEqual => (7, 8),
            BinaryOperator::Less | BinaryOperator::LessEqual | 
            BinaryOperator::Greater | BinaryOperator::GreaterEqual => (9, 10),
            BinaryOperator::Plus | BinaryOperator::Minus => (11, 12),
            BinaryOperator::Multiply | BinaryOperator::Divide | BinaryOperator::Modulo => (13, 14)
        }
    }
}

pub fn parse_expression(raw_code: &str) -> Result<Expression, BoxError> {
    let trimmed = raw_code.trim().trim_end_matches(';');
    if trimmed.is_empty() { return Err("Empty expression".into()); }
    let mut lexer = Lexer::new(trimmed);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    parser.parse_expression(0)
}