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
    chunk: Chunk,
    ip: usize,
    stack: Vec<Value>,
    options: Options,
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
        let mut chunk = Chunk::new();
        compiler::Compiler::new(source, &mut chunk).compile()?;
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
                Some(OpCode::Nil) => self.push(Value::Nil),
                Some(OpCode::True) => self.push(Value::Bool(true)),
                Some(OpCode::False) => self.push(Value::Bool(false)),
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
                            return Err(LoxError::new(
                                LoxErrorKind::RuntimeError,
                                "Operands must be two numbers or two strings.".to_string(),
                            )
                            .into())
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
                Some(OpCode::Return) => {
                    let value = self.pop();
                    println!("{}", value);
                    return Ok(value);
                }
                None => {
                    println!("Unknown opcode {}", instruction);
                    return Err(LoxError::new(
                        LoxErrorKind::RuntimeError,
                        "Unknown opcode".to_string(),
                    )
                    .into());
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

        return LoxError::new(LoxErrorKind::RuntimeError, message.to_string()).into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic() {
        let mut vm = VM::new(Options::default());
        let res = vm.interpret("3 * (1 + 2)").unwrap();
        if let Value::Number(n) = res {
            assert_eq!(n, 9.0);
        } else {
            panic!("Expected number");
        }
    }

    #[test]
    fn compare() {
        let mut vm = VM::new(Options::default());
        let res = vm.interpret("!(5 - 4 > 3 * 2 == !nil)").unwrap();
        if let Value::Bool(b) = res {
            assert_eq!(b, true);
        } else {
            panic!("Expected bool");
        }
    }

    #[test]
    fn unicode_comment() {
        let mut vm = VM::new(Options::default());
        let res = vm
            .interpret(
                "
        // סבבה
        1 / 2
        ",
            )
            .unwrap();
        if let Value::Number(n) = res {
            assert_eq!(n, 0.5);
        } else {
            panic!("Expected number");
        }
    }

    #[test]
    fn runtime_error() {
        let mut vm = VM::new(Options::default());
        let res = vm.interpret("1 + true");
        // TODO: try to downcast to LoxError
        assert_eq!(res.is_err(), true);
    }

    #[test]
    fn compare_strings() {
        let mut vm = VM::new(Options::default());
        let res = vm.interpret("\"abc\" == \"abc\"").unwrap();
        if let Value::Bool(b) = res {
            assert!(b);
        } else {
            panic!("Expected number");
        }
        let res = vm.interpret("\"abc\" == \"abd\"").unwrap();
        if let Value::Bool(b) = res {
            assert!(!b);
        } else {
            panic!("Expected number");
        }
    }

    #[test]
    fn string_concat() {
        let mut vm = VM::new(Options::default());
        let res = vm.interpret("\"foo\" + \"bar\" + \"baz\"").unwrap();
        if let Value::Obj(Obj::String(s)) = res {
            assert_eq!(s, "foobarbaz");
        } else {
            panic!("Expected string");
        }
    }
}
