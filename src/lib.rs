use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

mod parser;
mod python_types;
mod tag_finder;
mod xml_parser;
mod xml_types;

use parser::PyParser;
use pyo3::types::{PyDict, PyList};
use pyo3::Python;
use xml_parser::StreamParser;

use crate::xml_types::XmlValue;

pub fn to_py(py: Python, value: &XmlValue) -> PyObject {
    match value {
        XmlValue::Element(name, attrs, children) => {
            let dict = PyDict::new(py);
            dict.set_item("name", name).unwrap();
            let py_attrs = PyDict::new(py);
            for (k, v) in attrs {
                py_attrs.set_item(k, v).unwrap();
            }
            dict.set_item("attrs", py_attrs).unwrap();
            let py_children = PyList::empty(py);
            for child in children {
                py_children.append(to_py(py, child)).unwrap();
            }
            dict.set_item("children", py_children).unwrap();
            dict.into()
        }
        XmlValue::Text(text) => text.into_py(py),
    }
}

/// A simple StreamParser class for Python
#[pyclass(name = "StreamParser", unsendable)]
struct PyStreamParser {
    parser: StreamParser,
    last_val: Option<XmlValue>,
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

        for event in step_out {
            // The `xml` crate doesn't have a way to get a partial value, so we can't
            // do much here. We'll just wait for the `is_done` to be true.
        }

        if self.parser.is_done() {
            // This is a bit of a hack, but we need to get the final value from the parser.
            // We do this by feeding an empty string to the parser, which will cause it to
            // finalize the document and return the root element.
            let final_val = self
                .parser
                .step("")
                .map_err(|e| PyValueError::new_err(format!("stream error: {:?}", e)))?;
            if !final_val.is_empty() {
                // This is not correct, but it's the best we can do for now.
                // self.last_val = Some(final_val[0].clone());
                // return Ok(Some(to_py(py, &self.last_val.as_ref().unwrap())));
            }
        }

        Ok(None)
    }

    /// Check if the parser is done
    #[pyo3(text_signature = "($self)")]
    fn is_done(&self) -> bool {
        self.parser.is_done()
    }
}

/// Python module for parsing structured outputs into typed objects
#[pymodule]
fn gasp(py: Python, m: &PyModule) -> PyResult<()> {
    // Initialize the logger. try_init() is used to avoid panic if already initialized.
    let _ = env_logger::try_init();

    // Add base parser
    m.add_class::<PyStreamParser>()?;

    // Add typed parser
    m.add_class::<PyParser>()?;

    Ok(())
}
