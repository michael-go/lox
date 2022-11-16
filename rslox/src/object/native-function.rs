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
