use crate::chunk;
use crate::scanner;
use crate::scanner::TokenKind;
use crate::value::*;

use anyhow::Result;
use num_traits::{FromPrimitive, ToPrimitive};

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

struct ParseRule<'a> {
    prefix: Option<fn(&mut Compiler<'a>, can_assign: bool) -> Result<()>>,
    infix: Option<fn(&mut Compiler<'a>) -> Result<()>>,
    precedence: Precedence,
}

struct Local {
    name: scanner::Token,
    depth: i32,
}

// TODO: maybe split to Parser & Compiler
pub struct Compiler<'a> {
    scanner: scanner::Scanner,
    chunk: &'a mut chunk::Chunk,
    current: scanner::Token,
    previous: scanner::Token,
    ran: bool,

    // TODO: these are defined in a separate "Compiler" struct in the book
    locals: Vec<Local>,
    scope_depth: i32,
}

impl<'a> Compiler<'a> {
    // TODO: bah ... don't really want a constructor here, just did it to avoid global var
    //  another alternative is to have a function with a closure as context
    pub fn new(source: &str, chunk: &'a mut chunk::Chunk) -> Compiler<'a> {
        let scanner = scanner::Scanner::new(source);
        static EOF: scanner::Token = scanner::Token {
            kind: TokenKind::Eof,
            lexeme: String::new(),
            line: 0,
        };
        Compiler {
            scanner,
            chunk,
            current: EOF.clone(),
            previous: EOF.clone(),
            ran: false,

            locals: Vec::with_capacity((u8::MAX as usize) + 1),
            scope_depth: 0,
        }
    }

    pub fn compile(&mut self) -> Result<()> {
        if self.ran {
            return Err(anyhow::anyhow!("Compiler can only be used once"));
        }

        self.advance()?;

        while !self.match_token(TokenKind::Eof)? {
            let res = self.declaration();
            if res.is_err() {
                self.synchronize()?;
            }
        }

        self.end();
        Ok(())
    }

    fn get_rule(kind: TokenKind) -> ParseRule<'a> {
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
                infix: Some(Compiler::binary),
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
                prefix: Some(Compiler::variable),
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::String => ParseRule {
                prefix: Some(Compiler::string),
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
                prefix: Some(Compiler::literal),
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
                prefix: Some(Compiler::literal),
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
                prefix: Some(Compiler::literal),
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

    fn parse_precedence(&mut self, precedence: Precedence) -> Result<()> {
        self.advance()?;

        let can_assign = precedence.to_u8() <= Precedence::Assignment.to_u8();

        let prefix_rule = Self::get_rule(self.previous.kind).prefix;
        match prefix_rule {
            Some(prefix_rule) => prefix_rule(self, can_assign)?,
            None => {
                return self.error("Expect expression.");
            }
        };

        while precedence.to_u8() <= Self::get_rule(self.current.kind).precedence.to_u8() {
            self.advance()?;
            let infix_rule = Self::get_rule(self.previous.kind).infix;
            match infix_rule {
                Some(infix_rule) => infix_rule(self)?,
                None => {
                    return self.error(r"Compiler internal error ¯\_(ツ)_/¯.");
                }
            }
        }

        if can_assign && self.match_token(TokenKind::Equal)? {
            return self.error("Invalid assignment target");
        }

        Ok(())
    }

    fn advance(&mut self) -> Result<()> {
        self.previous = self.current.clone();

        loop {
            self.current = self.scanner.scan_token();
            if self.current.kind != scanner::TokenKind::Error {
                break;
            }

            return self.error_at_current(&self.current.lexeme);
        }
        Ok(())
    }

    fn expression(&mut self) -> Result<()> {
        self.parse_precedence(Precedence::Assignment)
    }

    fn error_at_current(&self, message: &str) -> Result<()> {
        self.error_at(&self.current, message)
    }

    fn error(&self, message: &str) -> Result<()> {
        self.error_at(&self.previous, message)
    }

    fn error_at(&self, token: &scanner::Token, message: &str) -> Result<()> {
        eprint!("[line {}] Error", token.line);

        if token.kind == scanner::TokenKind::Eof {
            eprint!(" at end");
        } else if token.kind == scanner::TokenKind::Error {
            // Nothing
        } else {
            eprint!(" at '{}'", token.lexeme);
        }

        eprintln!(": {}", message);
        Err(anyhow::anyhow!("Compiler error"))
    }

    fn consume(&mut self, kind: scanner::TokenKind, message: &str) -> Result<()> {
        if self.current.kind == kind {
            return self.advance();
        }

        self.error_at_current(message)
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

    fn number(&mut self, _can_assign: bool) -> Result<()> {
        let value = self.previous.lexeme.parse::<f64>().unwrap();
        self.emit_constant(Value::Number(value))
    }

    fn emit_constant(&mut self, value: Value) -> Result<()> {
        let constant = self.make_constant(value)?;
        self.emit_bytes(chunk::OpCode::Constant.u8(), constant);
        Ok(())
    }

    fn make_constant(&mut self, value: Value) -> Result<u8> {
        self.current_chunk().add_constant(value)
    }

    fn grouping(&mut self, _can_assign: bool) -> Result<()> {
        self.expression()?;
        self.consume(
            scanner::TokenKind::RightParen,
            "Expect ')' after expression.",
        )?;
        Ok(())
    }

    fn unary(&mut self, _can_assign: bool) -> Result<()> {
        let operator_type = self.previous.kind.clone();

        // Compile the operand.
        self.parse_precedence(Precedence::Unary)?;

        // Emit the operator instruction.
        match operator_type {
            TokenKind::Minus => Ok(self.emit_byte(chunk::OpCode::Negate.u8())),
            TokenKind::Bang => Ok(self.emit_byte(chunk::OpCode::Not.u8())),
            _ => Err(anyhow::anyhow!("Internal compiler error")), // Unreachable.
        }
    }

    fn binary(&mut self) -> Result<()> {
        let operator_type = self.previous.kind.clone();
        let rule = Compiler::get_rule(operator_type);
        self.parse_precedence(rule.precedence.next())?;

        match operator_type {
            TokenKind::BangEqual => {
                self.emit_bytes(chunk::OpCode::Equal.u8(), chunk::OpCode::Not.u8())
            }
            TokenKind::EqualEqual => self.emit_byte(chunk::OpCode::Equal.u8()),
            TokenKind::Greater => self.emit_byte(chunk::OpCode::Greater.u8()),
            TokenKind::GreaterEqual => {
                self.emit_bytes(chunk::OpCode::Less.u8(), chunk::OpCode::Not.u8())
            }
            TokenKind::Less => self.emit_byte(chunk::OpCode::Less.u8()),
            TokenKind::LessEqual => {
                self.emit_bytes(chunk::OpCode::Greater.u8(), chunk::OpCode::Not.u8())
            }
            TokenKind::Plus => self.emit_byte(chunk::OpCode::Add.u8()),
            TokenKind::Minus => self.emit_byte(chunk::OpCode::Subtract.u8()),
            TokenKind::Star => self.emit_byte(chunk::OpCode::Multiply.u8()),
            TokenKind::Slash => self.emit_byte(chunk::OpCode::Divide.u8()),
            _ => return Err(anyhow::anyhow!("Internal compiler error")), // Unreachable.
        }
        Ok(())
    }

    fn literal(&mut self, _can_assign: bool) -> Result<()> {
        match self.previous.kind {
            scanner::TokenKind::False => self.emit_byte(chunk::OpCode::False.u8()),
            scanner::TokenKind::Nil => self.emit_byte(chunk::OpCode::Nil.u8()),
            scanner::TokenKind::True => self.emit_byte(chunk::OpCode::True.u8()),
            _ => return Err(anyhow::anyhow!("Internal compiler error")), // Unreachable.
        }
        Ok(())
    }

    fn string(&mut self, _can_assign: bool) -> Result<()> {
        let str_obj = Value::Obj(Obj::String(
            self.previous.lexeme[1..self.previous.lexeme.len() - 1].to_string(),
        ));
        self.emit_constant(str_obj)
    }

    fn declaration(&mut self) -> Result<()> {
        if self.match_token(scanner::TokenKind::Var)? {
            self.var_declaration()?;
        } else {
            self.statement()?;
        }
        Ok(())
    }

    fn statement(&mut self) -> Result<()> {
        if self.match_token(TokenKind::Print)? {
            self.print_statement()
        } else if self.match_token(TokenKind::If)? {
            self.if_statement()
        } else if self.match_token(TokenKind::LeftBrace)? {
            self.begin_scope();
            self.block()?;
            Ok(self.end_scope())
        } else {
            self.expresstion_statement()
        }
    }

    fn match_token(&mut self, kind: scanner::TokenKind) -> Result<bool> {
        if !self.check(kind) {
            return Ok(false);
        }
        self.advance()?;
        Ok(true)
    }

    fn check(&self, kind: scanner::TokenKind) -> bool {
        self.current.kind == kind
    }

    fn print_statement(&mut self) -> Result<()> {
        self.expression()?;
        self.consume(scanner::TokenKind::Semicolon, "Expect ';' after value.")?;
        self.emit_byte(chunk::OpCode::Print.u8());
        Ok(())
    }

    fn expresstion_statement(&mut self) -> Result<()> {
        self.expression()?;
        self.consume(scanner::TokenKind::Semicolon, "Expect ';' after value.")?;
        self.emit_byte(chunk::OpCode::Pop.u8());
        Ok(())
    }

    fn synchronize(&mut self) -> Result<()> {
        while self.current.kind != scanner::TokenKind::Eof {
            if self.previous.kind == scanner::TokenKind::Semicolon {
                return Ok(());
            }

            match self.current.kind {
                scanner::TokenKind::Class
                | scanner::TokenKind::Fun
                | scanner::TokenKind::Var
                | scanner::TokenKind::For
                | scanner::TokenKind::If
                | scanner::TokenKind::While
                | scanner::TokenKind::Print
                | scanner::TokenKind::Return => return Ok(()),
                _ => {}
            }

            self.advance()?
        }

        Ok(())
    }

    fn var_declaration(&mut self) -> Result<()> {
        let global = self.parse_variable("Expect variable name.")?;

        if self.match_token(scanner::TokenKind::Equal)? {
            self.expression()?;
        } else {
            self.emit_byte(chunk::OpCode::Nil.u8());
        }
        self.consume(
            scanner::TokenKind::Semicolon,
            "Expect ';' after variable declaration.",
        )?;

        self.define_variable(global);
        Ok(())
    }

    fn parse_variable(&mut self, error_message: &str) -> Result<u8> {
        self.consume(scanner::TokenKind::Identifier, error_message)?;

        self.declare_variable()?;
        if self.scope_depth > 0 {
            return Ok(0);
        }

        self.identifier_constant(self.previous.lexeme.clone())
    }

    fn identifier_constant(&mut self, name: String) -> Result<u8> {
        let str_obj = Value::Obj(Obj::String(name));
        self.make_constant(str_obj)
    }

    fn define_variable(&mut self, global: u8) {
        if self.scope_depth > 0 {
            self.mark_initialized();
            return;
        }

        self.emit_bytes(chunk::OpCode::DefineGlobal.u8(), global);
    }

    fn declare_variable(&mut self) -> Result<()> {
        if self.scope_depth == 0 {
            return Ok(());
        }

        for local in self.locals.iter().rev() {
            if local.depth >= 0 && local.depth < self.scope_depth {
                break;
            }

            if local.name.lexeme == self.previous.lexeme {
                return self.error("Already a variable with this name in this scope.");
            }
        }

        let name = self.previous.clone();
        self.add_local(name)
    }

    fn variable(&mut self, can_assign: bool) -> Result<()> {
        self.named_variable(self.previous.lexeme.clone(), can_assign)?;
        Ok(())
    }

    fn named_variable(&mut self, name: String, can_assign: bool) -> Result<()> {
        let arg: u8;

        let get_op: chunk::OpCode;
        let set_op: chunk::OpCode;

        if let Some(local_index) = self.resolve_local(&name)? {
            get_op = chunk::OpCode::GetLocal;
            set_op = chunk::OpCode::SetLocal;
            arg = local_index;
        } else {
            arg = self.identifier_constant(name)?;
            get_op = chunk::OpCode::GetGlobal;
            set_op = chunk::OpCode::SetGlobal;
        }

        if can_assign && self.match_token(scanner::TokenKind::Equal)? {
            self.expression()?;
            self.emit_bytes(set_op.u8(), arg);
        } else {
            self.emit_bytes(get_op.u8(), arg);
        }
        Ok(())
    }

    fn add_local(&mut self, name: scanner::Token) -> Result<()> {
        if self.locals.len() == u8::MAX as usize {
            return self.error("Too many local variables in function.");
        }

        self.locals.push(Local { name, depth: -1 });
        Ok(())
    }

    fn block(&mut self) -> Result<()> {
        while !self.check(TokenKind::RightBrace) && !self.check(TokenKind::Eof) {
            self.declaration()?;
        }

        self.consume(TokenKind::RightBrace, "Expect '}' after block.")
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.scope_depth -= 1;

        while self.locals.len() > 0 && self.locals.last().unwrap().depth > self.scope_depth {
            self.emit_byte(chunk::OpCode::Pop.u8());
            self.locals.pop();
        }
    }

    fn resolve_local(&self, name: &str) -> Result<Option<u8>> {
        // TODO: can assert that locals.len() < u8::MAX
        for i in (0..self.locals.len()).rev() {
            if self.locals[i].name.lexeme == name {
                if self.locals[i].depth == -1 {
                    return self
                        .error("Can't read local variable in its own initializer.")
                        .map(|_| None);
                }
                return Ok(Some(i as u8));
            }
        }

        Ok(None)
    }

    fn mark_initialized(&mut self) {
        if self.scope_depth == 0 {
            return;
        }

        self.locals.last_mut().unwrap().depth = self.scope_depth;
    }

    fn if_statement(&mut self) -> Result<()> {
        self.consume(TokenKind::LeftParen, "Expect '(' after 'if'.")?;
        self.expression()?;
        self.consume(TokenKind::RightParen, "Expect ')' after condition.")?;

        let then_jump = self.emit_jump(chunk::OpCode::JumpIfFalse);
        self.emit_byte(chunk::OpCode::Pop.u8());
        self.statement()?;

        let else_jump = self.emit_jump(chunk::OpCode::Jump);
        self.patch_jump(then_jump)?;
        self.emit_byte(chunk::OpCode::Pop.u8());

        if self.match_token(TokenKind::Else)? {
            self.statement()?;
        }

        self.patch_jump(else_jump)
    }

    fn emit_jump(&mut self, instruction: chunk::OpCode) -> usize {
        self.emit_byte(instruction.u8());
        self.emit_byte(0xff);
        self.emit_byte(0xff);
        self.current_chunk().code.len() - 2
    }

    fn patch_jump(&mut self, offset: usize) -> Result<()> {
        let jump = self.current_chunk().code.len() - offset - 2;

        if jump > u16::MAX as usize {
            return self.error("Too much code to jump over.");
        }

        self.current_chunk().code[offset] = ((jump >> 8) & 0xff) as u8;
        self.current_chunk().code[offset + 1] = (jump & 0xff) as u8;

        Ok(())
    }
}
