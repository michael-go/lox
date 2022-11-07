use std::collections::HashMap;
use std::fmt::Formatter;

use num_traits::FromPrimitive;

use anyhow::Result;

use crate::chunk::*;
use crate::compiler;
use crate::value::*;

pub struct Options {
    pub trace_execution: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            trace_execution: false,
        }
    }
}

pub struct VM {
    options: Options,
    chunk: Chunk,
    ip: usize,
    stack: Vec<Value>,
    globals: HashMap<String, Value>,
}

// TODO: move to common module, add CompilerError
#[derive(Debug, PartialEq)]
pub enum LoxErrorKind {
    RuntimeError,
}

#[derive(Debug, PartialEq)]
pub struct LoxError {
    #[allow(dead_code)]
    pub kind: LoxErrorKind,
    #[allow(dead_code)]
    pub message: String,
}

impl std::fmt::Display for LoxError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for LoxError {}

impl LoxError {
    pub fn new(kind: LoxErrorKind, message: &str) -> LoxError {
        LoxError {
            kind,
            message: message.to_string(),
        }
    }
}

impl VM {
    pub fn new(options: Options) -> VM {
        VM {
            options: options,
            chunk: Chunk::new(),
            ip: 0,
            // TODO: reserve space for the stack, maybe have STACK_MAX
            stack: Vec::new(),
            globals: HashMap::new(),
        }
    }

    fn init(&mut self, chunk: Chunk) {
        self.chunk = chunk;
        self.ip = 0;
        self.stack.clear();
    }

    pub fn interpret(&mut self, source: &str) -> Result<()> {
        let mut chunk = Chunk::new();
        compiler::Compiler::new(source, &mut chunk).compile()?;
        if self.options.trace_execution {
            chunk.dissasemble("code");
        }
        self.init(chunk);
        self.run()
    }

    fn run(&mut self) -> Result<()> {
        loop {
            if self.options.trace_execution {
                println!("          ");
                for slot in &self.stack {
                    print!("[ {} ]", slot);
                }
                println!("");

                self.chunk.disassemble_instruction(self.ip);
            }

            let instruction = self.read_byte();
            match FromPrimitive::from_u8(instruction) {
                Some(OpCode::Constant) => {
                    let constant = self.read_constant();
                    self.push(constant);
                }
                Some(OpCode::Nil) => self.push(Value::Nil),
                Some(OpCode::True) => self.push(Value::Bool(true)),
                Some(OpCode::False) => self.push(Value::Bool(false)),
                Some(OpCode::Pop) => {
                    self.pop();
                }
                Some(OpCode::GetLocal) => {
                    let slot = self.read_byte();
                    self.push(self.stack[slot as usize].clone());
                }
                Some(OpCode::SetLocal) => {
                    let slot = self.read_byte();
                    self.stack[slot as usize] = self.peek(0).clone();
                }
                Some(OpCode::GetGlobal) => {
                    let name_obj = self.read_constant();
                    if let Value::Obj(Obj::String(name)) = name_obj {
                        let value = self.globals.get(&name).unwrap_or(&Value::Nil);
                        self.push(value.clone());
                    } else {
                        return Err(self
                            .runtime_error("internal error: expected variable name")
                            .into());
                    }
                }
                Some(OpCode::DefineGlobal) => {
                    let name = self.read_constant();
                    if let Value::Obj(Obj::String(name)) = name {
                        let val = self.peek(0);
                        self.globals.insert(name, val);
                        self.pop();
                    } else {
                        return Err(self
                            .runtime_error("internal error: expected variable name")
                            .into());
                    }
                }
                Some(OpCode::SetGlobal) => {
                    let name = self.read_constant();
                    if let Value::Obj(Obj::String(name)) = name {
                        let val = self.peek(0);

                        if self.globals.contains_key(&name) {
                            self.globals.insert(name, val);
                        } else {
                            return Err(self
                                .runtime_error(&format!("Undefined variable {}", name))
                                .into());
                        }
                    } else {
                        return Err(self
                            .runtime_error("internal error: expected variable name")
                            .into());
                    }
                }
                Some(OpCode::Equal) => {
                    let b = self.pop();
                    let a = self.pop();
                    self.push(Value::Bool(a == b));
                }
                Some(OpCode::Greater) => {
                    self.binary_op_compare(|a, b| a > b)?;
                }
                Some(OpCode::Less) => {
                    self.binary_op_compare(|a, b| a < b)?;
                }
                Some(OpCode::Add) => {
                    let b = self.pop();
                    let a = self.pop();
                    match (a, b) {
                        (Value::Number(a), Value::Number(b)) => self.push(Value::Number(a + b)),
                        (Value::Obj(Obj::String(a)), Value::Obj(Obj::String(b))) => {
                            let new_str = a + &b;
                            self.push(Value::Obj(Obj::String(new_str)))
                        }
                        _ => {
                            return Err(self
                                .runtime_error("Operands must be two numbers or two strings.")
                                .into());
                        }
                    }
                }
                Some(OpCode::Subtract) => {
                    self.binary_op_num(|a, b| a - b)?;
                }
                Some(OpCode::Multiply) => {
                    self.binary_op_num(|a, b| a * b)?;
                }
                Some(OpCode::Divide) => {
                    self.binary_op_num(|a, b| a / b)?;
                }
                Some(OpCode::Negate) => {
                    let value = self.pop();
                    match value {
                        Value::Number(v) => self.push(Value::Number(v * -1.0)),
                        _ => return Err(self.runtime_error("Operand must be a number.").into()),
                    }
                }
                Some(OpCode::Not) => {
                    let value = self.pop();
                    match value {
                        Value::Bool(v) => self.push(Value::Bool(!v)),
                        Value::Nil => self.push(Value::Bool(true)),
                        _ => self.push(Value::Bool(false)),
                    }
                }
                Some(OpCode::Print) => {
                    let value = self.pop();
                    println!("{}", value);
                }
                Some(OpCode::Jump) => {
                    let offset = self.read_short();
                    self.ip += offset as usize;
                }
                Some(OpCode::JumpIfFalse) => {
                    let offset = self.read_short();
                    if Self::is_falsey(self.peek(0)) {
                        self.ip += offset as usize;
                    }
                }
                Some(OpCode::Return) => {
                    return Ok(());
                }
                None => {
                    return Err(self.runtime_error("Unknown opcode.").into());
                }
            }
        }
    }

    fn read_byte(&mut self) -> u8 {
        let byte = self.chunk.code[self.ip];
        self.ip += 1;
        byte
    }

    fn read_constant(&mut self) -> Value {
        let constant_index = self.read_byte();
        // TODO: try to avoid the clone
        self.chunk.constants[constant_index as usize].clone()
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn pop(&mut self) -> Value {
        self.stack.pop().unwrap()
    }

    fn peek(&self, distance: usize) -> Value {
        self.stack[self.stack.len() - 1 - distance].clone()
    }

    fn reset_stack(&mut self) {
        self.stack.clear();
    }

    fn binary_op_num(&mut self, op: fn(f64, f64) -> f64) -> Result<()> {
        let b = self.pop();
        let a = self.pop();

        match (a, b) {
            (Value::Number(a), Value::Number(b)) => {
                self.push(Value::Number(op(a, b)));
                Ok(())
            }
            _ => return Err(self.runtime_error("Operands must be numbers.").into()),
        }
    }

    fn binary_op_compare(&mut self, op: fn(f64, f64) -> bool) -> Result<()> {
        let b = self.pop();
        let a = self.pop();

        match (a, b) {
            (Value::Number(a), Value::Number(b)) => {
                self.push(Value::Bool(op(a, b)));
                Ok(())
            }
            _ => return Err(self.runtime_error("Operands must be numbers.").into()),
        }
    }

    fn runtime_error(&mut self, message: &str) -> LoxError {
        eprintln!("Runtime error: {}", message);

        let line = self.chunk.lines[&(self.ip - 1)];
        eprintln!("[line {}] in script", line);
        self.reset_stack();

        return LoxError::new(LoxErrorKind::RuntimeError, message).into();
    }

    fn is_falsey(val: Value) -> bool {
        match val {
            Value::Nil => true,
            Value::Bool(false) => true,
            _ => false,
        }
    }

    fn read_short(&mut self) -> u16 {
        let offset = (self.read_byte() as u16) << 8;
        offset | self.read_byte() as u16
    }
}
