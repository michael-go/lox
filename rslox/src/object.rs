use crate::chunk::Chunk;

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

#[derive(Clone, PartialEq)]
pub enum Obj {
    String(String),
    Function(Function),
}

impl<'a> std::fmt::Display for Obj {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Obj::String(s) => write!(fmt, "\"{}\"", s),
            Obj::Function(f) => write!(fmt, "{}", f),
        }
    }
}
