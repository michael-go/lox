use std::collections::HashMap;
use std::fmt::Formatter;

use num_traits::FromPrimitive;

use anyhow::Result;

use crate::chunk::*;
use crate::compiler;
use crate::object::*;
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

struct CallFrame {
    function: Function,
    ip: usize,
    slots_base: usize,
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

pub struct VM {
    options: Options,

    // TODO: in clox these are allocated on the stack. consider https://crates.io/crates/arrayvec
    frames: Vec<CallFrame>,
    stack: Vec<Value>,

    globals: HashMap<String, Value>,
}

impl VM {
    pub fn new(options: Options) -> VM {
        VM {
            options: options,
            // TODO: reserve space for the stack, maybe have FRAMES_MAX
            frames: Vec::new(),
            // TODO: reserve space for the stack, maybe have STACK_MAX
            stack: Vec::new(),
            globals: HashMap::new(),
        }
    }

    fn init(&mut self) {
        self.stack.clear();
        self.frames.clear();

        self.define_native("clock", Self::clock_native);
    }

    pub fn interpret(&mut self, source: &str) -> Result<()> {
        // TODO: braindump: `globals`'s lifetime is tied to the lifetime of the VM,
        //   but the `stack` and `frames` are tied only to the lifetime of the `interpret` function.
        //   some of the Values in the stack/frames can reference globals (including function objects) from previous runs.
        // ... so maybe the globals should be the owners of the objects and the stack/frames should store references to them?

        let mut compiler = compiler::Compiler::new(source, compiler::FunctionType::Script);
        let function = compiler.compile()?;
        if self.options.trace_execution {
            function.chunk.dissasemble("code");
        }

        self.init();

        self.push(Value::Obj(Obj::Function(function.clone())));
        self.call(function, 0)?;

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

                self.frames
                    .last()
                    .unwrap()
                    .function
                    .chunk
                    .disassemble_instruction(self.frames.last().unwrap().ip);
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
                    self.push(
                        self.stack[self.frames.last().unwrap().slots_base + slot as usize].clone(),
                    )
                }
                Some(OpCode::SetLocal) => {
                    let slot = self.read_byte();
                    self.stack[self.frames.last().unwrap().slots_base + slot as usize] =
                        self.peek(0).clone();
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
                    self.frames.last_mut().unwrap().ip += offset as usize;
                }
                Some(OpCode::JumpIfFalse) => {
                    let offset = self.read_short();
                    if Self::is_falsey(self.peek(0)) {
                        self.frames.last_mut().unwrap().ip += offset as usize;
                    }
                }
                Some(OpCode::Loop) => {
                    let offset = self.read_short();
                    self.frames.last_mut().unwrap().ip -= offset as usize;
                }
                Some(OpCode::Call) => {
                    let arg_count = self.read_byte();
                    self.call_value(self.peek(arg_count as usize), arg_count)?;
                }
                Some(OpCode::Return) => {
                    let result = self.pop();
                    let current_frame_base = self.frames.last().unwrap().slots_base;
                    self.frames.pop();
                    if self.frames.is_empty() {
                        return Ok(());
                    }

                    self.stack.truncate(current_frame_base);
                    self.push(result);
                }
                None => {
                    return Err(self.runtime_error("Unknown opcode.").into());
                }
            }
        }
    }

    fn current_frame(&self) -> &CallFrame {
        self.frames.last().unwrap()
    }

    fn read_byte(&mut self) -> u8 {
        let byte = self.current_frame().function.chunk.code[self.current_frame().ip];
        self.frames.last_mut().unwrap().ip += 1;
        byte
    }

    fn read_constant(&mut self) -> Value {
        let constant_index = self.read_byte();
        // TODO: try to avoid the clone
        self.current_frame().function.chunk.constants[constant_index as usize].clone()
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

        for frame in self.frames.iter().rev() {
            let line = frame.function.chunk.lines[&(frame.ip - 1)];
            eprintln!("[line {}] in {}", line, frame.function);
        }

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

    fn call_value(&mut self, callee: Value, arg_count: u8) -> Result<()> {
        match callee {
            Value::Obj(Obj::Function(function)) => {
                if arg_count as usize != function.arity {
                    return Err(self
                        .runtime_error(&format!(
                            "Expected {} arguments but got {}.",
                            function.arity, arg_count
                        ))
                        .into());
                }
                self.call(function, arg_count)
            }
            Value::Obj(Obj::NativeFunction(native)) => {
                let args = &self.stack[self.stack.len() - arg_count as usize..];
                let result = (native.function)(args);
                self.stack
                    .truncate(self.stack.len() - arg_count as usize - 1);
                self.push(result);
                Ok(())
            }
            _ => {
                return Err(self
                    .runtime_error("Can only call functions and classes.")
                    .into())
            }
        }
    }

    fn call(&mut self, function: Function, arg_count: u8) -> Result<()> {
        if self.frames.len() > u8::MAX as usize {
            return Err(self.runtime_error("Stack overflow.").into());
        }

        let frame = CallFrame {
            function,
            ip: 0,
            slots_base: self.stack.len() - arg_count as usize - 1,
        };
        self.frames.push(frame);
        Ok(())
    }

    fn define_native(&mut self, name: &str, function: fn(&[Value]) -> Value) {
        // TODO: in the book key & value pushed/popped to the stack to protect from GC
        self.globals.insert(
            name.to_string(),
            Value::Obj(Obj::NativeFunction(NativeFunction::new(function))),
        );
    }

    fn clock_native(_args: &[Value]) -> Value {
        let now = std::time::SystemTime::now();
        let since_the_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
        Value::Number(since_the_epoch.as_secs_f64())
    }
}
