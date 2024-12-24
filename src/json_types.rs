use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum JsonValue {
    Object(HashMap<String, JsonValue>),
    Array(Vec<JsonValue>),
    String(String),
    Number(Number),
    Boolean(bool),
    Null,
}

#[derive(Debug, Clone)]
pub enum Number {
    Integer(i64),
    Float(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum JsonError {
    DuplicateKey(String),
    UnexpectedChar(char),
    UnexpectedEof,
    InvalidNumber(String),
    UnmatchedBrace,
    UnmatchedBracket,
    ExpectedColon,
    ExpectedComma,
    InvalidEscape,
    InvalidString,
}

impl JsonValue {
    pub fn as_string(&self) -> Option<&String> {
        match self {
            JsonValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<JsonValue>> {
        match self {
            JsonValue::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&HashMap<String, JsonValue>> {
        match self {
            JsonValue::Object(o) => Some(o),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<&Number> {
        match self {
            JsonValue::Number(n) => Some(n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, JsonValue::Null)
    }
}
