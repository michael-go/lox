use crate::chunk::Chunk;
use crate::value::Value;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use downcast_rs::{impl_downcast, Downcast};

#[derive(PartialEq)]
pub enum ObjType {
    String,
    Function,
    NativeFunction,
}

pub trait Obj: Downcast {
    fn obj_type(&self) -> ObjType;
}
impl_downcast!(Obj);

impl std::fmt::Display for dyn Obj {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.obj_type() {
            ObjType::String => write!(fmt, "\"{}\"", self.downcast_ref::<ObjString>().unwrap()),
            ObjType::Function => write!(fmt, "{}", self.downcast_ref::<Function>().unwrap()),
            ObjType::NativeFunction => {
                write!(fmt, "{}", self.downcast_ref::<NativeFunction>().unwrap())
            }
        }
    }
}

impl PartialEq for dyn Obj {
    fn eq(&self, other: &Self) -> bool {
        if self.obj_type() != other.obj_type() {
            return false;
        }

        match self.obj_type() {
            ObjType::String => {
                return self.downcast_ref::<ObjString>() == other.downcast_ref::<ObjString>();
            }
            ObjType::Function => {
                return self.downcast_ref::<Function>() == other.downcast_ref::<Function>();
            }
            ObjType::NativeFunction => {
                return self.downcast_ref::<NativeFunction>()
                    == other.downcast_ref::<NativeFunction>();
            }
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Function {
    pub arity: usize,
    pub chunk: Chunk,
    pub name: String, // TODO: this should be an Obj::String?
}

impl Obj for Function {
    fn obj_type(&self) -> ObjType {
        ObjType::Function
    }
}

impl std::fmt::Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.name.is_empty() {
            write!(f, "<script>")
        } else {
            write!(f, "<fn {}>", self.name)
        }
    }
}

impl Function {
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

impl Obj for NativeFunction {
    fn obj_type(&self) -> ObjType {
        ObjType::NativeFunction
    }
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

#[derive(Clone)]
pub struct ObjString {
    pub string: String,
    pub hash: u64,
}

impl Obj for ObjString {
    fn obj_type(&self) -> ObjType {
        ObjType::String
    }
}

impl ObjString {
    pub fn new(string: String) -> ObjString {
        let mut hasher = DefaultHasher::new();
        hasher.write(string.as_bytes());
        let hash = hasher.finish();

        ObjString { string, hash }
    }
}

impl PartialEq for ObjString {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash && self.string == other.string
    }
}

impl Eq for ObjString {}

// A naive attenot to speed-up access to HashMap<ObjString, ...>
impl Hash for ObjString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl std::fmt::Display for ObjString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.string)
    }
}
