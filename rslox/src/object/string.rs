use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

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
