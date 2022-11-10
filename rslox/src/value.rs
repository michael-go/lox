use crate::object::Obj;

#[derive(Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    Nil,
    Number(f64),
    Obj(Obj),
}

impl<'a> std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Bool(b) => write!(f, "{}", b),
            Value::Nil => write!(f, "nil"),
            Value::Number(n) => write!(f, "{}", n),
            Value::Obj(o) => write!(f, "{}", o),
        }
    }
}
