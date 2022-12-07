use hashbrown::HashMap;

#[derive(PartialEq)]
pub struct Class {
    name: ObjString,
    pub methods: RefCell<HashMap<ObjString, Rc<Closure>>>,
}

impl Class {
    pub fn new(name: ObjString) -> Self {
        Self {
            name,
            methods: RefCell::new(HashMap::new()),
        }
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

#[derive(PartialEq)]
pub struct BoundMethod {
    pub receiver: Value,
    pub method: Rc<Closure>,
}

impl BoundMethod {
    pub fn new(receiver: Value, method: Rc<Closure>) -> Self {
        Self { receiver, method }
    }
}

impl Obj for BoundMethod {
    fn obj_type(&self) -> ObjType {
        ObjType::BoundMethod
    }
}

impl std::fmt::Display for BoundMethod {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{}", self.method)
    }
}