use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

mod json_parser;
mod json_sax_scanner;
mod json_tok;
mod json_types;
mod parser;
mod python_types;
mod tag_finder;

use json_parser::StreamParser;
use parser::PyParser;
use pyo3::types::{PyDict, PyFloat, PyList, PyLong, PyString, PyType};
use pyo3::Python;
use std::collections::HashMap;

use crate::json_types::{JsonValue, Number};

pub fn to_py(py: Python, value: &JsonValue) -> PyObject {
    match value {
        JsonValue::Object(map) => {
            let dict = PyDict::new(py);
            for (k, v) in map {
                dict.set_item(k, json_value_to_py_object(py, v)).unwrap();
            }
            dict.into()
        }
        JsonValue::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(json_value_to_py_object(py, item)).unwrap();
            }
            list.into()
        }
        JsonValue::String(s) => s.into_py(py),
        JsonValue::Number(n) => match n {
            Number::Integer(i) => i.into_py(py),
            Number::Float(f) => f.into_py(py),
        },
        JsonValue::Boolean(b) => b.into_py(py),
        JsonValue::Null => py.None(),
    }
}

pub fn json_value_to_py_object(py: Python, value: &JsonValue) -> PyObject {
    match value {
        JsonValue::Object(map) => {
            let dict = PyDict::new(py);
            for (k, v) in map {
                dict.set_item(k, json_value_to_py_object(py, v)).unwrap();
            }
            dict.into()
        }
        JsonValue::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(json_value_to_py_object(py, item)).unwrap();
            }
            list.into()
        }
        JsonValue::String(s) => s.into_py(py),
        JsonValue::Number(n) => match n {
            Number::Integer(i) => i.into_py(py),
            Number::Float(f) => f.into_py(py),
        },
        JsonValue::Boolean(b) => b.into_py(py),
        JsonValue::Null => py.None(),
    }
}

/// A simple StreamParser class for Python
#[pyclass(name = "StreamParser", unsendable)]
struct PyStreamParser {
    parser: StreamParser,
    last_val: Option<JsonValue>,
}

#[pymethods]
impl PyStreamParser {
    #[new]
    fn new() -> Self {
        Self {
            parser: StreamParser::default(),
            last_val: None,
        }
    }

    /// Feed a chunk; returns the parsed value once complete, else `None`.
    #[pyo3(text_signature = "($self, chunk)")]
    fn parse<'p>(&mut self, py: Python<'p>, chunk: &str) -> PyResult<Option<PyObject>> {
        let step_out = self
            .parser
            .step(chunk)
            .map_err(|e| PyValueError::new_err(format!("stream error: {:?}", e)))?;

        if let Some(val) = step_out {
            self.last_val = Some(val.clone());
            return Ok(Some(to_py(py, &val)));
        }
        Ok(None)
    }

    /// Check if the parser is done
    #[pyo3(text_signature = "($self)")]
    fn is_done(&self) -> bool {
        self.parser.is_done()
    }
}

/// Create a pure Python implementation of Deserializable since
/// it's easier to work with directly in Python
fn create_deserializable_class(py: Python, module: &PyModule) -> PyResult<()> {
    let deserializable_code = r#"
class Deserializable:
    """Base class for types that can be deserialized from JSON"""
    
    @classmethod
    def __gasp_register__(cls):
        """Register the type for deserialization"""
        pass
    
    @classmethod
    def __gasp_from_partial__(cls, partial_data):
        """Create an instance from partial data"""
        instance = cls()
        for key, value in partial_data.items():
            setattr(instance, key, value)
        return instance
    
    def __gasp_update__(self, new_data):
        """Update instance with new data"""
        for key, value in new_data.items():
            setattr(self, key, value)
    
    # Pydantic V2 compatibility methods
    @classmethod
    def model_validate(cls, obj):
        """Pydantic V2 compatible validation method"""
        return cls.__gasp_from_partial__(obj)
    
    @classmethod
    def model_fields(cls):
        """Return field information compatible with Pydantic V2"""
        fields = {}
        for name, type_hint in getattr(cls, "__annotations__", {}).items():
            fields[name] = {"type": type_hint}
        return fields
    
    def model_dump(self):
        """Convert model to dict (Pydantic V2 compatible)"""
        return {k: v for k, v in self.__dict__.items() if not k.startswith('_')}

# Explicitly add Deserializable to __all__ to make it importable
__all__ = ['Deserializable', 'Parser', 'StreamParser']
"#;

    py.run(deserializable_code, None, Some(module.dict()))?;

    // Make sure Deserializable is explicitly added to the module
    module.add(
        "Deserializable",
        py.eval("Deserializable", None, Some(module.dict()))?,
    )?;

    Ok(())
}

/// Python module for parsing structured outputs into typed objects
#[pymodule]
fn gasp(py: Python, m: &PyModule) -> PyResult<()> {
    // Add base parser
    m.add_class::<PyStreamParser>()?;

    // Add typed parser
    m.add_class::<PyParser>()?;

    // Add Deserializable base class implemented in Python
    create_deserializable_class(py, m)?;

    Ok(())
}
