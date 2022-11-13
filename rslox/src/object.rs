use crate::chunk::Chunk;
use crate::value::Value;

#[derive(Clone, PartialEq)]
pub struct Function {
    pub arity: usize,
    pub chunk: Chunk,
    pub name: String, // TODO: this should be an Obj::String?
}

impl<'a> std::fmt::Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.name.is_empty() {
            write!(f, "<script>")
        } else {
            write!(f, "<fn {}>", self.name)
        }
    }
}

impl<'a> Function {
    pub fn new(name: Option<String>) -> Function {
        let func_name: String;
        if let Some(name) = name {
            func_name = name;
        } else {
            func_name = String::new();
        }

        Function {
            arity: 0,
            chunk: Chunk::new(),
            name: func_name,
        }
    }
}

#[derive(Clone)]
pub struct NativeFunction {
    pub function: fn(&[Value]) -> Value,
}

impl std::fmt::Display for NativeFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<native fn>")
    }
}

impl PartialEq for NativeFunction {
    fn eq(&self, other: &Self) -> bool {
        self.function as usize == other.function as usize
    }
}

impl NativeFunction {
    pub fn new(function: fn(&[Value]) -> Value) -> NativeFunction {
        NativeFunction { function }
    }
}

#[derive(Clone, PartialEq)]
pub enum Obj {
    String(String),
    Function(Function),
    NativeFunction(NativeFunction),
}

impl<'a> std::fmt::Display for Obj {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Obj::String(s) => write!(fmt, "\"{}\"", s),
            Obj::Function(f) => write!(fmt, "{}", f),
            Obj::NativeFunction(f) => write!(fmt, "{}", f),
        }
    }
}
