use crate::common::BoxError;

#[derive(Debug, Clone)]
pub enum Expression {
    Literal(String),
    Variable { name: String },
    BinaryOp { left: Box<Expression>, op: String, right: Box<Expression> },
    FunctionCall { name: String, arguments: Vec<Expression> },
    Assign { name: String, value: Box<Expression> },
    Increment { name: String, postfix: bool },
    Decrement { name: String, postfix: bool },
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
    EOF,
}

struct Lexer {
    chars: Vec<char>,
    pos: usize,
}

impl Lexer {
    fn new(input: &str) -> Self { Self { chars: input.chars().collect(), pos: 0 } }
    fn peek(&self) -> Option<char> { self.chars.get(self.pos).copied() }
    fn advance(&mut self) -> Option<char> { let ch = self.peek(); self.pos += 1; ch }
    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) { self.advance(); }
    }

    fn read_while<F>(&mut self, cond: F) -> String where F: Fn(char) -> bool {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if cond(c) { s.push(c); self.advance(); } else { break; }
        }
        s
    }

    fn next_token(&mut self) -> Result<Token, BoxError> {
        self.skip_whitespace();
        match self.peek() {
            None => Ok(Token::EOF),
            Some(c) if c.is_ascii_digit() =>
                Ok(Token::Num(self.read_while(|c| c.is_ascii_digit() || c == '.'))),
            Some('"') => {
                self.advance();
                let s = self.read_while(|c| c != '"');
                self.advance(); Ok(Token::Str(s))
            }
            Some(c) if c.is_alphabetic() => {
                let id = self.read_while(|c| c.is_alphanumeric());
                match id.as_str() {
                    "true" | "false" => Ok(Token::Bool(id)),
                    _ => Ok(Token::Id(id))
                }
            }
            Some('(') => { self.advance(); Ok(Token::LParen) }
            Some(')') => { self.advance(); Ok(Token::RParen) }
            Some(',') => { self.advance(); Ok(Token::Comma) }
            Some(_) => Ok(Token::Op(self.read_while(|c| "+-*/%=<>!&|".contains(c)))),
        }
    }

    fn tokenize(&mut self) -> Result<Vec<Token>, BoxError> {
        let mut tokens = vec![];
        loop {
            let token = self.next_token()?;
            tokens.push(token.clone());
            if token == Token::EOF { break; }
        }
        Ok(tokens)
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self { Self { tokens, pos: 0 } }
    fn parse(&mut self) -> Result<Expression, BoxError> { self.parse_expression(0) }

    fn parse_expression(&mut self, min_power: u8) -> Result<Expression, BoxError> {
        let mut left = {
            let token = self.tokens.get(self.pos).cloned().unwrap_or(Token::EOF);
            match token {
                Token::Num(v) => {
                    self.pos += 1;
                    Expression::Literal(v)
                }
                Token::Str(v) => {
                    self.pos += 1;
                    Expression::Literal(format!("\"{}\"", v))
                }
                Token::Bool(v) => {
                    self.pos += 1;
                    Expression::Literal(v)
                }
                Token::Id(name) => {
                    self.pos += 1;
                    if self.tokens.get(self.pos) == Some(&Token::LParen) {
                        self.pos += 1;
                        let mut args = Vec::new();
                        while self.tokens.get(self.pos) != Some(&Token::RParen) {
                            args.push(self.parse_expression(0)?);
                            if self.tokens.get(self.pos) == Some(&Token::Comma) {
                                self.pos += 1;
                            }
                        }
                        self.pos += 1;
                        Expression::FunctionCall { name, arguments: args }
                    } else {
                        Expression::Variable { name }
                    }
                }
                Token::Op(op) if op == "++" || op == "--" => {
                    let op_name = op.clone(); self.pos += 1;
                    if let Token::Id(name) = self.tokens.get(self.pos).cloned().unwrap_or(Token::EOF) {
                        self.pos += 1;
                        match op_name.as_str() {
                            "++" => Expression::Increment { name, postfix: false },
                            _ => Expression::Decrement { name, postfix: false },
                        }
                    } else {
                        return Err(format!("Expected variable after '{}'", op_name).into());
                    }
                }
                Token::Op(op) if op == "-" || op == "!" => {
                    let op_name = op.clone(); self.pos += 1;
                    let right = self.parse_expression(10)?;
                    Expression::BinaryOp {
                        left: Box::new(Expression::Literal("0".into())),
                        op: op_name,
                        right: Box::new(right)
                    }
                }
                Token::LParen => {
                    self.pos += 1;
                    let expression = self.parse_expression(0)?;
                    self.pos += 1;
                    expression
                }
                _ => return Err(format!("Unexpected token: {:?}", token).into()),
            }
        };

        loop {
            let token = self.tokens.get(self.pos).cloned().unwrap_or(Token::EOF);
            let op = match token {
                Token::Op(ref op) if !["++", "--"].contains(&op.as_str()) => op.clone(),
                _ => break,
            };
            let (left_power, right_power) = Self::binding_power(&op)?;
            if left_power < min_power { break; }
            self.pos += 1;
            let right = self.parse_expression(right_power)?;
            if op == "=" {
                if let Expression::Variable { name } = left {
                    left = Expression::Assign { name, value: Box::new(right) };
                } else { return Err("Left side of assignment must be variable".into()); }
            } else {
                left = Expression::BinaryOp { left: Box::new(left), op, right: Box::new(right) };
            }
        }

        Ok(left)
    }

    fn binding_power(op: &str) -> Result<(u8, u8), BoxError> {
        let bp = match op {
            "=" | "+=" | "-=" | "*=" | "/=" => (1, 2),
            "||" => (3, 4),
            "&&" => (5, 6),
            "==" | "!=" => (7, 8),
            "<" | "<=" | ">" | ">=" => (9, 10),
            "+" | "-" => (11, 12),
            "*" | "/" | "%" => (13, 14),
            _ => return Err(format!("Unknown operator '{}'", op).into()),
        };
        Ok(bp)
    }
}

pub fn parse_expression(raw_code: String) -> Result<Expression, BoxError> {
    if raw_code.trim().is_empty() {
        return Err("Empty expression".into());
    }
    let mut lexer = Lexer::new(&raw_code);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    parser.parse()
}
