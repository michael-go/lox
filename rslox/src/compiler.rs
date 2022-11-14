use std::rc::Rc;

use crate::chunk;
use crate::object::*;
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

struct ParseRule {
    prefix: Option<fn(&mut Compiler, can_assign: bool) -> Result<()>>,
    infix: Option<fn(&mut Compiler) -> Result<()>>,
    precedence: Precedence,
}

#[derive(Clone)]
struct Local {
    name: scanner::Token,
    depth: i32,
}

#[derive(Clone)]
enum FunctionType {
    Function,
    Script,
}

#[derive(Clone)]
struct CompilationUnit {
    pub enclosing: Option<Box<CompilationUnit>>, // TODO: in clox it's a pointer to the stack, no heap allocation

    pub locals: Vec<Local>,
    pub scope_depth: i32,
    pub function_type: FunctionType,
    pub function: Function,
}

impl CompilationUnit {
    pub fn new(
        function_type: FunctionType,
        name: Option<String>,
        enclosing: Option<Box<CompilationUnit>>,
    ) -> CompilationUnit {
        let mut locals = Vec::<Local>::with_capacity((u8::MAX as usize) + 1);
        locals.push(Local {
            name: scanner::Token {
                kind: TokenKind::Eof,
                lexeme: String::new(),
                line: 0,
            },
            depth: 0,
        });
        let function = Function::new(name);
        CompilationUnit {
            enclosing,
            locals,
            scope_depth: 0,
            function_type,
            function,
        }
    }
}

pub struct Compiler {
    ran: bool,

    scanner: scanner::Scanner,
    current: scanner::Token,
    previous: scanner::Token,

    // TODO: this might need be a pointer
    comp_unit: CompilationUnit,
}

impl Compiler {
    // TODO: bah ... don't really want a constructor here, just did it to avoid global var
    //  another alternative is to have a function with a closure as context
    pub fn new(source: &str) -> Compiler {
        let scanner = scanner::Scanner::new(source);
        static EOF: scanner::Token = scanner::Token {
            kind: TokenKind::Eof,
            lexeme: String::new(),
            line: 0,
        };
        Compiler {
            ran: false,

            scanner,
            current: EOF.clone(),
            previous: EOF.clone(),

            comp_unit: CompilationUnit::new(FunctionType::Script, None, None),
        }
    }

    pub fn compile(&mut self) -> Result<Function> {
        if self.ran {
            return Err(anyhow::anyhow!("Compiler can only be used once"));
        }
        self.ran = true;

        let mut had_error = false;

        self.advance()?;

        while !self.match_token(TokenKind::Eof)? {
            let res = self.declaration();
            if res.is_err() {
                had_error = true;
                self.synchronize()?;
            }
        }

        if had_error {
            // TODO: return error (need to update some tests, so do in next commit)
        }
        // TODO: try to avoid the clone
        Ok(self.end_comp_unit())
    }

    fn get_rule(kind: TokenKind) -> ParseRule {
        match kind {
            TokenKind::LeftParen => ParseRule {
                prefix: Some(Compiler::grouping),
                infix: Some(Compiler::call),
                precedence: Precedence::Call,
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
                infix: Some(Compiler::and),
                precedence: Precedence::And,
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
                infix: Some(Compiler::or),
                precedence: Precedence::Or,
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
        &mut self.comp_unit.function.chunk
    }

    fn emit_byte(&mut self, byte: u8) {
        let line = self.previous.line;
        self.current_chunk().write_chunk(byte, line);
    }

    fn emit_bytes(&mut self, byte1: u8, byte2: u8) {
        self.emit_byte(byte1);
        self.emit_byte(byte2);
    }

    fn end_comp_unit(&mut self) -> Function {
        self.emit_return();

        let func = self.comp_unit.function.clone();

        // TODO: this is hacky, temporaryly doing it to avoid defining comp_unit as Option
        if self.comp_unit.enclosing.is_some() {
            self.comp_unit = *self.comp_unit.enclosing.take().unwrap();
        }

        return func;
    }

    fn emit_return(&mut self) {
        self.emit_byte(chunk::OpCode::Nil.u8());
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
        let str_obj = Value::Obj(Rc::new(
            self.previous.lexeme[1..self.previous.lexeme.len() - 1].to_string(),
        ));
        self.emit_constant(str_obj)
    }

    fn declaration(&mut self) -> Result<()> {
        if self.match_token(TokenKind::Fun)? {
            self.fun_declaration()
        } else if self.match_token(TokenKind::Var)? {
            self.var_declaration()
        } else {
            self.statement()
        }
    }

    fn statement(&mut self) -> Result<()> {
        if self.match_token(TokenKind::Print)? {
            self.print_statement()
        } else if self.match_token(TokenKind::If)? {
            self.if_statement()
        } else if self.match_token(TokenKind::For)? {
            self.for_statement()
        } else if self.match_token(TokenKind::Return)? {
            self.return_statement()
        } else if self.match_token(TokenKind::While)? {
            self.while_statement()
        } else if self.match_token(TokenKind::LeftBrace)? {
            self.begin_scope();
            self.block()?;
            Ok(self.end_scope())
        } else {
            self.expression_statement()
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

    fn expression_statement(&mut self) -> Result<()> {
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
        if self.comp_unit.scope_depth > 0 {
            return Ok(0);
        }

        self.identifier_constant(self.previous.lexeme.clone())
    }

    fn identifier_constant(&mut self, name: String) -> Result<u8> {
        let str_obj = Value::Obj(Rc::new(name));
        self.make_constant(str_obj)
    }

    fn define_variable(&mut self, global: u8) {
        if self.comp_unit.scope_depth > 0 {
            self.mark_initialized();
            return;
        }

        self.emit_bytes(chunk::OpCode::DefineGlobal.u8(), global);
    }

    fn declare_variable(&mut self) -> Result<()> {
        if self.comp_unit.scope_depth == 0 {
            return Ok(());
        }

        for local in self.comp_unit.locals.iter().rev() {
            if local.depth >= 0 && local.depth < self.comp_unit.scope_depth {
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
        if self.comp_unit.locals.len() == u8::MAX as usize {
            return self.error("Too many local variables in function.");
        }

        self.comp_unit.locals.push(Local { name, depth: -1 });
        Ok(())
    }

    fn block(&mut self) -> Result<()> {
        while !self.check(TokenKind::RightBrace) && !self.check(TokenKind::Eof) {
            self.declaration()?;
        }

        self.consume(TokenKind::RightBrace, "Expect '}' after block.")
    }

    fn begin_scope(&mut self) {
        self.comp_unit.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.comp_unit.scope_depth -= 1;

        while self.comp_unit.locals.len() > 0
            && self.comp_unit.locals.last().unwrap().depth > self.comp_unit.scope_depth
        {
            self.emit_byte(chunk::OpCode::Pop.u8());
            self.comp_unit.locals.pop();
        }
    }

    fn resolve_local(&self, name: &str) -> Result<Option<u8>> {
        // TODO: can assert that locals.len() < u8::MAX
        for i in (0..self.comp_unit.locals.len()).rev() {
            if self.comp_unit.locals[i].name.lexeme == name {
                if self.comp_unit.locals[i].depth == -1 {
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
        if self.comp_unit.scope_depth == 0 {
            return;
        }
        self.comp_unit.locals.last_mut().unwrap().depth = self.comp_unit.scope_depth;
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

    fn and(&mut self) -> Result<()> {
        let end_jump = self.emit_jump(chunk::OpCode::JumpIfFalse);

        self.emit_byte(chunk::OpCode::Pop.u8());
        self.parse_precedence(Precedence::And)?;

        self.patch_jump(end_jump)
    }

    fn or(&mut self) -> Result<()> {
        let else_jump = self.emit_jump(chunk::OpCode::JumpIfFalse);
        let end_jump = self.emit_jump(chunk::OpCode::Jump);

        self.patch_jump(else_jump)?;
        self.emit_byte(chunk::OpCode::Pop.u8());

        self.parse_precedence(Precedence::Or)?;
        self.patch_jump(end_jump)
    }

    fn while_statement(&mut self) -> Result<()> {
        let loop_start = self.current_chunk().code.len() - 1;

        self.consume(TokenKind::LeftParen, "Expect '(' after 'while'.")?;
        self.expression()?;
        self.consume(TokenKind::RightParen, "Expect ')' after condition.")?;

        let exit_jump = self.emit_jump(chunk::OpCode::JumpIfFalse);

        self.emit_byte(chunk::OpCode::Pop.u8());
        self.statement()?;

        self.emit_loop(loop_start)?;

        self.patch_jump(exit_jump)?;
        self.emit_byte(chunk::OpCode::Pop.u8());

        Ok(())
    }

    fn emit_loop(&mut self, loop_start: usize) -> Result<()> {
        let offset = self.current_chunk().code.len() - loop_start + 2;

        if offset > u16::MAX as usize {
            return self.error("Loop body too large.");
        }

        self.emit_byte(chunk::OpCode::Loop.u8());
        self.emit_byte(((offset >> 8) & 0xff) as u8);
        self.emit_byte((offset & 0xff) as u8);

        Ok(())
    }

    fn for_statement(&mut self) -> Result<()> {
        self.begin_scope();
        self.consume(TokenKind::LeftParen, "Expect '(' after 'for'.")?;

        // Initializer:
        if self.match_token(TokenKind::Semicolon)? {
            // No initializer.
        } else if self.match_token(TokenKind::Var)? {
            self.var_declaration()?;
        } else {
            self.expression_statement()?;
        }

        // Condition:
        let mut loop_start = self.current_chunk().code.len() - 1;
        let mut exit_jump: Option<usize> = None;
        if !self.match_token(TokenKind::Semicolon)? {
            self.expression()?;
            self.consume(TokenKind::Semicolon, "Expect ';'.")?;

            // Jump out of the loop if the condition is false.
            exit_jump = Some(self.emit_jump(chunk::OpCode::JumpIfFalse));
            self.emit_byte(chunk::OpCode::Pop.u8());
        }

        // Increment:
        if !self.match_token(TokenKind::RightParen)? {
            let body_jump = self.emit_jump(chunk::OpCode::Jump);
            let increment_start = self.current_chunk().code.len() - 1;
            self.expression()?;
            self.emit_byte(chunk::OpCode::Pop.u8());
            self.consume(TokenKind::RightParen, "Expect ')' after for clauses.")?;
            self.emit_loop(loop_start)?;
            loop_start = increment_start;
            self.patch_jump(body_jump)?;
        }

        // Body:
        self.statement()?;

        self.emit_loop(loop_start)?;
        if let Some(exit_jump) = exit_jump {
            self.patch_jump(exit_jump)?;
            self.emit_byte(chunk::OpCode::Pop.u8()); // Condition.
        }

        self.end_scope();
        Ok(())
    }

    fn fun_declaration(&mut self) -> Result<()> {
        let global = self.parse_variable("Expect function name.")?;
        self.mark_initialized();
        self.function(FunctionType::Function)?;
        self.define_variable(global);
        Ok(())
    }

    fn function(&mut self, function_type: FunctionType) -> Result<()> {
        let name = self.previous.lexeme.clone();
        let comp_unit = CompilationUnit::new(
            function_type,
            Some(name),
            Some(Box::new(self.comp_unit.clone())),
        );
        self.comp_unit = comp_unit;

        self.begin_scope();

        self.consume(TokenKind::LeftParen, "Expect '(' after function name.")?;
        if !self.check(TokenKind::RightParen) {
            loop {
                self.comp_unit.function.arity += 1;
                if self.comp_unit.function.arity > 255 {
                    return self.error_at_current("Cannot have more than 255 parameters.");
                }

                let constant = self.parse_variable("Expect parameter name.")?;
                self.define_variable(constant);

                if !self.match_token(TokenKind::Comma)? {
                    break;
                }
            }
        }
        self.consume(TokenKind::RightParen, "Expect ')' after function name.")?;
        self.consume(TokenKind::LeftBrace, "Expect '{' before function body.")?;
        self.block()?;

        let func = self.end_comp_unit();
        let constant = self.make_constant(Value::Obj(Rc::new(func)))?;
        self.emit_bytes(chunk::OpCode::Constant.u8(), constant);

        Ok(())
    }

    fn call(&mut self) -> Result<()> {
        let arg_count = self.argument_list()?;
        self.emit_bytes(chunk::OpCode::Call.u8(), arg_count);
        Ok(())
    }

    fn argument_list(&mut self) -> Result<u8> {
        let mut arg_count = 0;
        if !self.check(TokenKind::RightParen) {
            loop {
                self.expression()?;
                if arg_count == 255 {
                    return self
                        .error_at_current("Cannot have more than 255 arguments.")
                        .map(|_| 0);
                }
                arg_count += 1;
                if !self.match_token(TokenKind::Comma)? {
                    break;
                }
            }
        }
        self.consume(TokenKind::RightParen, "Expect ')' after arguments.")?;
        Ok(arg_count)
    }

    fn return_statement(&mut self) -> Result<()> {
        if let FunctionType::Script = self.comp_unit.function_type {
            return self.error("Cannot return from top-level code.");
        }

        if self.match_token(TokenKind::Semicolon)? {
            self.emit_return();
        } else {
            self.expression()?;
            self.consume(TokenKind::Semicolon, "Expect ';' after return value.")?;
            self.emit_byte(chunk::OpCode::Return.u8());
        }
        Ok(())
    }
}
