use num_traits::FromPrimitive;

use crate::chunk::*;
use crate::value::*;

pub struct VM {
    chunk: Chunk,
    ip: usize,
    stack: Vec<Value>,
}

#[derive(Debug)]
pub enum LoxErrorKind {
    RuntimeError,
}

#[derive(Debug)]
pub struct LoxError {
    #[allow(dead_code)]
    kind: LoxErrorKind,
    #[allow(dead_code)]
    message: String,
}

impl LoxError {
    pub fn new(kind: LoxErrorKind, message: String) -> LoxError {
        LoxError { kind, message }
    }
}

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

impl VM {
    pub fn new(chunk: Chunk) -> VM {
        VM {
            chunk,
            ip: 0,
            // TODO: reserve space for the stack, maybe have STACK_MAX
            stack: Vec::new(),
        }
    }

    pub fn interpret(&mut self, options: Options) -> Result<(), LoxError> {
        self.ip = 0;
        self.run(options)
    }

    fn run(&mut self, options: Options) -> Result<(), LoxError> {
        loop {
            if options.trace_execution {
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
                Some(OpCode::Negate) => {
                    let value = self.pop();
                    self.push(-value);
                }
                Some(OpCode::Return) => {
                    VM::print_value(self.pop());
                    return Ok(());
                }
                None => {
                    println!("Unknown opcode {}", instruction);
                    return Err(LoxError::new(
                        LoxErrorKind::RuntimeError,
                        "Unknown opcode".to_string(),
                    ));
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
}
