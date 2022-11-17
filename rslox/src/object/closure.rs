use std::rc::Rc;
use std::cell::RefCell;

#[derive(Clone, PartialEq)]
pub struct Upvalue {
    pub location: usize,
    pub closed: Option<Value>,
}

#[derive(PartialEq)]
pub struct Closure {
    pub function: Rc<Function>,
    pub upvalues: Vec<Rc<RefCell<Upvalue>>>,
}

impl Closure {
    pub fn new(function: Function) -> Closure {
        let upvalue_count = function.upvalue_count;
        Closure { 
            function: Rc::new(function),
            upvalues: Vec::with_capacity(upvalue_count as usize),
        }
    }
}

impl Obj for Closure {
    fn obj_type(&self) -> ObjType {
        ObjType::Closure
    }
}

impl std::fmt::Display for Closure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.function)
    }
}