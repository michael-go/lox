use crate::value::Value;
use downcast_rs::{impl_downcast, Downcast};

include!("string.rs");
include!("function.rs");
include!("native-function.rs");
include!("closure.rs");
include!("class.rs");

#[derive(PartialEq)]
pub enum ObjType {
    String,
    Function,
    NativeFunction,
    Closure,
    Class,
    Instance,
    BoundMethod,
}

pub trait Obj: Downcast {
    fn obj_type(&self) -> ObjType;
}
impl_downcast!(Obj);

impl std::fmt::Display for dyn Obj {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.obj_type() {
            ObjType::String => write!(fmt, "{}", self.downcast_ref::<ObjString>().unwrap()),
            ObjType::Function => write!(fmt, "{}", self.downcast_ref::<Function>().unwrap()),
            ObjType::NativeFunction => {
                write!(fmt, "{}", self.downcast_ref::<NativeFunction>().unwrap())
            }
            ObjType::Closure => write!(fmt, "{}", self.downcast_ref::<Closure>().unwrap()),
            ObjType::Class => write!(fmt, "{}", self.downcast_ref::<Class>().unwrap()),
            ObjType::Instance => write!(fmt, "{}", self.downcast_ref::<Instance>().unwrap()),
            ObjType::BoundMethod => write!(fmt, "{}", self.downcast_ref::<BoundMethod>().unwrap()),
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
            _ => return self as *const _ == other as *const _,
        }
    }
}
