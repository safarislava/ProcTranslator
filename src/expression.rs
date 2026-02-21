use std::iter::Peekable;
use std::str::Chars;
use std::vec::IntoIter;
use crate::common::BoxError;

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
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
    Not,
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
    Negate,
    Increment,
    Decrement,
}

#[derive(Debug, Clone)]
pub enum Expression {
    Literal(String),
    Variable { name: String },
    BinaryOp { left: Box<Expression>, op: Operator, right: Box<Expression> },
    FunctionCall { name: String, arguments: Vec<Expression> },
    Assign { name: String, value: Box<Expression> },
    Increment { name: String },
    Decrement { name: String },
    Negate { expression: Box<Expression> },
    Not { expression: Box<Expression> },
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
}

struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self { chars: input.chars().peekable() }
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
            c if c.is_alphabetic() => {
                let id = self.read_while(|c| c.is_alphanumeric());
                match id.as_str() {
                    "true" | "false" => Token::Bool(id),
                    _ => Token::Id(id),
                }
            }
            '(' => { self.chars.next(); Token::LParen }
            ')' => { self.chars.next(); Token::RParen }
            ',' => { self.chars.next(); Token::Comma }
            _ => Token::Op(self.read_while(|c| "+-*/%=<>!&|".contains(c))),
        };

        Ok(Some(token))
    }

    fn tokenize(&mut self) -> Result<Vec<Token>, BoxError> {
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
        Self { tokens: tokens.into_iter().peekable() }
    }

    fn parse(&mut self) -> Result<Expression, BoxError> {
        self.parse_expression(0)
    }

    fn parse_expression(&mut self, min_order: u8) -> Result<Expression, BoxError> {
        let token = self.tokens.next().ok_or("Unexpected end of expression")?;

        let mut left = match token {
            Token::Num(v) => Expression::Literal(v),
            Token::Str(v) => Expression::Literal(format!("\"{}\"", v)),
            Token::Bool(v) => Expression::Literal(v),
            Token::Id(name) => {
                if let Some(Token::LParen) = self.tokens.peek() {
                    self.tokens.next();
                    let mut args = Vec::new();

                    while let Some(token) = self.tokens.peek() {
                        if token == &Token::RParen { break; }
                        args.push(self.parse_expression(0)?);
                        if let Some(Token::Comma) = self.tokens.peek() {
                            self.tokens.next();
                        }
                    }
                    if self.tokens.next() != Some(Token::RParen) {
                        return Err("Expected ')' after function arguments".into());
                    }
                    Expression::FunctionCall { name, arguments: args }
                } else {
                    Expression::Variable { name }
                }
            }
            Token::Op(op) if op == "++" || op == "--" => {
                if let Some(Token::Id(name)) = self.tokens.next() {
                    match op.as_str() {
                        "++" => Expression::Increment { name },
                        _ => Expression::Decrement { name },
                    }
                } else {
                    return Err(format!("Expected variable after '{}'", op).into());
                }
            }
            Token::Op(op) if op == "-" => {
                let right = self.parse_expression(10)?;
                Expression::Negate { expression: Box::new(right) }
            }
            Token::Op(op) if op == "!" => {
                let right = self.parse_expression(10)?;
                Expression::Not { expression: Box::new(right) }
            }
            Token::LParen => {
                let expression = self.parse_expression(0)?;
                if self.tokens.next() != Some(Token::RParen) {
                    return Err("Expected ')'".into());
                }
                expression
            }
            _ => return Err(format!("Unexpected token: {:?}", token).into()),
        };

        while let Some(Token::Op(op)) = self.tokens.peek() {
            if op == "++" || op == "--" {
                break;
            }

            let op = Self::link_binary_operator(op)?;
            let (left_order, right_order) = Self::binding_order(op)?;
            if left_order < min_order {
                break;
            }

            let op = if let Some(Token::Op(op)) = self.tokens.next() {
                Self::link_binary_operator(&op)? 
            } else { 
                unreachable!() 
            };

            let right = self.parse_expression(right_order)?;

            if op == Operator::Assign {
                if let Expression::Variable { name } = left {
                    left = Expression::Assign { name, value: Box::new(right) };
                } else {
                    return Err("Left side of assignment must be variable".into());
                }
            } else {
                left = Expression::BinaryOp { left: Box::new(left), op, right: Box::new(right) };
            }
        }

        Ok(left)
    }
    
    fn link_binary_operator(op: &str) -> Result<Operator, BoxError> {
        match op {
            "=" => Ok(Operator::Assign),
            "+=" => Ok(Operator::AssignAdd),
            "-=" => Ok(Operator::AssignSub),
            "*=" => Ok(Operator::AssignMul),
            "/=" => Ok(Operator::AssignDiv),
            "||" => Ok(Operator::Or),
            "&&" => Ok(Operator::And),
            "==" => Ok(Operator::Equal),
            "!=" => Ok(Operator::NotEqual),
            "<" => Ok(Operator::Less),
            "<=" => Ok(Operator::LessEqual),
            ">" => Ok(Operator::Greater),
            ">=" => Ok(Operator::GreaterEqual),
            "+" => Ok(Operator::Plus),
            "-" => Ok(Operator::Minus),
            "*" => Ok(Operator::Multiply),
            "/" => Ok(Operator::Divide),
            "%" => Ok(Operator::Modulo),
            _ => Err("Unknown binary operator".into()),
        }
    }

    fn binding_order(op: Operator) -> Result<(u8, u8), BoxError> {
        match op {
            Operator::Assign | 
            Operator::AssignAdd | Operator::AssignSub | 
            Operator::AssignMul | Operator::AssignDiv => Ok((1, 2)),
            Operator::Or => Ok((3, 4)),
            Operator::And => Ok((5, 6)),
            Operator::Equal | Operator::NotEqual => Ok((7, 8)),
            Operator::Less | Operator::LessEqual | 
            Operator::Greater | Operator::GreaterEqual => Ok((9, 10)),
            Operator::Plus | Operator::Minus => Ok((11, 12)),
            Operator::Multiply | Operator::Divide | Operator::Modulo => Ok((13, 14)),
            _ => Err(format!("Unknown operator: {:?}", op).into()),
        }
    }
}

pub fn parse_expression(raw_code: &str) -> Result<Expression, BoxError> {
    if raw_code.trim().is_empty() {
        return Err("Empty expression".into());
    }
    let mut lexer = Lexer::new(raw_code);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    parser.parse()
}