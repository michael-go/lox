use hashbrown::HashMap;
use std::cell::RefCell;
use std::collections::LinkedList;
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
    closure: Rc<Closure>,
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
    open_upvalues: LinkedList<Rc<RefCell<Upvalue>>>,
    init_string: ObjString,
}

impl RunCtx {
    pub fn new() -> RunCtx {
        RunCtx {
            frames: ArrayVec::new(),
            stack: ArrayVec::new(),
            open_upvalues: LinkedList::new(),
            init_string: ObjString::new("init".to_string()),
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
        let byte = self.current_frame().closure.function.chunk.code[self.current_frame().ip];
        self.frames.last_mut().unwrap().ip += 1;
        byte
    }

    fn read_short(&mut self) -> u16 {
        let offset = (self.read_byte() as u16) << 8;
        offset | self.read_byte() as u16
    }

    fn read_constant(&mut self) -> &Value {
        let constant_index = self.read_byte();
        &self.current_frame().closure.function.chunk.constants[constant_index as usize]
    }

    fn runtime_error(&mut self, message: &str) -> LoxError {
        eprintln!("{}", message);

        for frame in self.frames.iter().rev() {
            let line = frame.closure.function.chunk.lines[&(frame.ip - 1)];
            eprintln!("[line {}] in {}", line, frame.closure.function);
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

    fn call(&mut self, closure: Rc<Closure>, arg_count: u8) -> Result<()> {
        if self.frames.len() >= self.frames.capacity() {
            return Err(self.runtime_error("Stack overflow.").into());
        }
        if arg_count as usize != closure.function.arity {
            return Err(self
                .runtime_error(&format!(
                    "Expected {} arguments but got {}.",
                    closure.function.arity, arg_count
                ))
                .into());
        }

        let frame = CallFrame {
            closure,
            ip: 0,
            slots_base: self.stack.len() - arg_count as usize - 1,
        };
        self.frames.push(frame);
        Ok(())
    }

    fn call_value(&mut self, callee: &Value, arg_count: u8) -> Result<()> {
        match callee {
            Value::Obj(obj) => {
                if let Ok(bound_method) = obj.clone().downcast_rc::<BoundMethod>() {
                    let this_offset = self.stack.len() - arg_count as usize - 1;
                    self.stack[this_offset] = bound_method.receiver.clone();
                    return self.call(bound_method.method.clone(), arg_count);
                } else if let Ok(class) = obj.clone().downcast_rc::<Class>() {
                    let offset = self.stack.len() - arg_count as usize - 1;
                    self.stack[offset] = Value::Obj(Rc::new(Instance::new(class.clone())));
                    if let Some(initializer) = class.methods.borrow().get(&self.init_string) {
                        self.call(initializer.clone(), arg_count)?;
                    } else if arg_count > 0 {
                        return Err(self
                            .runtime_error(&format!("Expected 0 arguments but got {}.", arg_count))
                            .into());
                    }
                    Ok(())
                } else if let Ok(closure) = obj.clone().downcast_rc::<Closure>() {
                    self.call(closure, arg_count)
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

    fn capture_upvalue(&mut self, stack_offset: usize) -> Rc<RefCell<Upvalue>> {
        let mut cursor = self.open_upvalues.cursor_front_mut();
        let mut next = cursor.current();

        while let Some(upvalue) = next {
            if upvalue.as_ref().borrow().location == stack_offset {
                return upvalue.clone();
            }

            if upvalue.as_ref().borrow().location < stack_offset {
                break;
            }

            next = cursor.peek_next();
        }

        let created_upvalue = Rc::new(RefCell::new(Upvalue {
            location: stack_offset,
            closed: None,
        }));

        cursor.insert_after(created_upvalue.clone());
        created_upvalue
    }

    fn close_upvalues(&mut self, last_location: usize) -> Result<()> {
        let mut cursor = self.open_upvalues.cursor_front_mut();

        while let Some(upvalue) = cursor.current() {
            if upvalue.as_ref().borrow().location < last_location {
                break;
            }

            let value = self.stack[upvalue.as_ref().borrow().location].clone();
            upvalue.borrow_mut().closed = Some(value);
            cursor.remove_current();
        }
        Ok(())
    }

    fn define_method(&mut self, name: &ObjString) -> Result<()> {
        if let Value::Obj(obj) = self.peek(1) {
            //TODO: the else brances should be unreachable as we trust the compiler
            if let Ok(class) = obj.clone().downcast_rc::<Class>() {
                let method = self.peek(0);
                if let Value::Obj(obj) = method {
                    if let Ok(closure) = obj.clone().downcast_rc::<Closure>() {
                        class.methods.borrow_mut().insert(name.clone(), closure);
                        self.pop();
                    } else {
                        return Err(self
                            .runtime_error("internal error: Expected a function.")
                            .into());
                    }
                } else {
                    return Err(self
                        .runtime_error("internal error: Expected a function.")
                        .into());
                }
            } else {
                return Err(self
                    .runtime_error("internal error: Expected a class.")
                    .into());
            }
        } else {
            return Err(self.runtime_error("Only classes have methods.").into());
        }
        Ok(())
    }

    fn bind_method(&mut self, class: &Rc<Class>, name: &ObjString) -> Result<()> {
        if let Some(method) = class.methods.borrow().get(name) {
            let bound_method = Value::Obj(Rc::new(BoundMethod::new(
                self.peek(0).clone(),
                method.clone(),
            )));
            self.pop();
            self.push(bound_method);
            Ok(())
        } else {
            Err(self
                .runtime_error(&format!("Undefined property '{}'.", name))
                .into())
        }
    }

    fn invoke(&mut self, name: &ObjString, arg_count: u8) -> Result<()> {
        if let Value::Obj(reciever) = self.peek(arg_count as usize) {
            if let Ok(instance) = reciever.clone().downcast_rc::<Instance>() {
                if let Some(field) = instance.fields.borrow().get(name) {
                    // expecting a callable field here
                    let stack_offset = self.stack.len() - arg_count as usize - 1;
                    self.stack[stack_offset] = field.clone();
                    self.call_value(field, arg_count)
                } else {
                    self.invoke_from_class(&instance.class, name, arg_count)
                }
            } else {
                Err(self
                    .runtime_error("Only instances have methods/properties.")
                    .into())
            }
        } else {
            Err(self
                .runtime_error("Only instances have methods/properties.")
                .into())
        }
    }

    fn invoke_from_class(&mut self, class: &Class, name: &ObjString, arg_count: u8) -> Result<()> {
        if let Some(method) = class.methods.borrow().get(name) {
            self.call(method.clone(), arg_count)
        } else {
            Err(self
                .runtime_error(&format!("Undefined property '{}'.", name))
                .into())
        }
    }
}

pub struct VM {
    options: Options,

    globals: HashMap<ObjString, Value>,
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
        let function = compiler::compile(source)?;
        if self.options.trace_execution {
            function.chunk.dissasemble("code");
        }

        let mut run_ctx = RunCtx::new();

        let closure = Rc::new(Closure::new(function));
        run_ctx.push(Value::Obj(closure.clone()));
        run_ctx.call(closure, 0)?;

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
                    .closure
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
                    let name = Self::as_objstring(ctx.read_constant()).unwrap();
                    let value = self.globals.get(name).unwrap_or(&Value::Nil);
                    ctx.push(value.clone());
                }
                Some(OpCode::DefineGlobal) => {
                    let name = Self::as_objstring(ctx.read_constant()).unwrap();
                    self.globals.insert(name.clone(), ctx.peek(0).clone());
                    ctx.pop();
                }
                Some(OpCode::SetGlobal) => {
                    let name_obj = ctx.read_constant().clone();
                    let name = Self::as_objstring(&name_obj).unwrap();
                    if self.globals.contains_key(name) {
                        self.globals.insert(name.clone(), ctx.peek(0).clone());
                    } else {
                        return Err(ctx
                            .runtime_error(&format!("Undefined variable '{}'.", name))
                            .into());
                    }
                }
                Some(OpCode::GetProperty) => {
                    let name_obj = ctx.read_constant().clone();
                    let name = Self::as_objstring(&name_obj).unwrap();
                    if let Value::Obj(instance) = ctx.peek(0).clone() {
                        if let Some(instance) = instance.downcast_ref::<Instance>() {
                            if let Some(value) = instance.fields.borrow().get(name) {
                                ctx.pop();
                                ctx.push(value.clone());
                            } else {
                                ctx.bind_method(&instance.class, name)?;
                            }
                        } else {
                            return Err(ctx
                                .runtime_error("Only instances have properties.")
                                .into());
                        }
                    } else {
                        return Err(ctx.runtime_error("Only instances have properties.").into());
                    }
                }
                Some(OpCode::SetProperty) => {
                    let name_obj = ctx.read_constant().clone();
                    let name = Self::as_objstring(&name_obj).unwrap();
                    let value = ctx.pop();
                    if let Value::Obj(obj) = ctx.pop() {
                        if let Some(instance) = obj.downcast_ref::<Instance>() {
                            instance
                                .fields
                                .borrow_mut()
                                .insert(name.clone(), value.clone());
                            ctx.push(value);
                        } else {
                            return Err(ctx.runtime_error("Only instances have fields.").into());
                        }
                    } else {
                        return Err(ctx.runtime_error("Only instances have fields.").into());
                    }
                }
                Some(OpCode::GetUpvalue) => {
                    let slot = ctx.read_byte();
                    let upvalue =
                        ctx.frames.last().unwrap().closure.upvalues[slot as usize].clone();
                    if let Some(value) = upvalue.clone().as_ref().borrow().closed.clone() {
                        ctx.push(value.clone());
                    } else {
                        let location = upvalue.as_ref().borrow().location;
                        let value = ctx.stack[location].clone();
                        ctx.push(value);
                    }
                }
                Some(OpCode::SetUpvalue) => {
                    let slot = ctx.read_byte();
                    let value = ctx.peek(0).clone();
                    let upvalue =
                        ctx.frames.last().unwrap().closure.upvalues[slot as usize].clone();
                    if upvalue.as_ref().borrow().closed.is_some() {
                        upvalue.borrow_mut().closed = Some(value);
                    } else {
                        let location = upvalue.as_ref().borrow().location;
                        ctx.stack[location] = value
                    }
                }
                Some(OpCode::GetSuper) => {
                    let name_obj = ctx.read_constant().clone();
                    let name = Self::as_objstring(&name_obj).unwrap();

                    let superclass_value = ctx.pop();
                    let superclass = Self::as_class(&superclass_value).unwrap();

                    ctx.bind_method(&Rc::new(superclass.clone()), name)?;
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
                            match (a.downcast_ref::<ObjString>(), b.downcast_ref::<ObjString>()) {
                                (Some(a), Some(b)) => {
                                    let new_str = ObjString::new(a.clone().string + &b.string);
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
                Some(OpCode::Invoke) => {
                    let method_constant = ctx.read_constant().clone();
                    let method_name = Self::as_objstring(&method_constant).unwrap();
                    let arg_count = ctx.read_byte();
                    ctx.invoke(method_name, arg_count)?;
                }
                Some(OpCode::SuperInvoke) => {
                    let method_constant = ctx.read_constant().clone();
                    let method_name = Self::as_objstring(&method_constant).unwrap();
                    let arg_count = ctx.read_byte();
                    let superclass_value = ctx.pop();
                    let superclass = Self::as_class(&superclass_value).unwrap();
                    ctx.invoke_from_class(superclass, method_name, arg_count)?;
                }
                Some(OpCode::Closure) => {
                    let constant = ctx.read_constant();
                    if let Value::Obj(obj) = constant {
                        if let Some(function) = obj.downcast_ref::<Function>() {
                            let mut closure = Closure::new(function.clone());
                            for _ in 0..function.upvalue_count {
                                let is_local = ctx.read_byte();
                                let index = ctx.read_byte();
                                if is_local == 1 {
                                    let stack_offset =
                                        ctx.frames.last().unwrap().slots_base + index as usize;
                                    let upvalue = ctx.capture_upvalue(stack_offset);
                                    closure.upvalues.push(upvalue);
                                } else {
                                    let upvalue = ctx.frames.last().unwrap().closure.upvalues
                                        [index as usize]
                                        .clone();
                                    closure.upvalues.push(upvalue);
                                }
                            }
                            ctx.push(Value::Obj(Rc::new(closure)));
                        } else {
                            return Err(ctx
                                .runtime_error("internal error: expected function")
                                .into());
                        }
                    } else {
                        return Err(ctx
                            .runtime_error("internal error: expected function")
                            .into());
                    }
                }
                Some(OpCode::CloseUpvalue) => {
                    ctx.close_upvalues(ctx.stack.len() - 1)?;
                    ctx.pop();
                }
                Some(OpCode::Return) => {
                    let result = ctx.pop();
                    let current_frame_base = ctx.frames.last().unwrap().slots_base;

                    ctx.close_upvalues(current_frame_base)?;

                    ctx.frames.pop();
                    if ctx.frames.is_empty() {
                        return Ok(());
                    }

                    ctx.stack.truncate(current_frame_base);
                    ctx.push(result);
                }
                Some(OpCode::Class) => {
                    let name = Self::as_objstring(ctx.read_constant()).unwrap();
                    let class = Class::new(name.clone());
                    ctx.push(Value::Obj(Rc::new(class)));
                }
                Some(OpCode::Inherit) => {
                    if let Some(superclass) = Self::as_class(ctx.peek(1)) {
                        let subclass = Self::as_class(ctx.peek(0)).unwrap();
                        for (name, method) in superclass.methods.borrow().iter() {
                            subclass
                                .methods
                                .borrow_mut()
                                .insert(name.clone(), method.clone());
                        }
                        ctx.pop(); // Subclass.
                    } else {
                        return Err(ctx.runtime_error("Superclass must be a class.").into());
                    }
                }
                Some(OpCode::Method) => {
                    let name_constant = ctx.read_constant().clone();
                    let name = Self::as_objstring(&name_constant).unwrap();
                    ctx.define_method(name)?;
                }
                None => {
                    return Err(ctx.runtime_error("Unknown opcode.").into());
                }
            }
        }
    }

    fn as_objstring<'a>(value: &'a Value) -> Option<&'a ObjString> {
        if let Value::Obj(obj) = value {
            if let Some(obj_string) = obj.downcast_ref::<ObjString>() {
                Some(obj_string)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn as_class<'a>(value: &'a Value) -> Option<&'a Class> {
        if let Value::Obj(obj) = value {
            if let Some(class) = obj.downcast_ref::<Class>() {
                Some(class)
            } else {
                None
            }
        } else {
            None
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
            ObjString::new(name.to_string()),
            Value::Obj(Rc::new(NativeFunction::new(function))),
        );
    }

    fn clock_native(_args: &[Value]) -> Value {
        let now = std::time::SystemTime::now();
        let since_the_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
        Value::Number(since_the_epoch.as_secs_f64())
    }
}
