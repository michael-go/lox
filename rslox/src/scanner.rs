#[derive(PartialEq, Debug, Clone, Copy)]
pub enum TokenKind {
    // Single character tokens:
    LeftParen = 0,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    // One or two character tokens:
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    // Literals:
    Identifier,
    String,
    Number,
    // Keywords:
    And,
    Class,
    Else,
    False,
    Fun,
    For,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,
    // Error & EOF:
    Error,
    Eof,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String, // TODO: try to make it a ref
    pub line: usize,
}

pub struct Scanner {
    source_chars: Vec<char>,
    start_pos: usize,
    current_pos: usize,
    line: usize,
}

impl Scanner {
    pub fn new(source: &str) -> Scanner {
        Scanner {
            // this simplifies random access to the utf8 string,
            //  but we can make it more efficient by holding the ref to original string instead,
            //  and using start_pos and current_pos to index the utf8 byte positions
            source_chars: source.chars().collect(),
            start_pos: 0,
            current_pos: 0,
            line: 1,
        }
    }

    pub fn scan_token(&mut self) -> Token {
        self.skip_whitespace();
        self.start_pos = self.current_pos;

        if self.is_at_end() {
            return self.make_token(TokenKind::Eof);
        }

        let c = self.advance();

        if is_digit(c) {
            return self.number();
        }
        if is_alpha(c) {
            return self.identifier();
        }

        match c {
            '(' => self.make_token(TokenKind::LeftParen),
            ')' => self.make_token(TokenKind::RightParen),
            '{' => self.make_token(TokenKind::LeftBrace),
            '}' => self.make_token(TokenKind::RightBrace),
            ',' => self.make_token(TokenKind::Comma),
            '.' => self.make_token(TokenKind::Dot),
            '-' => self.make_token(TokenKind::Minus),
            '+' => self.make_token(TokenKind::Plus),
            '/' => self.make_token(TokenKind::Slash),
            ';' => self.make_token(TokenKind::Semicolon),
            '*' => self.make_token(TokenKind::Star),
            '!' => {
                if self.match_char('=') {
                    self.make_token(TokenKind::BangEqual)
                } else {
                    self.make_token(TokenKind::Bang)
                }
            }
            '=' => {
                if self.match_char('=') {
                    self.make_token(TokenKind::EqualEqual)
                } else {
                    self.make_token(TokenKind::Equal)
                }
            }
            '<' => {
                if self.match_char('=') {
                    self.make_token(TokenKind::LessEqual)
                } else {
                    self.make_token(TokenKind::Less)
                }
            }
            '>' => {
                if self.match_char('=') {
                    self.make_token(TokenKind::GreaterEqual)
                } else {
                    self.make_token(TokenKind::Greater)
                }
            }
            '"' => self.string(),

            _ => self.error_token("Unexpected character."),
        }
    }

    fn is_at_end(&self) -> bool {
        self.current_pos >= self.source_chars.len()
    }

    fn make_token(&self, kind: TokenKind) -> Token {
        Token {
            kind: kind,
            lexeme: self.source_chars[self.start_pos..self.current_pos]
                .iter()
                .collect(),
            line: self.line,
        }
    }

    fn error_token(&self, message: &str) -> Token {
        Token {
            kind: TokenKind::Error,
            lexeme: message.to_string(),
            line: self.line,
        }
    }

    fn advance(&mut self) -> char {
        let c = self.peek();
        self.current_pos += 1;
        c
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        }

        let c = self.peek();
        if c != expected {
            return false;
        }

        self.current_pos += 1;
        true
    }

    fn skip_whitespace(&mut self) {
        loop {
            if self.is_at_end() {
                break;
            }
            let c = self.peek();
            match c {
                ' ' | '\r' | '\t' => {
                    self.advance();
                }
                '\n' => {
                    self.line += 1;
                    self.advance();
                }
                '/' => {
                    if self.peek_next() == '/' {
                        // A comment goes until the end of the line.
                        while self.peek() != '\n' && !self.is_at_end() {
                            self.advance();
                        }
                    } else {
                        return;
                    }
                }
                _ => return,
            };
        }
    }

    fn peek(&self) -> char {
        if self.current_pos >= self.source_chars.len() {
            return '\0';
        }
        self.source_chars[self.current_pos]
    }

    fn peek_next(&self) -> char {
        if self.current_pos + 1 >= self.source_chars.len() {
            return '\0';
        }

        self.source_chars[self.current_pos + 1]
    }

    fn string(&mut self) -> Token {
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }
            self.advance();
        }

        if self.is_at_end() {
            return self.error_token("Unterminated string.");
        }

        // The closing ".
        self.advance();
        self.make_token(TokenKind::String)
    }

    fn number(&mut self) -> Token {
        while is_digit(self.peek()) {
            self.advance();
        }

        // Look for a fractional part.
        if self.peek() == '.' && is_digit(self.peek_next()) {
            // Consume the "."
            self.advance();

            while is_digit(self.peek()) {
                self.advance();
            }
        }

        self.make_token(TokenKind::Number)
    }

    fn identifier(&mut self) -> Token {
        while is_alpha_numeric(self.peek()) {
            self.advance();
        }

        self.make_token(self.identifier_type())
    }

    fn identifier_type(&self) -> TokenKind {
        match self.source_chars[self.start_pos..self.current_pos]
            .iter()
            .collect::<String>()
            .as_str()
        {
            "and" => TokenKind::And,
            "class" => TokenKind::Class,
            "else" => TokenKind::Else,
            "false" => TokenKind::False,
            "for" => TokenKind::For,
            "fun" => TokenKind::Fun,
            "if" => TokenKind::If,
            "nil" => TokenKind::Nil,
            "or" => TokenKind::Or,
            "print" => TokenKind::Print,
            "return" => TokenKind::Return,
            "super" => TokenKind::Super,
            "this" => TokenKind::This,
            "true" => TokenKind::True,
            "var" => TokenKind::Var,
            "while" => TokenKind::While,
            _ => TokenKind::Identifier,
        }
    }
}

fn is_digit(c: char) -> bool {
    c >= '0' && c <= '9'
}

fn is_alpha(c: char) -> bool {
    (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_'
}

fn is_alpha_numeric(c: char) -> bool {
    is_alpha(c) || is_digit(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan() {
        let mut scanner = Scanner::new(
            "
            fun foo() { // comment 
                return 3 * (4 + 5);
            }",
        );
        let mut tokens: Vec<super::Token> = Vec::new();
        loop {
            let token = scanner.scan_token();
            if token.kind == super::TokenKind::Eof {
                break;
            }
            tokens.push(token);
        }

        assert_eq!(
            tokens,
            vec![
                Token {
                    kind: TokenKind::Fun,
                    lexeme: "fun".to_string(),
                    line: 2
                },
                Token {
                    kind: TokenKind::Identifier,
                    lexeme: "foo".to_string(),
                    line: 2
                },
                Token {
                    kind: TokenKind::LeftParen,
                    lexeme: "(".to_string(),
                    line: 2
                },
                Token {
                    kind: TokenKind::RightParen,
                    lexeme: ")".to_string(),
                    line: 2
                },
                Token {
                    kind: TokenKind::LeftBrace,
                    lexeme: "{".to_string(),
                    line: 2
                },
                Token {
                    kind: TokenKind::Return,
                    lexeme: "return".to_string(),
                    line: 3
                },
                Token {
                    kind: TokenKind::Number,
                    lexeme: "3".to_string(),
                    line: 3
                },
                Token {
                    kind: TokenKind::Star,
                    lexeme: "*".to_string(),
                    line: 3
                },
                Token {
                    kind: TokenKind::LeftParen,
                    lexeme: "(".to_string(),
                    line: 3
                },
                Token {
                    kind: TokenKind::Number,
                    lexeme: "4".to_string(),
                    line: 3
                },
                Token {
                    kind: TokenKind::Plus,
                    lexeme: "+".to_string(),
                    line: 3
                },
                Token {
                    kind: TokenKind::Number,
                    lexeme: "5".to_string(),
                    line: 3
                },
                Token {
                    kind: TokenKind::RightParen,
                    lexeme: ")".to_string(),
                    line: 3
                },
                Token {
                    kind: TokenKind::Semicolon,
                    lexeme: ";".to_string(),
                    line: 3
                },
                Token {
                    kind: TokenKind::RightBrace,
                    lexeme: "}".to_string(),
                    line: 4
                },
            ]
        );
    }
}
