use std::rc::Rc;

use crate::chunk;
use crate::object::*;
use crate::scanner;
use crate::scanner::TokenKind;
use crate::value::*;

use anyhow::Result;
use num_traits::{FromPrimitive, ToPrimitive};

pub fn compile(source: &str) -> Result<Function> {
    Compiler::new(source).compile()
}

#[derive(Clone, Copy, FromPrimitive, ToPrimitive)]
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
    prefix: Option<fn(&mut Compiler, can_assign: bool)>,
    infix: Option<fn(&mut Compiler, can_assign: bool)>,
    precedence: Precedence,
}

#[derive(Clone)]
struct Local {
    name: scanner::Token,
    depth: i32,
    is_captured: bool,
}

#[derive(Clone, PartialEq)]
enum FunctionType {
    Function,
    Initializer,
    Method,
    Script,
}

#[derive(Clone)]
struct Upvalue {
    index: u8,
    is_local: bool,
}

struct CompUnitError {
    message: String,
}

impl CompUnitError {
    fn new(message: String) -> CompUnitError {
        CompUnitError { message }
    }
}

#[derive(Clone)]
struct CompilationUnit {
    pub enclosing: Option<Box<CompilationUnit>>, // TODO: in clox it's a pointer to the stack, no heap allocation

    pub locals: Vec<Local>,
    pub upvalues: Vec<Upvalue>,
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
                kind: TokenKind::Identifier,
                lexeme: if function_type != FunctionType::Function {
                    "this".to_string()
                } else {
                    "".to_string()
                },
                line: 0,
            },
            depth: 0,
            is_captured: false,
        });
        let function = Function::new(name);
        CompilationUnit {
            enclosing,
            locals,
            upvalues: Vec::new(),
            scope_depth: 0,
            function_type,
            function,
        }
    }

    fn resolve_local(&self, name: &str) -> Result<Option<u8>, CompUnitError> {
        // TODO: can assert that locals.len() < u8::MAX
        for i in (0..self.locals.len()).rev() {
            if self.locals[i].name.lexeme == name {
                if self.locals[i].depth == -1 {
                    return Err(CompUnitError::new(
                        "Can't read local variable in its own initializer.".to_string(),
                    ));
                }
                return Ok(Some(i as u8));
            }
        }

        Ok(None)
    }

    fn resolve_upvalue(&mut self, name: &str) -> Result<Option<u8>, CompUnitError> {
        if self.enclosing.is_none() {
            return Ok(None);
        }

        let local = self.enclosing.as_ref().unwrap().resolve_local(name)?;
        if let Some(local) = local {
            self.enclosing.as_mut().unwrap().locals[local as usize].is_captured = true;
            return Ok(Some(self.add_upvalue(local, true)?));
        }

        let upvalue = self.enclosing.as_mut().unwrap().resolve_upvalue(name)?;
        if let Some(upvalue) = upvalue {
            return Ok(Some(self.add_upvalue(upvalue, false)?));
        }

        Ok(None)
    }

    fn add_upvalue(&mut self, index: u8, is_local: bool) -> Result<u8, CompUnitError> {
        for i in 0..self.upvalues.len() {
            let upvalue = &self.upvalues[i];
            if upvalue.index == index && upvalue.is_local == is_local {
                return Ok(i as u8);
            }
        }

        if self.upvalues.len() == u8::MAX as usize {
            return Err(CompUnitError::new(
                "Too many closure variables in function.".to_string(),
            ));
        }

        self.function.upvalue_count += 1;
        let upvalue = Upvalue { index, is_local };
        self.upvalues.push(upvalue);
        Ok(self.upvalues.len() as u8 - 1)
    }
}

#[derive(Clone)]
struct ClassCompiler {
    enclosing: Box<Option<ClassCompiler>>,
    has_superclass: bool,
}

struct Compiler {
    ran: bool,
    had_error: bool,
    panic_mode: bool,

    scanner: scanner::Scanner,
    current: scanner::Token,
    previous: scanner::Token,

    // TODO: these might need be a pointer
    //  - in the book it's pointers to the stack, no heap allocation
    comp_unit: CompilationUnit,
    class_compiler: Option<ClassCompiler>,
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
            had_error: false,
            panic_mode: false,

            scanner,
            current: EOF.clone(),
            previous: EOF.clone(),

            comp_unit: CompilationUnit::new(FunctionType::Script, None, None),
            class_compiler: None,
        }
    }

    pub fn compile(&mut self) -> Result<Function> {
        if self.ran {
            return Err(anyhow::anyhow!("Compiler can only be used once"));
        }
        self.ran = true;

        self.advance();

        while !self.match_token(TokenKind::Eof) {
            self.declaration();
        }

        if self.had_error {
            return Err(anyhow::anyhow!("Had compilation errors"));
        }
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
                infix: Some(Compiler::dot),
                precedence: Precedence::Call,
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
                prefix: Some(Compiler::super_),
                infix: None,
                precedence: Precedence::None,
            },
            TokenKind::This => ParseRule {
                prefix: Some(Compiler::this),
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

    fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();

        let can_assign = precedence.to_u8() <= Precedence::Assignment.to_u8();

        let prefix_rule = Self::get_rule(self.previous.kind).prefix;
        match prefix_rule {
            Some(prefix_rule) => prefix_rule(self, can_assign),
            None => {
                self.error("Expect expression.");
                return;
            }
        };

        while precedence.to_u8() <= Self::get_rule(self.current.kind).precedence.to_u8() {
            self.advance();
            let infix_rule = Self::get_rule(self.previous.kind).infix;
            match infix_rule {
                Some(infix_rule) => infix_rule(self, can_assign),
                None => {
                    self.error(r"Compiler internal error ¯\_(ツ)_/¯.");
                }
            }
        }

        if can_assign && self.match_token(TokenKind::Equal) {
            self.error("Invalid assignment target");
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
        self.parse_precedence(Precedence::Assignment)
    }

    fn error_at_current(&mut self, message: &str) {
        self.error_at(&self.current.clone(), message);
    }

    fn error(&mut self, message: &str) {
        self.error_at(&self.previous.clone(), message);
    }

    fn consume(&mut self, kind: scanner::TokenKind, message: &str) {
        if self.current.kind == kind {
            self.advance();
            return;
        }

        self.error_at_current(message);
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
        if self.comp_unit.function_type == FunctionType::Initializer {
            self.emit_bytes(chunk::OpCode::GetLocal.u8(), 0)
        } else {
            self.emit_byte(chunk::OpCode::Nil.u8());
        }
        self.emit_byte(chunk::OpCode::Return.u8());
    }

    fn number(&mut self, _can_assign: bool) {
        let value = self.previous.lexeme.parse::<f64>().unwrap();
        self.emit_constant(Value::Number(value))
    }

    fn emit_constant(&mut self, value: Value) {
        let constant = self.make_constant(value);
        self.emit_bytes(chunk::OpCode::Constant.u8(), constant);
    }

    fn make_constant(&mut self, value: Value) -> u8 {
        let constant = self.current_chunk().add_constant(value);
        if constant > u8::MAX as usize {
            self.error("Too many constants in one chunk.");
            return 0;
        }
        return constant as u8;
    }

    fn grouping(&mut self, _can_assign: bool) {
        self.expression();
        self.consume(
            scanner::TokenKind::RightParen,
            "Expect ')' after expression.",
        );
    }

    fn unary(&mut self, _can_assign: bool) {
        let operator_type = self.previous.kind.clone();

        // Compile the operand.
        self.parse_precedence(Precedence::Unary);

        // Emit the operator instruction.
        match operator_type {
            TokenKind::Minus => self.emit_byte(chunk::OpCode::Negate.u8()),
            TokenKind::Bang => self.emit_byte(chunk::OpCode::Not.u8()),
            _ => panic!("Internal compiler error"), // Unreachable.
        }
    }

    fn binary(&mut self, _can_assign: bool) {
        let operator_type = self.previous.kind.clone();
        let rule = Compiler::get_rule(operator_type);
        self.parse_precedence(rule.precedence.next());

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
            _ => panic!("Internal compiler error"), // Unreachable.
        }
    }

    fn literal(&mut self, _can_assign: bool) {
        match self.previous.kind {
            scanner::TokenKind::False => self.emit_byte(chunk::OpCode::False.u8()),
            scanner::TokenKind::Nil => self.emit_byte(chunk::OpCode::Nil.u8()),
            scanner::TokenKind::True => self.emit_byte(chunk::OpCode::True.u8()),
            _ => panic!("Internal compiler error"), // Unreachable.
        }
    }

    fn string(&mut self, _can_assign: bool) {
        let str_obj = Value::Obj(Rc::new(ObjString::new(
            self.previous.lexeme[1..self.previous.lexeme.len() - 1].to_string(),
        )));
        self.emit_constant(str_obj)
    }

    fn declaration(&mut self) {
        if self.match_token(TokenKind::Class) {
            self.class_declaration()
        } else if self.match_token(TokenKind::Fun) {
            self.fun_declaration()
        } else if self.match_token(TokenKind::Var) {
            self.var_declaration()
        } else {
            self.statement()
        }

        if self.panic_mode {
            self.synchronize();
        }
    }

    fn statement(&mut self) {
        if self.match_token(TokenKind::Print) {
            self.print_statement()
        } else if self.match_token(TokenKind::If) {
            self.if_statement()
        } else if self.match_token(TokenKind::For) {
            self.for_statement()
        } else if self.match_token(TokenKind::Return) {
            self.return_statement()
        } else if self.match_token(TokenKind::While) {
            self.while_statement()
        } else if self.match_token(TokenKind::LeftBrace) {
            self.begin_scope();
            self.block();
            self.end_scope();
        } else {
            self.expression_statement()
        }
    }

    fn match_token(&mut self, kind: scanner::TokenKind) -> bool {
        if !self.check(kind) {
            return false;
        }
        self.advance();
        true
    }

    fn check(&self, kind: scanner::TokenKind) -> bool {
        self.current.kind == kind
    }

    fn print_statement(&mut self) {
        self.expression();
        self.consume(scanner::TokenKind::Semicolon, "Expect ';' after value.");
        self.emit_byte(chunk::OpCode::Print.u8());
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.consume(scanner::TokenKind::Semicolon, "Expect ';' after value.");
        self.emit_byte(chunk::OpCode::Pop.u8());
    }

    fn synchronize(&mut self) {
        self.panic_mode = false;

        while self.current.kind != scanner::TokenKind::Eof {
            if self.previous.kind == scanner::TokenKind::Semicolon {
                return;
            }

            match self.current.kind {
                scanner::TokenKind::Class
                | scanner::TokenKind::Fun
                | scanner::TokenKind::Var
                | scanner::TokenKind::For
                | scanner::TokenKind::If
                | scanner::TokenKind::While
                | scanner::TokenKind::Print
                | scanner::TokenKind::Return => return,
                _ => {}
            }

            self.advance()
        }
    }

    fn var_declaration(&mut self) {
        let global = self.parse_variable("Expect variable name.");

        if self.match_token(scanner::TokenKind::Equal) {
            self.expression();
        } else {
            self.emit_byte(chunk::OpCode::Nil.u8());
        }
        self.consume(
            scanner::TokenKind::Semicolon,
            "Expect ';' after variable declaration.",
        );

        self.define_variable(global);
    }

    fn parse_variable(&mut self, error_message: &str) -> u8 {
        self.consume(scanner::TokenKind::Identifier, error_message);

        self.declare_variable();
        if self.comp_unit.scope_depth > 0 {
            return 0;
        }

        self.identifier_constant(self.previous.lexeme.clone())
    }

    fn identifier_constant(&mut self, name: String) -> u8 {
        let str_obj = Value::Obj(Rc::new(ObjString::new(name)));
        self.make_constant(str_obj)
    }

    fn define_variable(&mut self, global: u8) {
        if self.comp_unit.scope_depth > 0 {
            self.mark_initialized();
            return;
        }

        self.emit_bytes(chunk::OpCode::DefineGlobal.u8(), global);
    }

    fn declare_variable(&mut self) {
        if self.comp_unit.scope_depth == 0 {
            return;
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

    fn variable(&mut self, can_assign: bool) {
        self.named_variable(self.previous.lexeme.clone(), can_assign);
    }

    fn named_variable(&mut self, name: String, can_assign: bool) {
        let arg: u8;

        let get_op: chunk::OpCode;
        let set_op: chunk::OpCode;

        if let Some(local_index) = self.comp_unit.resolve_local(&name).unwrap_or_else(|e| {
            self.error_at(&self.previous.clone(), &e.message);
            None
        }) {
            get_op = chunk::OpCode::GetLocal;
            set_op = chunk::OpCode::SetLocal;
            arg = local_index;
        } else if let Some(upvalue_index) =
            self.comp_unit.resolve_upvalue(&name).unwrap_or_else(|e| {
                self.error_at(&self.previous.clone(), &e.message);
                None
            })
        {
            get_op = chunk::OpCode::GetUpvalue;
            set_op = chunk::OpCode::SetUpvalue;
            arg = upvalue_index;
        } else {
            arg = self.identifier_constant(name);
            get_op = chunk::OpCode::GetGlobal;
            set_op = chunk::OpCode::SetGlobal;
        }

        if can_assign && self.match_token(scanner::TokenKind::Equal) {
            self.expression();
            self.emit_bytes(set_op.u8(), arg);
        } else {
            self.emit_bytes(get_op.u8(), arg);
        }
    }

    fn add_local(&mut self, name: scanner::Token) {
        if self.comp_unit.locals.len() == u8::MAX as usize {
            self.error("Too many local variables in function.");
            return;
        }

        self.comp_unit.locals.push(Local {
            name,
            depth: -1,
            is_captured: false,
        });
    }

    fn block(&mut self) {
        while !self.check(TokenKind::RightBrace) && !self.check(TokenKind::Eof) {
            self.declaration();
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
            if self.comp_unit.locals.last().unwrap().is_captured {
                self.emit_byte(chunk::OpCode::CloseUpvalue.u8());
            } else {
                self.emit_byte(chunk::OpCode::Pop.u8());
            }
            self.comp_unit.locals.pop();
        }
    }

    fn mark_initialized(&mut self) {
        if self.comp_unit.scope_depth == 0 {
            return;
        }
        self.comp_unit.locals.last_mut().unwrap().depth = self.comp_unit.scope_depth;
    }

    fn if_statement(&mut self) {
        self.consume(TokenKind::LeftParen, "Expect '(' after 'if'.");
        self.expression();
        self.consume(TokenKind::RightParen, "Expect ')' after condition.");

        let then_jump = self.emit_jump(chunk::OpCode::JumpIfFalse);
        self.emit_byte(chunk::OpCode::Pop.u8());
        self.statement();

        let else_jump = self.emit_jump(chunk::OpCode::Jump);
        self.patch_jump(then_jump);
        self.emit_byte(chunk::OpCode::Pop.u8());

        if self.match_token(TokenKind::Else) {
            self.statement();
        }

        self.patch_jump(else_jump)
    }

    fn emit_jump(&mut self, instruction: chunk::OpCode) -> usize {
        self.emit_byte(instruction.u8());
        self.emit_byte(0xff);
        self.emit_byte(0xff);
        self.current_chunk().code.len() - 2
    }

    fn patch_jump(&mut self, offset: usize) {
        let jump = self.current_chunk().code.len() - offset - 2;

        if jump > u16::MAX as usize {
            self.error("Too much code to jump over.");
        }

        self.current_chunk().code[offset] = ((jump >> 8) & 0xff) as u8;
        self.current_chunk().code[offset + 1] = (jump & 0xff) as u8;
    }

    fn and(&mut self, _can_assign: bool) {
        let end_jump = self.emit_jump(chunk::OpCode::JumpIfFalse);

        self.emit_byte(chunk::OpCode::Pop.u8());
        self.parse_precedence(Precedence::And);

        self.patch_jump(end_jump)
    }

    fn or(&mut self, _can_assign: bool) {
        let else_jump = self.emit_jump(chunk::OpCode::JumpIfFalse);
        let end_jump = self.emit_jump(chunk::OpCode::Jump);

        self.patch_jump(else_jump);
        self.emit_byte(chunk::OpCode::Pop.u8());

        self.parse_precedence(Precedence::Or);
        self.patch_jump(end_jump)
    }

    fn while_statement(&mut self) {
        let loop_start = self.current_chunk().code.len() - 1;

        self.consume(TokenKind::LeftParen, "Expect '(' after 'while'.");
        self.expression();
        self.consume(TokenKind::RightParen, "Expect ')' after condition.");

        let exit_jump = self.emit_jump(chunk::OpCode::JumpIfFalse);

        self.emit_byte(chunk::OpCode::Pop.u8());
        self.statement();

        self.emit_loop(loop_start);

        self.patch_jump(exit_jump);
        self.emit_byte(chunk::OpCode::Pop.u8());
    }

    fn emit_loop(&mut self, loop_start: usize) {
        let offset = self.current_chunk().code.len() - loop_start + 2;

        if offset > u16::MAX as usize {
            self.error("Loop body too large.");
        }

        self.emit_byte(chunk::OpCode::Loop.u8());
        self.emit_byte(((offset >> 8) & 0xff) as u8);
        self.emit_byte((offset & 0xff) as u8);
    }

    fn for_statement(&mut self) {
        self.begin_scope();
        self.consume(TokenKind::LeftParen, "Expect '(' after 'for'.");

        // Initializer:
        if self.match_token(TokenKind::Semicolon) {
            // No initializer.
        } else if self.match_token(TokenKind::Var) {
            self.var_declaration();
        } else {
            self.expression_statement();
        }

        // Condition:
        let mut loop_start = self.current_chunk().code.len() - 1;
        let mut exit_jump: Option<usize> = None;
        if !self.match_token(TokenKind::Semicolon) {
            self.expression();
            self.consume(TokenKind::Semicolon, "Expect ';'.");

            // Jump out of the loop if the condition is false.
            exit_jump = Some(self.emit_jump(chunk::OpCode::JumpIfFalse));
            self.emit_byte(chunk::OpCode::Pop.u8());
        }

        // Increment:
        if !self.match_token(TokenKind::RightParen) {
            let body_jump = self.emit_jump(chunk::OpCode::Jump);
            let increment_start = self.current_chunk().code.len() - 1;
            self.expression();
            self.emit_byte(chunk::OpCode::Pop.u8());
            self.consume(TokenKind::RightParen, "Expect ')' after for clauses.");
            self.emit_loop(loop_start);
            loop_start = increment_start;
            self.patch_jump(body_jump);
        }

        // Body:
        self.statement();

        self.emit_loop(loop_start);
        if let Some(exit_jump) = exit_jump {
            self.patch_jump(exit_jump);
            self.emit_byte(chunk::OpCode::Pop.u8()); // Condition.
        }

        self.end_scope();
    }

    fn fun_declaration(&mut self) {
        let global = self.parse_variable("Expect function name.");
        self.mark_initialized();
        self.function(FunctionType::Function);
        self.define_variable(global);
    }

    fn function(&mut self, function_type: FunctionType) {
        let name = self.previous.lexeme.clone();
        let comp_unit = CompilationUnit::new(
            function_type,
            Some(name),
            Some(Box::new(self.comp_unit.clone())),
        );
        self.comp_unit = comp_unit;

        self.begin_scope();

        self.consume(TokenKind::LeftParen, "Expect '(' after function name.");
        if !self.check(TokenKind::RightParen) {
            loop {
                self.comp_unit.function.arity += 1;
                if self.comp_unit.function.arity > 255 {
                    self.error_at_current("Can't have more than 255 parameters.");
                }

                let constant = self.parse_variable("Expect parameter name.");
                self.define_variable(constant);

                if !self.match_token(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.consume(TokenKind::RightParen, "Expect ')' after function name.");
        self.consume(TokenKind::LeftBrace, "Expect '{' before function body.");
        self.block();

        let upvalues = self.comp_unit.upvalues.clone();
        let func = self.end_comp_unit();
        let upvalue_count = func.upvalue_count;

        let constant = self.make_constant(Value::Obj(Rc::new(func)));
        self.emit_bytes(chunk::OpCode::Closure.u8(), constant);

        for i in 0..upvalue_count {
            let upvalue = upvalues[i as usize].clone();
            self.emit_bytes(upvalue.is_local as u8, upvalue.index);
        }
    }

    fn call(&mut self, _can_assign: bool) {
        let arg_count = self.argument_list();
        self.emit_bytes(chunk::OpCode::Call.u8(), arg_count);
    }

    fn argument_list(&mut self) -> u8 {
        let mut arg_count = 0;
        if !self.check(TokenKind::RightParen) {
            loop {
                self.expression();
                if arg_count == 255 {
                    self.error("Can't have more than 255 arguments.");
                    return 0;
                }
                arg_count += 1;
                if !self.match_token(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.consume(TokenKind::RightParen, "Expect ')' after arguments.");
        arg_count
    }

    fn return_statement(&mut self) {
        if let FunctionType::Script = self.comp_unit.function_type {
            self.error("Cannot return from top-level code.");
        }

        if self.match_token(TokenKind::Semicolon) {
            self.emit_return();
        } else {
            if self.comp_unit.function_type == FunctionType::Initializer {
                self.error("Can't return a value from an initializer.");
            }

            self.expression();
            self.consume(TokenKind::Semicolon, "Expect ';' after return value.");
            self.emit_byte(chunk::OpCode::Return.u8());
        }
    }

    fn class_declaration(&mut self) {
        self.consume(TokenKind::Identifier, "Expect class name.");
        let class_name = self.previous.lexeme.clone();
        let name_constant = self.identifier_constant(class_name.clone());
        self.declare_variable();

        self.emit_bytes(chunk::OpCode::Class.u8(), name_constant);
        self.define_variable(name_constant);

        let class_compiler = ClassCompiler {
            enclosing: Box::new(self.class_compiler.clone()),
            has_superclass: false,
        };
        self.class_compiler = Some(class_compiler);

        if self.match_token(TokenKind::Less) {
            self.consume(TokenKind::Identifier, "Expect superclass name.");
            self.variable(false);

            if class_name == self.previous.lexeme {
                return self.error("A class can't inherit from itself.");
            }

            self.begin_scope();
            self.add_local(synthetic_token("super"));
            self.define_variable(0);

            self.named_variable(class_name.clone(), false);
            self.emit_byte(chunk::OpCode::Inherit.u8());
            self.class_compiler.as_mut().unwrap().has_superclass = true;
        }

        self.named_variable(class_name, false);

        self.consume(TokenKind::LeftBrace, "Expect '{' before class body.");
        while !self.check(TokenKind::RightBrace) && !self.check(TokenKind::Eof) {
            self.method();
        }
        self.consume(TokenKind::RightBrace, "Expect '}' after class body.");
        self.emit_byte(chunk::OpCode::Pop.u8());

        if self.class_compiler.as_ref().unwrap().has_superclass {
            self.end_scope();
        }

        // TODO: try to simplify this
        let enclosing = self.class_compiler.as_ref().unwrap().enclosing.as_ref();
        self.class_compiler = enclosing.clone();
    }

    fn dot(&mut self, can_assign: bool) {
        self.consume(TokenKind::Identifier, "Expect property name after '.'.");
        let name = self.identifier_constant(self.previous.lexeme.clone());

        if can_assign && self.match_token(TokenKind::Equal) {
            self.expression();
            self.emit_bytes(chunk::OpCode::SetProperty.u8(), name);
        } else if self.match_token(TokenKind::LeftParen) {
            let arg_count = self.argument_list();
            self.emit_bytes(chunk::OpCode::Invoke.u8(), name);
            self.emit_byte(arg_count);
        } else {
            self.emit_bytes(chunk::OpCode::GetProperty.u8(), name);
        }
    }

    fn method(&mut self) {
        self.consume(TokenKind::Identifier, "Expect method name.");
        let name_constant = self.identifier_constant(self.previous.lexeme.clone());
        let mut func_type = FunctionType::Method;
        if self.previous.lexeme == "init" {
            func_type = FunctionType::Initializer;
        }
        self.function(func_type);
        self.emit_bytes(chunk::OpCode::Method.u8(), name_constant);
    }

    fn this(&mut self, _can_assign: bool) {
        if self.class_compiler.is_none() {
            self.error("Cannot use 'this' outside of a class.");
            return;
        }

        self.variable(false)
    }

    fn super_(&mut self, _can_assign: bool) {
        if self.class_compiler.is_none() {
            return self.error("Cannot use 'super' outside of a class.");
        } else if !self.class_compiler.as_ref().unwrap().has_superclass {
            return self.error("Cannot use 'super' in a class with no superclass.");
        }

        self.consume(TokenKind::Dot, "Expect '.' after 'super'.");
        self.consume(TokenKind::Identifier, "Expect superclass method name.");
        let name = self.identifier_constant(self.previous.lexeme.clone());

        self.named_variable("this".to_string(), false);

        if self.match_token(TokenKind::LeftParen) {
            let arg_count = self.argument_list();
            self.named_variable("super".to_string(), false);
            self.emit_bytes(chunk::OpCode::SuperInvoke.u8(), name);
            self.emit_byte(arg_count);
        } else {
            self.named_variable("super".to_string(), false);
            self.emit_bytes(chunk::OpCode::GetSuper.u8(), name);
        }
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
}

fn synthetic_token(lexeme: &str) -> scanner::Token {
    scanner::Token {
        kind: scanner::TokenKind::Identifier,
        lexeme: lexeme.to_string(),
        line: 0,
    }
}
