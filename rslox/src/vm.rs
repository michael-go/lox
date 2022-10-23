use std::fmt::Formatter;

use num_traits::FromPrimitive;

use anyhow::Result;

use crate::chunk::*;
use crate::value::*;
use crate::compiler;

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
    chunk: Chunk,
    ip: usize,
    stack: Vec<Value>,
    options: Options,
}

// TODO: move to common module, add CompilerError
#[derive(Debug)]
pub enum LoxErrorKind {
    RuntimeError,
}

#[derive(Debug)]
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
    pub fn new(kind: LoxErrorKind, message: String) -> LoxError {
        LoxError { kind, message }
    }
}

impl VM {
    pub fn new(options: Options) -> VM {
        VM {
            chunk: Chunk::new(),
            ip: 0,
            // TODO: reserve space for the stack, maybe have STACK_MAX
            stack: Vec::new(),
            options: options,
        }
    }
    
    fn init(&mut self, chunk: Chunk) {
        self.chunk = chunk;
        self.ip = 0;
        self.stack.clear();
    }

    pub fn interpret(&mut self, source: &str) -> Result<Value> {
        let mut compiler = compiler::Compiler::new();
        let chunk = compiler.compile(source)?;
        if self.options.trace_execution {
            chunk.dissasemble("code");
        }
        self.init(chunk);
        self.run()
    }

    fn run(&mut self) -> Result<Value> {
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
                Some(OpCode::Add) => {
                    self.binary_op(|a, b| a + b);
                }
                Some(OpCode::Subtract) => {
                    self.binary_op(|a, b| a - b);
                }
                Some(OpCode::Multiply) => {
                    self.binary_op(|a, b| a * b);
                }
                Some(OpCode::Divide) => {
                    self.binary_op(|a, b| a / b);
                }
                Some(OpCode::Negate) => {
                    let value = self.pop();
                    self.push(-value);
                }
                Some(OpCode::Return) => {
                    let value = self.pop();
                    VM::print_value(value);
                    return Ok(value);
                }
                None => {
                    println!("Unknown opcode {}", instruction);
                    return Err(LoxError::new(
                        LoxErrorKind::RuntimeError,
                        "Unknown opcode".to_string(),
                    ).into());
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
        self.chunk.constants[constant_index as usize]
    }

    fn print_value(value: Value) {
        println!("{}", value);
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn pop(&mut self) -> Value {
        self.stack.pop().unwrap()
    }

    fn binary_op(&mut self, op: fn(Value, Value) -> Value) {
        let b = self.pop();
        let a = self.pop();
        self.push(op(a, b));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpret() {
        let mut vm = VM::new(Options::default());
        let res = vm.interpret("3 * (1 + 2)").unwrap();
        assert_eq!(res, 9.0);
    }
}
