use crate::chunk;
use crate::scanner;
use crate::scanner::TokenKind;

use anyhow::Result;
use num_traits::{FromPrimitive, ToPrimitive};

// TODO: avoid the clones

// TODO: maybe split to Parser & Compiler
pub struct Compiler {
    scanner: scanner::Scanner,
    chunk: chunk::Chunk,
    current: scanner::Token,
    previous: scanner::Token,
    had_error: bool,
    panic_mode: bool,
}

#[derive(FromPrimitive, ToPrimitive)]
enum Precedence {
    None = 0,
    Assignment, // =
    Or,         // or
    And,        // and
    Equality,   // == !=
    Comparison, // < > <= >=
    Term,       // + -
    Factor,     // * /
    Unary,      // ! -
    Call,       // . ()
    Primary,
}

impl Precedence {
    pub fn next(&self) -> Precedence {
        let next = *self as u8 + 1;
        match Precedence::from_u8(next) {
            Some(p) => p,
            None => Precedence::None,
        }
    }
}

struct ParseRule {
    prefix: Option<fn(&mut Compiler)>,
    infix: Option<fn(&mut Compiler)>,
    precedence: Precedence,
}

impl Compiler {
    // TODO: bah ... don't really want a constructor here, just did it to avoid global var
    pub fn new() -> Compiler {
        let scanner = scanner::Scanner::new("");
        static EOF: scanner::Token = scanner::Token {
            kind: TokenKind::Eof,
            lexeme: String::new(),
            line: 0,
        };
        Compiler {
            scanner,
            chunk: chunk::Chunk::new(),
            current: EOF.clone(),
            previous: EOF.clone(),
            had_error: false,
            panic_mode: false,
        }
    }
    
    pub fn compile(&mut self, source: &str) -> Result<chunk::Chunk> {
        *self = Compiler::new();
        self.scanner = scanner::Scanner::new(source);

        self.advance();
        self.expression();

        if self.had_error {
            Err(anyhow::anyhow!("Compilation error"))
        } else {
            self.end();
            Ok(self.chunk.clone())
        }
    }

    fn get_rule(kind: &TokenKind) -> ParseRule {
        match kind {
            TokenKind::LeftParen => ParseRule {
                prefix: Some(Compiler::grouping),
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::RightParen => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::LeftBrace => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::RightBrace => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::Comma => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::Dot => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::Minus => ParseRule {
                prefix: Some(Compiler::unary),
                infix: Some(Compiler::binary),
                precedence: Precedence::Term,
            },
            TokenKind::Plus => ParseRule {
                prefix: None,
                infix: Some(Compiler::binary),
                precedence: Precedence::Term,
            },
            TokenKind::Semicolon => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::Slash => ParseRule {
                prefix: None,
                infix: Some(Compiler::binary),
                precedence: Precedence::Factor,
            },
            TokenKind::Star => ParseRule {
                prefix: None,
                infix: Some(Compiler::binary),
                precedence: Precedence::Factor,
            },
            TokenKind::Bang => ParseRule {
                prefix: Some(Compiler::unary),
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::BangEqual => ParseRule {
                prefix: None,
                infix: Some(Compiler::binary),
                precedence: Precedence::Equality,
            },
            TokenKind::Equal => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::EqualEqual => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::Equality,
            },
            TokenKind::Greater => ParseRule {
                prefix: None,
                infix: Some(Compiler::binary),
                precedence: Precedence::Comparison,
            },
            TokenKind::GreaterEqual => ParseRule {
                prefix: None,
                infix: Some(Compiler::binary),
                precedence: Precedence::Comparison,
            },
            TokenKind::Less => ParseRule {
                prefix: None,
                infix: Some(Compiler::binary),
                precedence: Precedence::Comparison,
            },
            TokenKind::LessEqual => ParseRule {
                prefix: None,
                infix: Some(Compiler::binary),
                precedence: Precedence::Comparison,
            },
            TokenKind::Identifier => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::String => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::Number => ParseRule {
                prefix: Some(Compiler::number),
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::And => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::Class => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::Else => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::False => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::For => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::Fun => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::If => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::Nil => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::Or => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::Print => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::Return => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::Super => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::This => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::True => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::Var => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::While => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::Error => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::Eof => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        }
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();

        let prefix_rule = Self::get_rule(&self.previous.kind).prefix;
        match prefix_rule {
            Some(prefix_rule) => prefix_rule(self),
            None => {
                self.error("Expect expression.");
                return;
            }
        };

        while precedence.to_u8() <= Self::get_rule(&self.current.kind).precedence.to_u8() {
            self.advance();
            let infix_rule = Self::get_rule(&self.previous.kind).infix;
            match infix_rule {
                Some(infix_rule) => infix_rule(self),
                None => {
                    self.error(r"Compiler internal error ¯\_(ツ)_/¯.");
                    return;
                }
            };
        }
    }

    fn advance(&mut self) {
        self.previous = self.current.clone();

        loop {
            self.current = self.scanner.scan_token();
            if self.current.kind != scanner::TokenKind::Error {
                break;
            }

            self.error_at_current(&self.current.lexeme.clone());
        }
    }

    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn error_at_current(&mut self, message: &str) {
        self.error_at(&self.current.clone(), message);
    }

    fn error(&mut self, message: &str) {
        self.error_at(&self.previous.clone(), message);
    }

    fn error_at(&mut self, token: &scanner::Token, message: &str) {
        if self.panic_mode {
            return;
        }
        self.panic_mode = true;
        eprint!("[line {}] Error", token.line);

        if token.kind == scanner::TokenKind::Eof {
            eprint!(" at end");
        } else if token.kind == scanner::TokenKind::Error {
            // Nothing
        } else {
            eprint!(" at '{}'", token.lexeme);
        }

        eprintln!(": {}", message);
        self.had_error = true;
    }

    fn consume(&mut self, kind: scanner::TokenKind, message: &str) {
        if self.current.kind == kind {
            self.advance();
            return;
        }

        self.error_at_current(message);
    }

    fn current_chunk(&mut self) -> &mut chunk::Chunk {
        &mut self.chunk
    }

    fn emit_byte(&mut self, byte: u8) {
        let line = self.previous.line;
        self.current_chunk().write_chunk(byte, line);
    }

    fn emit_bytes(&mut self, byte1: u8, byte2: u8) {
        self.emit_byte(byte1);
        self.emit_byte(byte2);
    }

    fn end(&mut self) {
        self.emit_return();
    }

    fn emit_return(&mut self) {
        self.emit_byte(chunk::OpCode::Return.u8());
    }

    fn number(&mut self) {
        let value = self.previous.lexeme.parse::<f64>().unwrap();
        self.emit_constant(value);
    }

    fn emit_constant(&mut self, value: f64) {
        let constant = self.make_constant(value);
        self.emit_bytes(chunk::OpCode::Constant.u8(), constant);
    }

    fn make_constant(&mut self, value: f64) -> u8 {
        let constant = self.current_chunk().add_constant(value);
        // TODO: ensure we don't allow more than u8::MAX constants
        //if constant > u8::MAX {
        //    self.error_at_current("Too many constants in one chunk.");
        //    return 0;
        //}

        constant
    }

    fn grouping(&mut self) {
        self.expression();
        self.consume(
            scanner::TokenKind::RightParen,
            "Expect ')' after expression.",
        );
    }

    fn unary(&mut self) {
        let operator_type = self.previous.clone().kind;

        // Compile the operand.
        self.parse_precedence(Precedence::Unary);

        // Emit the operator instruction.
        match operator_type {
            scanner::TokenKind::Minus => self.emit_byte(chunk::OpCode::Negate.u8()),
            _ => return, // Unreachable.
        }
    }

    fn binary(&mut self) {
        let operator_type = self.previous.clone().kind;
        let rule = Compiler::get_rule(&operator_type);
        self.parse_precedence(rule.precedence.next());

        match operator_type {
            scanner::TokenKind::Plus => self.emit_byte(chunk::OpCode::Add.u8()),
            scanner::TokenKind::Minus => self.emit_byte(chunk::OpCode::Subtract.u8()),
            scanner::TokenKind::Star => self.emit_byte(chunk::OpCode::Multiply.u8()),
            scanner::TokenKind::Slash => self.emit_byte(chunk::OpCode::Divide.u8()),
            _ => return, // Unreachable.
        }
    }
}
