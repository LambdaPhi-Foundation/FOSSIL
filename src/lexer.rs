use crate::token::{Token, TokenKind};

pub struct Lexer<'a> {
    input: &'a [u8],
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token();
            tokens.push(tok);
            if matches!(tokens.last().unwrap().kind, TokenKind::Eof) {
                break;
            }
        }
        tokens
    }

    fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();

        let start = self.pos;
        let line = self.line;
        let col = self.col;

        if self.is_at_end() {
            return Token::new(TokenKind::Eof, start, start, line, col);
        }

        let ch = self.peek();

        if ch.is_ascii_digit() {
            return self.read_number();
        }

        if ch.is_ascii_alphabetic() || ch == b'_' {
            return self.read_identifier_or_keyword();
        }

        match ch {
            b'"' => self.read_string(),
            b'\'' => self.read_char(),
            b'+' => self.consume_token(TokenKind::Plus),
            b'-' => {
                if self.peek_next() == b'>' {
                    self.pos += 2;
                    self.col += 2;
                    return Token::new(TokenKind::Arrow, start, self.pos, line, col);
                }
                self.consume_token(TokenKind::Minus)
            }
            b'*' => self.consume_token(TokenKind::Star),
            b'/' => self.consume_token(TokenKind::Slash),
            b'%' => self.consume_token(TokenKind::Percent),
            b'=' => {
                if self.peek_next() == b'=' {
                    self.pos += 2;
                    self.col += 2;
                    return Token::new(TokenKind::Eq, start, self.pos, line, col);
                }
                self.consume_token(TokenKind::EqSign)
            }
            b'!' => {
                if self.peek_next() == b'=' {
                    self.pos += 2;
                    self.col += 2;
                    return Token::new(TokenKind::Neq, start, self.pos, line, col);
                }
                self.consume_token(TokenKind::Not)
            }
            b'<' => {
                if self.peek_next() == b'=' {
                    self.pos += 2;
                    self.col += 2;
                    return Token::new(TokenKind::Le, start, self.pos, line, col);
                }
                if self.peek_next() == b'<' {
                    self.pos += 2;
                    self.col += 2;
                    return Token::new(TokenKind::LShift, start, self.pos, line, col);
                }
                self.consume_token(TokenKind::Lt)
            }
            b'>' => {
                if self.peek_next() == b'=' {
                    self.pos += 2;
                    self.col += 2;
                    return Token::new(TokenKind::Ge, start, self.pos, line, col);
                }
                if self.peek_next() == b'>' {
                    self.pos += 2;
                    self.col += 2;
                    return Token::new(TokenKind::RShift, start, self.pos, line, col);
                }
                self.consume_token(TokenKind::Gt)
            }
            b':' => {
                if self.peek_next() == b'=' {
                    self.pos += 2;
                    self.col += 2;
                    return Token::new(TokenKind::Assign, start, self.pos, line, col);
                }
                if self.peek_next() == b':' {
                    self.pos += 2;
                    self.col += 2;
                    return Token::new(TokenKind::DColon, start, self.pos, line, col);
                }
                self.consume_token(TokenKind::Colon)
            }
            b'&' => {
                if self.peek_next() == b'&' {
                    self.pos += 2;
                    self.col += 2;
                    return Token::new(TokenKind::And, start, self.pos, line, col);
                }
                self.consume_token(TokenKind::Amp)
            }
            b'|' => {
                if self.peek_next() == b'|' {
                    self.pos += 2;
                    self.col += 2;
                    return Token::new(TokenKind::Or, start, self.pos, line, col);
                }
                self.consume_token(TokenKind::Pipe)
            }
            b'^' => self.consume_token(TokenKind::Xor),
            b'~' => self.consume_token(TokenKind::Tilde),
            b'`' => self.consume_token(TokenKind::Backtick),
            b'(' => self.consume_token(TokenKind::LParen),
            b')' => self.consume_token(TokenKind::RParen),
            b'{' => self.consume_token(TokenKind::LBrace),
            b'}' => self.consume_token(TokenKind::RBrace),
            b'[' => self.consume_token(TokenKind::LBracket),
            b']' => self.consume_token(TokenKind::RBracket),
            b',' => self.consume_token(TokenKind::Comma),
            b'.' => self.consume_token(TokenKind::Dot),
            b';' => self.consume_token(TokenKind::Semicolon),
            _ => {
                let ch = self.advance();
                Token::new(
                    TokenKind::Error(format!("unexpected character: {}", ch as char)),
                    start,
                    self.pos,
                    line,
                    col,
                )
            }
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            while let Some(ch) = self.peek_if_not_end() {
                let next = self.peek_next();
                match ch {
                    b' ' | b'\t' | b'\r' => {
                        self.advance();
                    }
                    b'\n' => {
                        self.advance();
                        self.line += 1;
                        self.col = 1;
                    }
                    b'/' if next == b'/' => {
                        // line comment (//)
                        while !self.is_at_end() && self.peek() != b'\n' {
                            self.advance();
                        }
                    }
                    b';' => {
                        // line comment (;)
                        while !self.is_at_end() && self.peek() != b'\n' {
                            self.advance();
                        }
                    }
                    b'/' => return,
                    _ => return,
                }
            }
            if self.is_at_end() {
                return;
            }
        }
    }

    fn read_identifier_or_keyword(&mut self) -> Token {
        let start = self.pos;
        let line = self.line;
        let col = self.col;
        while !self.is_at_end() && (self.peek().is_ascii_alphanumeric() || self.peek() == b'_') {
            self.advance();
        }
        let text = std::str::from_utf8(&self.input[start..self.pos]).unwrap();
        let kind = crate::token::keyword_or_type(text)
            .unwrap_or_else(|| TokenKind::Ident(text.to_string()));
        
        // Special handling for ASM block - read until ENDASM
        if kind == TokenKind::Asm {
            return self.read_asm_block(start, line, col);
        }
        
        Token::new(kind, start, self.pos, line, col)
    }
    
    fn read_asm_block(&mut self, start: usize, line: usize, col: usize) -> Token {
        // Read everything until ENDASM
        let asm_start = self.pos;
        let mut depth = 0;
        while !self.is_at_end() {
            // Check for ENDASM
            if self.starts_with("ENDASM") {
                break;
            }
            self.advance();
        }
        let content = std::str::from_utf8(&self.input[asm_start..self.pos]).unwrap();
        // Now consume ENDASM
        if self.starts_with("ENDASM") {
            for _ in 0..6 { self.advance(); }
        }
        Token::new(TokenKind::AsmBlock(content.to_string()), start, self.pos, line, col)
    }
    
    fn starts_with(&self, s: &str) -> bool {
        let bytes = s.as_bytes();
        if self.pos + bytes.len() > self.input.len() {
            return false;
        }
        &self.input[self.pos..self.pos + bytes.len()] == bytes
    }

    fn read_number(&mut self) -> Token {
        let start = self.pos;
        let line = self.line;
        let col = self.col;
        let mut is_float = false;
        let mut is_hex = false;

        // Check for hex prefix 0x or 0X
        if !self.is_at_end() && self.peek() == b'0' {
            let next_pos = self.pos + 1;
            if next_pos < self.input.len() {
                let next_ch = self.input[next_pos];
                if next_ch == b'x' || next_ch == b'X' {
                    is_hex = true;
                    self.advance(); // consume '0'
                    self.advance(); // consume 'x' or 'X'
                    while !self.is_at_end() && self.peek().is_ascii_hexdigit() {
                        self.advance();
                    }
                }
            }
        }

        if !is_hex {
            while !self.is_at_end() && self.peek().is_ascii_digit() {
                self.advance();
            }

            if !self.is_at_end() && self.peek() == b'.' {
                // peek ahead to ensure it's a float and not a member access
                // Only treat as float if followed by a digit
                if self.peek_next_non_digit_check() {
                    self.advance(); // consume '.'
                    is_float = true;
                    while !self.is_at_end() && self.peek().is_ascii_digit() {
                        self.advance();
                    }
                }
            }
        }

        let text = std::str::from_utf8(&self.input[start..self.pos]).unwrap();
        let kind = if is_float {
            TokenKind::FloatLit(text.parse::<f64>().unwrap())
        } else if is_hex {
            let val = i64::from_str_radix(&text[2..], 16).unwrap();
            TokenKind::IntLit(val)
        } else {
            TokenKind::IntLit(text.parse::<i64>().unwrap())
        };
        Token::new(kind, start, self.pos, line, col)
    }

    // peek_next returns a digit AND current is '.' -> float
    fn peek_next_non_digit_check(&self) -> bool {
        let pos = self.pos + 1;
        pos < self.input.len() && self.input[pos].is_ascii_digit()
    }

    fn read_string(&mut self) -> Token {
        let start = self.pos;
        let line = self.line;
        let col = self.col;
        self.advance(); // consume opening "
        let mut s = String::new();
        while !self.is_at_end() && self.peek() != b'"' {
            if self.peek() == b'\\' {
                self.advance();
                let esc = self.peek();
                match esc {
                    b'n' => s.push('\n'),
                    b't' => s.push('\t'),
                    b'r' => s.push('\r'),
                    b'\\' => s.push('\\'),
                    b'"' => s.push('"'),
                    b'0' => s.push('\0'),
                    _ => s.push(esc as char),
                }
                self.advance();
            } else {
                s.push(self.peek() as char);
                self.advance();
            }
        }
        if !self.is_at_end() {
            self.advance(); // consume closing "
        }
        Token::new(TokenKind::StringLit(s), start, self.pos, line, col)
    }

    fn read_char(&mut self) -> Token {
        let start = self.pos;
        let line = self.line;
        let col = self.col;
        self.advance(); // consume opening '
        let ch = if !self.is_at_end() && self.peek() == b'\\' {
            self.advance();
            match self.peek() {
                b'n' => '\n',
                b't' => '\t',
                b'r' => '\r',
                b'\\' => '\\',
                b'\'' => '\'',
                b'0' => '\0',
                _ => self.peek() as char,
            }
        } else {
            self.peek() as char
        };
        self.advance();
        if !self.is_at_end() && self.peek() == b'\'' {
            self.advance();
        }
        Token::new(TokenKind::CharLit(ch), start, self.pos, line, col)
    }

    fn consume_token(&mut self, kind: TokenKind) -> Token {
        let start = self.pos;
        let line = self.line;
        let col = self.col;
        self.pos += 1;
        self.col += 1;
        Token::new(kind, start, self.pos, line, col)
    }

    fn peek(&self) -> u8 {
        if self.is_at_end() {
            b'\0'
        } else {
            self.input[self.pos]
        }
    }

    fn peek_next(&self) -> u8 {
        let pos = self.pos + 1;
        if pos >= self.input.len() {
            b'\0'
        } else {
            self.input[pos]
        }
    }

    fn peek_if_not_end(&self) -> Option<u8> {
        if self.is_at_end() {
            None
        } else {
            Some(self.input[self.pos])
        }
    }

    fn advance(&mut self) -> u8 {
        let ch = self.input[self.pos];
        self.pos += 1;
        if ch == b'\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        ch
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.input.len()
    }
}
