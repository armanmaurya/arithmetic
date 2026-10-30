use super::tokenizer::Token;

pub struct Parser {
    pub tokens: Vec<Token>,
    pub pos: usize,
    use_radians: bool,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, use_radians: bool) -> Self {
        Self {
            tokens,
            pos: 0,
            use_radians,
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let tok = self.tokens.get(self.pos).cloned();
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    pub fn parse_expr(&mut self) -> Result<f64, String> {
        let mut val = self.parse_term()?;
        while let Some(tok) = self.peek() {
            match tok {
                Token::Plus => {
                    self.next();
                    val += self.parse_term()?;
                }
                Token::Minus => {
                    self.next();
                    val -= self.parse_term()?;
                }
                _ => break,
            }
        }
        Ok(val)
    }

    fn parse_term(&mut self) -> Result<f64, String> {
        let mut val = self.parse_power()?;
        while let Some(tok) = self.peek() {
            match tok {
                Token::Multiply => {
                    self.next();
                    val *= self.parse_power()?;
                }
                Token::Divide => {
                    self.next();
                    let rhs = self.parse_power()?;
                    if rhs.abs() < 1e-15 {
                        return Err("Can't divide by 0".into());
                    }
                    val /= rhs;
                }
                _ => break,
            }
        }
        Ok(val)
    }

    fn parse_power(&mut self) -> Result<f64, String> {
        let base = self.parse_unary()?;
        if let Some(Token::Power) = self.peek() {
            self.next();
            let exp = self.parse_power()?;
            Ok(base.powf(exp))
        } else {
            Ok(base)
        }
    }

    fn parse_unary(&mut self) -> Result<f64, String> {
        match self.peek() {
            Some(Token::Plus) => {
                self.next();
                self.parse_unary()
            }
            Some(Token::Minus) => {
                self.next();
                Ok(-self.parse_unary()?)
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<f64, String> {
        let mut val = self.parse_primary()?;
        while let Some(tok) = self.peek() {
            match tok {
                Token::Percent => {
                    self.next();
                    val /= 100.0;
                }
                Token::Factorial => {
                    self.next();
                    if val < 0.0 || (val - val.round()).abs() > 1e-9 || val > 170.0 {
                        return Err("Domain error".into());
                    }
                    let n = val.round() as u64;
                    let mut acc = 1.0_f64;
                    for k in 2..=n {
                        acc *= k as f64;
                    }
                    val = acc;
                }
                _ => break,
            }
        }
        Ok(val)
    }

    fn parse_primary(&mut self) -> Result<f64, String> {
        match self.next() {
            Some(Token::Number(n)) => Ok(n),
            Some(Token::LParen) => {
                let val = self.parse_expr()?;
                if let Some(Token::RParen) = self.peek() {
                    self.next();
                }
                Ok(val)
            }
            Some(Token::Func(name)) => {
                let arg = self.parse_primary()?;
                match name.as_str() {
                    "sin" => {
                        let a = if self.use_radians {
                            arg
                        } else {
                            arg.to_radians()
                        };
                        Ok(a.sin())
                    }
                    "cos" => {
                        let a = if self.use_radians {
                            arg
                        } else {
                            arg.to_radians()
                        };
                        Ok(a.cos())
                    }
                    "tan" => {
                        let a = if self.use_radians {
                            arg
                        } else {
                            arg.to_radians()
                        };
                        Ok(a.tan())
                    }
                    "asin" => {
                        if !(-1.0..=1.0).contains(&arg) {
                            Err("Domain error".into())
                        } else {
                            let r = arg.asin();
                            Ok(if self.use_radians { r } else { r.to_degrees() })
                        }
                    }
                    "acos" => {
                        if !(-1.0..=1.0).contains(&arg) {
                            Err("Domain error".into())
                        } else {
                            let r = arg.acos();
                            Ok(if self.use_radians { r } else { r.to_degrees() })
                        }
                    }
                    "atan" => {
                        let r = arg.atan();
                        Ok(if self.use_radians { r } else { r.to_degrees() })
                    }
                    "sqrt" => {
                        if arg < 0.0 {
                            Err("Keep it real".into())
                        } else {
                            Ok(arg.sqrt())
                        }
                    }
                    "ln" => {
                        if arg <= 0.0 {
                            Err("Domain error".into())
                        } else {
                            Ok(arg.ln())
                        }
                    }
                    "log" => {
                        if arg <= 0.0 {
                            Err("Domain error".into())
                        } else {
                            Ok(arg.log10())
                        }
                    }
                    _ => Err("Format error".into()),
                }
            }
            _ => Err("Format error".into()),
        }
    }
}
