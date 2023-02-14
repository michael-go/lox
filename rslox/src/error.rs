use std::fmt::Formatter;

#[derive(Debug, PartialEq)]
pub enum LoxErrorKind {
    RuntimeError,
}

#[derive(Debug, PartialEq)]
pub struct LoxError {
    #[allow(dead_code)]
    pub kind: LoxErrorKind,
    #[allow(dead_code)]
    pub message: String,
}

impl std::fmt::Display for LoxError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for LoxError {}

impl LoxError {
    pub fn new(kind: LoxErrorKind, message: &str) -> LoxError {
        LoxError {
            kind,
            message: message.to_string(),
        }
    }
}
