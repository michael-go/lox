use crate::chunk::Chunk;

#[derive(Clone, PartialEq)]
pub struct Function {
    pub arity: usize,
    pub chunk: Chunk,
    pub name: String, // TODO: this should be an Obj::String?
    pub upvalue_count: usize,
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
            upvalue_count: 0,
        }
    }
}
