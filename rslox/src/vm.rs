use std::collections::HashMap;
use std::fmt::Formatter;
use std::rc::Rc;

use arrayvec::ArrayVec;
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
    function: Rc<Function>,
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

const FRAMES_MAX: usize = u8::MAX as usize;
const STACK_MAX: usize = u8::MAX as usize;

struct RunCtx {
    frames: ArrayVec<CallFrame, FRAMES_MAX>,
    stack: ArrayVec<Value, STACK_MAX>,
}

impl RunCtx {
    pub fn new() -> RunCtx {
        RunCtx {
            frames: ArrayVec::new(),
            stack: ArrayVec::new(),
        }
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn pop(&mut self) -> Value {
        self.stack.pop().unwrap()
    }

    fn peek(&self, distance: usize) -> &Value {
        &self.stack[self.stack.len() - 1 - distance]
    }

    fn reset_stack(&mut self) {
        self.stack.clear();
    }

    fn current_frame(&self) -> &CallFrame {
        self.frames.last().unwrap()
    }

    fn read_byte(&mut self) -> u8 {
        let byte = self.current_frame().function.chunk.code[self.current_frame().ip];
        self.frames.last_mut().unwrap().ip += 1;
        byte
    }

    fn read_short(&mut self) -> u16 {
        let offset = (self.read_byte() as u16) << 8;
        offset | self.read_byte() as u16
    }

    fn read_constant(&mut self) -> &Value {
        let constant_index = self.read_byte();
        &self.current_frame().function.chunk.constants[constant_index as usize]
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

    fn call(&mut self, function: Rc<Function>, arg_count: u8) -> Result<()> {
        if self.frames.len() >= self.frames.capacity() {
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

    fn call_value(&mut self, callee: &Value, arg_count: u8) -> Result<()> {
        match callee {
            Value::Obj(obj) => {
                if let Ok(function) = obj.clone().downcast_rc::<Function>() {
                    if arg_count as usize != function.arity {
                        return Err(self
                            .runtime_error(&format!(
                                "Expected {} arguments but got {}.",
                                function.arity, arg_count
                            ))
                            .into());
                    }
                    self.call(function, arg_count)
                } else if let Some(native) = obj.downcast_ref::<NativeFunction>() {
                    let args = &self.stack[self.stack.len() - arg_count as usize..];
                    let result = (native.function)(args);
                    self.stack
                        .truncate(self.stack.len() - arg_count as usize - 1);
                    self.push(result);
                    Ok(())
                } else {
                    return Err(self
                        .runtime_error("Can only call functions and classes.")
                        .into());
                }
            }

            _ => {
                return Err(self
                    .runtime_error("Can only call functions and classes.")
                    .into())
            }
        }
    }
}

pub struct VM {
    options: Options,

    globals: HashMap<String, Value>,
}

impl VM {
    pub fn new(options: Options) -> VM {
        let mut vm = VM {
            options: options,
            globals: HashMap::new(),
        };

        vm.define_native("clock", Self::clock_native);
        vm
    }

    pub fn interpret(&mut self, source: &str) -> Result<()> {
        let mut compiler = compiler::Compiler::new(source);
        let function = compiler.compile()?;
        if self.options.trace_execution {
            function.chunk.dissasemble("code");
        }

        let mut run_ctx = RunCtx::new();

        let func_rc = Rc::new(function);

        run_ctx.push(Value::Obj(func_rc.clone()));
        run_ctx.call(func_rc.clone(), 0)?;

        self.run(&mut run_ctx)
    }

    fn run(&mut self, ctx: &mut RunCtx) -> Result<()> {
        loop {
            if self.options.trace_execution {
                println!("          ");
                for slot in &ctx.stack {
                    print!("[ {} ]", slot);
                }
                println!("");

                ctx.frames
                    .last()
                    .unwrap()
                    .function
                    .chunk
                    .disassemble_instruction(ctx.frames.last().unwrap().ip);
            }

            let instruction = ctx.read_byte();
            match FromPrimitive::from_u8(instruction) {
                Some(OpCode::Constant) => {
                    let constant = ctx.read_constant().clone();
                    ctx.push(constant);
                }
                Some(OpCode::Nil) => ctx.push(Value::Nil),
                Some(OpCode::True) => ctx.push(Value::Bool(true)),
                Some(OpCode::False) => ctx.push(Value::Bool(false)),
                Some(OpCode::Pop) => {
                    ctx.pop();
                }
                Some(OpCode::GetLocal) => {
                    let slot = ctx.read_byte();
                    ctx.push(
                        ctx.stack[ctx.frames.last().unwrap().slots_base + slot as usize].clone(),
                    )
                }
                Some(OpCode::SetLocal) => {
                    let slot = ctx.read_byte();
                    ctx.stack[ctx.frames.last().unwrap().slots_base + slot as usize] =
                        ctx.peek(0).clone();
                }
                Some(OpCode::GetGlobal) => {
                    let name_obj = ctx.read_constant();
                    if let Value::Obj(obj) = name_obj {
                        if let Some(name) = obj.downcast_ref::<String>() {
                            let value = self.globals.get(name).unwrap_or(&Value::Nil);
                            ctx.push(value.clone());
                        } else {
                            return Err(ctx
                                .runtime_error("internal error: expected variable name")
                                .into());
                        }
                    } else {
                        return Err(ctx
                            .runtime_error("internal error: expected variable name")
                            .into());
                    }
                }
                Some(OpCode::DefineGlobal) => {
                    let name = ctx.read_constant();
                    if let Value::Obj(obj) = name {
                        if let Some(name) = obj.downcast_ref::<String>() {
                            self.globals.insert(name.clone(), ctx.peek(0).clone());
                            ctx.pop();
                        } else {
                            return Err(ctx
                                .runtime_error("internal error: expected variable name")
                                .into());
                        }
                    } else {
                        return Err(ctx
                            .runtime_error("internal error: expected variable name")
                            .into());
                    }
                }
                Some(OpCode::SetGlobal) => {
                    let name = ctx.read_constant().clone();

                    if let Value::Obj(obj) = name {
                        if let Some(name) = obj.downcast_ref::<String>() {
                            if self.globals.contains_key(name) {
                                self.globals.insert(name.clone(), ctx.peek(0).clone());
                            } else {
                                return Err(ctx
                                    .runtime_error(&format!("Undefined variable '{}'.", name))
                                    .into());
                            }
                        } else {
                            return Err(ctx
                                .runtime_error("internal error: expected variable name")
                                .into());
                        }
                    } else {
                        return Err(ctx
                            .runtime_error("internal error: expected variable name")
                            .into());
                    }
                }
                Some(OpCode::Equal) => {
                    let b = ctx.pop();
                    let a = ctx.pop();
                    ctx.push(Value::Bool(a == b));
                }
                Some(OpCode::Greater) => {
                    ctx.binary_op_compare(|a, b| a > b)?;
                }
                Some(OpCode::Less) => {
                    ctx.binary_op_compare(|a, b| a < b)?;
                }
                Some(OpCode::Add) => {
                    let b = ctx.pop();
                    let a = ctx.pop();
                    match (a, b) {
                        (Value::Number(a), Value::Number(b)) => ctx.push(Value::Number(a + b)),
                        (Value::Obj(a), Value::Obj(b)) => {
                            match (a.downcast_ref::<String>(), b.downcast_ref::<String>()) {
                                (Some(a), Some(b)) => {
                                    let new_str = a.clone() + b;
                                    ctx.push(Value::Obj(Rc::new(new_str)));
                                }
                                _ => {
                                    return Err(ctx
                                        .runtime_error(
                                            "Operands must be two numbers or two strings.",
                                        )
                                        .into())
                                }
                            }
                        }
                        _ => {
                            return Err(ctx
                                .runtime_error("Operands must be two numbers or two strings.")
                                .into());
                        }
                    }
                }
                Some(OpCode::Subtract) => {
                    ctx.binary_op_num(|a, b| a - b)?;
                }
                Some(OpCode::Multiply) => {
                    ctx.binary_op_num(|a, b| a * b)?;
                }
                Some(OpCode::Divide) => {
                    ctx.binary_op_num(|a, b| a / b)?;
                }
                Some(OpCode::Negate) => {
                    let value = ctx.pop();
                    match value {
                        Value::Number(v) => ctx.push(Value::Number(v * -1.0)),
                        _ => return Err(ctx.runtime_error("Operand must be a number.").into()),
                    }
                }
                Some(OpCode::Not) => {
                    let value = ctx.pop();
                    match value {
                        Value::Bool(v) => ctx.push(Value::Bool(!v)),
                        Value::Nil => ctx.push(Value::Bool(true)),
                        _ => ctx.push(Value::Bool(false)),
                    }
                }
                Some(OpCode::Print) => {
                    let value = ctx.pop();
                    println!("{}", value);
                }
                Some(OpCode::Jump) => {
                    let offset = ctx.read_short();
                    ctx.frames.last_mut().unwrap().ip += offset as usize;
                }
                Some(OpCode::JumpIfFalse) => {
                    let offset = ctx.read_short();
                    if Self::is_falsey(&ctx.peek(0)) {
                        ctx.frames.last_mut().unwrap().ip += offset as usize;
                    }
                }
                Some(OpCode::Loop) => {
                    let offset = ctx.read_short();
                    ctx.frames.last_mut().unwrap().ip -= offset as usize;
                }
                Some(OpCode::Call) => {
                    let arg_count = ctx.read_byte();
                    let val = ctx.peek(arg_count as usize).clone();
                    ctx.call_value(&val, arg_count)?;
                }
                Some(OpCode::Return) => {
                    let result = ctx.pop();
                    let current_frame_base = ctx.frames.last().unwrap().slots_base;
                    ctx.frames.pop();
                    if ctx.frames.is_empty() {
                        return Ok(());
                    }

                    ctx.stack.truncate(current_frame_base);
                    ctx.push(result);
                }
                None => {
                    return Err(ctx.runtime_error("Unknown opcode.").into());
                }
            }
        }
    }

    fn is_falsey(val: &Value) -> bool {
        match val {
            Value::Nil => true,
            Value::Bool(false) => true,
            _ => false,
        }
    }

    fn define_native(&mut self, name: &str, function: fn(&[Value]) -> Value) {
        // TODO: in the book key & value pushed/popped to the stack to protect from GC
        self.globals.insert(
            name.to_string(),
            Value::Obj(Rc::new(NativeFunction::new(function))),
        );
    }

    fn clock_native(_args: &[Value]) -> Value {
        let now = std::time::SystemTime::now();
        let since_the_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
        Value::Number(since_the_epoch.as_secs_f64())
    }
}
