use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum XmlValue {
    Element(String, HashMap<String, String>, Vec<XmlValue>),
    Text(String),
}

impl fmt::Display for XmlValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XmlValue::Element(name, attrs, children) => {
                write!(f, "<{}", name)?;
                for (k, v) in attrs {
                    write!(f, " {}=\"{}\"", k, v)?;
                }
                if children.is_empty() {
                    write!(f, "/>")
                } else {
                    write!(f, ">")?;
                    for child in children {
                        write!(f, "{}", child)?;
                    }
                    write!(f, "</{}>", name)
                }
            }
            XmlValue::Text(text) => write!(f, "{}", text),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum XmlError {
    UnexpectedEof,
    ParserError(String),
}

impl fmt::Display for XmlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XmlError::UnexpectedEof => write!(f, "Unexpected end of file"),
            XmlError::ParserError(msg) => write!(f, "XML Parser Error: {}", msg),
        }
    }
}

impl From<XmlError> for PyErr {
    fn from(err: XmlError) -> PyErr {
        PyValueError::new_err(err.to_string())
    }
}
