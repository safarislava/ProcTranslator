use crate::common::BoxError;

#[derive(Debug, Clone)]
pub enum Expression {
    Literal { value: String },
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
    fn new(input: &str) -> Self {
        Self { chars: input.chars().collect(), pos: 0, }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek();
        self.pos += 1;
        ch
    }

    fn skip_whitespaces(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn next_token(&mut self) -> Result<Token, BoxError> {
        self.skip_whitespaces();
        match self.peek() {
            None => Ok(Token::EOF),
            Some(ch) if ch.is_ascii_digit() => {
                let mut num = String::new();
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() || c == '.' {
                        num.push(c);
                        self.advance();
                    } else {
                        break;
                    }
                }
                Ok(Token::Num(num))
            }
            Some('"') => {
                self.advance();
                let mut s = String::new();
                while let Some(c) = self.peek() {
                    if c == '"' {
                        self.advance();
                        break;
                    }
                    s.push(c);
                    self.advance();
                }
                Ok(Token::Str(s))
            }
            Some(ch) if ch.is_alphabetic() => {
                let mut id = String::new();
                while let Some(c) = self.peek() {
                    if c.is_alphanumeric() {
                        id.push(c);
                        self.advance();
                    } else {
                        break;
                    }
                }
                Ok(Token::Id(id))
            }
            Some('(') => {
                self.advance();
                Ok(Token::LParen)
            }
            Some(')') => {
                self.advance();
                Ok(Token::RParen)
            }
            Some(',') => {
                self.advance();
                Ok(Token::Comma)
            }
            Some(_) => {
                let mut op = String::new();
                while let Some(c) = self.peek() {
                    if "+-*/%=<>!&|".contains(c) {
                        op.push(c);
                        self.advance();
                    } else {
                        break;
                    }
                }
                Ok(Token::Op(op))
            }
        }
    }

    fn tokenize(&mut self) -> Result<Vec<Token>, BoxError> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            let is_eof = token == Token::EOF;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::EOF)
    }

    fn advance(&mut self) -> &Token {
        self.pos += 1;
        self.current()
    }

    fn parse(&mut self) -> Result<Expression, BoxError> {
        let expr = self.parse_assignment()?;
        if *self.current() != Token::EOF {
            return Err(format!("Unexpected token: {:?}", self.current()).into());
        }
        Ok(expr)
    }

    fn parse_assignment(&mut self) -> Result<Expression, BoxError> {
        let left = self.parse_or()?;
        if let Token::Op(op) = self.current() && ["=", "+=", "-=", "*=", "/="].contains(&op.as_str()) {
            let op_name = op.clone();
            self.advance();
            let right = self.parse_assignment()?;
            if op_name == "=" && let Expression::Variable { name } = left {
                return Ok(Expression::Assign { name, value: Box::new(right) });
            }
            return Ok(Expression::BinaryOp { left: Box::new(left), op: op_name, right: Box::new(right) });
        }
        Ok(left)
    }

    fn parse_or(&mut self) -> Result<Expression, BoxError> {
        self.bin_op(Self::parse_and, &["||"])
    }

    fn parse_and(&mut self) -> Result<Expression, BoxError> {
        self.bin_op(Self::parse_equality, &["&&"])
    }

    fn parse_equality(&mut self) -> Result<Expression, BoxError> {
        self.bin_op(Self::parse_comparison, &["==", "!="])
    }

    fn parse_comparison(&mut self) -> Result<Expression, BoxError> {
        self.bin_op(Self::parse_additive, &["<", ">", "<=", ">="])
    }

    fn parse_additive(&mut self) -> Result<Expression, BoxError> {
        self.bin_op(Self::parse_multiplicative, &["+", "-"])
    }

    fn parse_multiplicative(&mut self) -> Result<Expression, BoxError> {
        self.bin_op(Self::parse_unary, &["*", "/", "%"])
    }

    fn bin_op<F>(&mut self, next: F, ops: &[&str]) -> Result<Expression, BoxError>
    where
        F: Fn(&mut Self) -> Result<Expression, BoxError>,
    {
        let mut left = next(self)?;
        while let Token::Op(op) = self.current() {
            if ops.contains(&op.as_str()) {
                let op_name = op.clone();
                self.advance();
                let right = next(self)?;
                left = Expression::BinaryOp { left: Box::new(left), op: op_name, right: Box::new(right) };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expression, BoxError> {
        match self.current() {
            Token::Op(op) if op == "++" || op == "--" => {
                let op_name = op.clone();
                self.advance();
                if let Token::Id(name) = self.current() {
                    let var_name = name.clone();
                    self.advance();
                    return Ok(if op_name == "++" {
                        Expression::Increment { name: var_name, postfix: false }
                    } else {
                        Expression::Decrement { name: var_name, postfix: false }
                    });
                }
                return Err(format!("Expected variable after '{}'", op_name).into());
            }
            Token::Op(op) if op == "-" || op == "!" => {
                let op_name = op.clone();
                self.advance();
                let operand = self.parse_unary()?;
                return Ok(Expression::BinaryOp {
                    left: Box::new(Expression::Literal { value: "0".into() }),
                    op: op_name,
                    right: Box::new(operand),
                });
            }
            _ => {}
        }

        let primary = self.parse_primary()?;

        match self.current() {
            Token::Op(op) if op == "++" || op == "--" => {
                if let Expression::Variable { name } = primary {
                    let op_name = op.clone();
                    self.advance();
                    return Ok(if op_name == "++" {
                        Expression::Increment { name, postfix: true }
                    } else {
                        Expression::Decrement { name, postfix: true }
                    });
                }
            }
            _ => {}
        }
        Ok(primary)
    }

    fn parse_primary(&mut self) -> Result<Expression, BoxError> {
        match self.current().clone() {
            Token::Num(v) => {
                self.advance();
                Ok(Expression::Literal { value: v })
            }
            Token::Str(v) => {
                self.advance();
                Ok(Expression::Literal {
                    value: format!("\"{}\"", v),
                })
            }
            Token::Id(name) => {
                self.advance();
                if *self.current() == Token::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    if *self.current() != Token::RParen {
                        args.push(self.parse_assignment()?);
                        while *self.current() == Token::Comma {
                            self.advance();
                            args.push(self.parse_assignment()?);
                        }
                    }
                    self.advance();
                    Ok(Expression::FunctionCall { name, arguments: args })
                } else {
                    Ok(Expression::Variable { name })
                }
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_assignment()?;
                self.advance();
                Ok(expr)
            }
            _ => Err(format!("Unexpected token: {:?}", self.current()).into()),
        }
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
