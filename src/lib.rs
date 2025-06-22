use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

mod parser;
mod python_types;
mod tag_finder;
mod type_string_parser;
mod xml_parser;
mod xml_types;

use parser::PyParser;
use xml_parser::StreamParser;

/// A simple StreamParser class for Python
#[pyclass(name = "StreamParser", unsendable)]
struct PyStreamParser {
    parser: StreamParser,
}

#[pymethods]
impl PyStreamParser {
    #[new]
    fn new() -> Self {
        Self {
            parser: StreamParser::default(),
        }
    }

    /// Feed a chunk; returns the parsed value once complete, else `None`.
    #[pyo3(text_signature = "($self, chunk)")]
    fn parse<'p>(&mut self, py: Python<'p>, chunk: &str) -> PyResult<Option<PyObject>> {
        // StreamParser is not fully implemented
        // Just consume the chunk and return None
        let _ = self
            .parser
            .step(chunk)
            .map_err(|e| PyValueError::new_err(format!("stream error: {:?}", e)))?;

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
