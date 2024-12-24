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
