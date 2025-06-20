use log::debug;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString};

use crate::python_types::{create_instance_from_xml_events, PyTypeInfo};
use crate::tag_finder::{TagEvent, TagFinder};
use crate::xml_parser::StreamParser as XmlStreamParser;
use crate::xml_types::XmlValue;

/// Wrapper for the StreamParser that handles typed conversions
#[derive(Debug)]
pub struct TypedStreamParser {
    tag_finder: TagFinder,
    xml_parser: XmlStreamParser,
    type_info: Option<PyTypeInfo>,
    partial_data: Option<XmlValue>,
    is_done: bool,
    capturing: bool,
    is_pydantic_model: bool,
    xml_buffer: String,
}

impl TypedStreamParser {
    pub fn new(wanted_tags: Vec<String>, ignored_tags: Vec<String>) -> Self {
        Self {
            tag_finder: TagFinder::new_with_filter(wanted_tags, ignored_tags),
            xml_parser: XmlStreamParser::new(),
            type_info: None,
            partial_data: None,
            is_done: false,
            capturing: false,
            is_pydantic_model: false,
            xml_buffer: String::new(),
        }
    }

    pub fn with_type(
        type_info: PyTypeInfo,
        wanted_tags: Vec<String>,
        ignored_tags: Vec<String>,
    ) -> Self {
        Self {
            tag_finder: TagFinder::new_with_filter(wanted_tags, ignored_tags),
            xml_parser: XmlStreamParser::new(),
            type_info: Some(type_info),
            partial_data: None,
            is_done: false,
            capturing: false,
            is_pydantic_model: false,
            xml_buffer: String::new(),
        }
    }

    /// Process a chunk of XML data
    pub fn step(&mut self, chunk: &str) -> PyResult<Option<PyObject>> {
        let mut result = None;
        self.tag_finder.push(chunk, |event| {
            match event {
                TagEvent::Open(_) => {
                    self.capturing = true;
                    self.xml_buffer.clear();
                }
                TagEvent::Bytes(bytes) => {
                    if self.capturing {
                        self.xml_buffer.push_str(&bytes);
                    }
                }
                TagEvent::Close(_) => {
                    self.capturing = false;
                    self.is_done = true;
                    if let Some(type_info) = &self.type_info {
                        let events = self.xml_parser.step(&self.xml_buffer)?;
                        result = Some(
                            pyo3::Python::with_gil(|py| {
                                create_instance_from_xml_events(
                                    py,
                                    type_info,
                                    events.into_iter().map(Ok).collect(),
                                )
                            })
                            .map_err(|e| crate::xml_types::XmlError::ParserError(e.to_string()))?,
                        );
                    }
                }
            }
            Ok(())
        })?;
        Ok(result)
    }

    /// Check if parsing is complete
    pub fn is_done(&self) -> bool {
        self.is_done
    }

    /// Convert current partial data to Python object using type info
    pub fn to_python_object(&mut self, py: Python) -> PyResult<Option<PyObject>> {
        // This function is no longer needed, as the Python object is created in the step function.
        // However, we need to return something.
        Ok(None)
    }
}

/// Python wrapper for the TypedStreamParser
#[pyclass(name = "Parser", unsendable)]
pub struct PyParser {
    parser: TypedStreamParser,
    result: Option<PyObject>,
}

#[pymethods]
impl PyParser {
    #[new]
    #[pyo3(signature = (type_obj=None, ignored_tags=vec!["think".to_string(), "thinking".to_string(), "system".to_string(), "thought".to_string()]))]
    fn new(py: Python, type_obj: Option<&PyAny>, ignored_tags: Vec<String>) -> PyResult<Self> {
        debug!(
            "[PyParser::new] type_obj: {:?}",
            type_obj.map(|o| o
                .repr()
                .unwrap_or_else(|_| PyString::new(py, "Error getting repr").into()))
        );
        match type_obj {
            Some(obj) => {
                let mut type_info = PyTypeInfo::extract_from_python(obj)?;
                debug!(
                    "[PyParser::new] Extracted type_info: name='{}', kind='{:?}', origin='{:?}'",
                    type_info.name, type_info.kind, type_info.origin
                );

                if type_info.py_type.is_none() {
                    type_info.py_type = Some(obj.into_py(py));
                }

                let wanted_tags = vec![type_info.name.clone()];
                let parser = TypedStreamParser::with_type(type_info, wanted_tags, ignored_tags);
                Ok(Self {
                    parser,
                    result: None,
                })
            }
            None => {
                debug!("[PyParser::new] No type_obj provided.");
                let parser = TypedStreamParser::new(Vec::new(), ignored_tags);
                Ok(Self {
                    parser,
                    result: None,
                })
            }
        }
    }

    /// Create a parser for a Pydantic model
    #[staticmethod]
    #[pyo3(text_signature = "(pydantic_model)")]
    fn from_pydantic(py: Python, pydantic_model: &PyAny) -> PyResult<Self> {
        let mut type_info = PyTypeInfo::extract_from_python(pydantic_model)?;
        if type_info.py_type.is_none() {
            type_info.py_type = Some(pydantic_model.into_py(py));
        }

        let wanted_tags = vec![type_info.name.clone()];
        let mut typed_stream_parser =
            TypedStreamParser::with_type(type_info, wanted_tags, Vec::new());
        typed_stream_parser.is_pydantic_model = true;

        Ok(Self {
            parser: typed_stream_parser,
            result: None,
        })
    }

    #[pyo3(text_signature = "($self, chunk)")]
    fn feed(&mut self, py: Python, chunk: &str) -> PyResult<Option<PyObject>> {
        debug!("Feeding chunk: {}", chunk);
        if let Some(res) = self.parser.step(chunk)? {
            self.result = Some(res);
        }
        Ok(self.result.clone())
    }

    #[pyo3(text_signature = "($self)")]
    fn is_complete(&self) -> bool {
        self.parser.is_done()
    }

    #[pyo3(text_signature = "($self)")]
    fn get_partial(&mut self, py: Python) -> PyResult<Option<PyObject>> {
        Ok(self.result.clone())
    }

    #[pyo3(text_signature = "($self)")]
    fn validate(&mut self, py: Python) -> PyResult<Option<PyObject>> {
        self.get_partial(py)
    }
}
