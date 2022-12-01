use hashbrown::HashMap;

#[derive(PartialEq)]
pub struct Class {
    name: ObjString
}

impl Class {
    pub fn new(name: ObjString) -> Self {
        Self { name }
    }
}

impl Obj for Class {
    fn obj_type(&self) -> ObjType {
        ObjType::Class
    }
}

impl std::fmt::Display for Class {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{}", self.name)
    }
}

#[derive(PartialEq)]
pub struct Instance {
    pub class: Rc<Class>,
    pub fields: RefCell<HashMap<ObjString, Value>>,
}

impl Instance {
    pub fn new(class: Rc<Class>) -> Self {
        Self {
            class,
            fields: RefCell::new(HashMap::new()),
        }
    }
}

impl Obj for Instance {
    fn obj_type(&self) -> ObjType {
        ObjType::Instance
    }
}

impl std::fmt::Display for Instance {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{} instance", self.class)
    }
}