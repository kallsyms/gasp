use log::debug;
use pyo3::prelude::*;
use pyo3::types::PyString;

use crate::python_types::PyTypeInfo;
use crate::tag_finder::TagFinder;
use crate::xml_parser::StreamParser as XmlStreamParser;

/// Wrapper for the StreamParser that handles typed conversions
#[derive(Debug)]
pub struct TypedStreamParser {
    tag_finder: TagFinder,
    xml_parser: XmlStreamParser,
    type_info: Option<PyTypeInfo>,
    is_done: bool,
    capturing: bool,
    is_pydantic_model: bool,
    xml_buffer: String,
    partial_instance: Option<PyObject>,
    current_field: Option<String>,
    current_field_content: String,
}

impl TypedStreamParser {
    pub fn new(wanted_tags: Vec<String>, ignored_tags: Vec<String>) -> Self {
        Self {
            tag_finder: TagFinder::new_with_filter(wanted_tags, ignored_tags),
            xml_parser: XmlStreamParser::new(),
            type_info: None,
            is_done: false,
            capturing: false,
            is_pydantic_model: false,
            xml_buffer: String::new(),
            partial_instance: None,
            current_field: None,
            current_field_content: String::new(),
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
            is_done: false,
            capturing: false,
            is_pydantic_model: false,
            xml_buffer: String::new(),
            partial_instance: None,
            current_field: None,
            current_field_content: String::new(),
        }
    }

    /// Process a chunk of XML data with support for partial field values
    pub fn step(&mut self, chunk: &str) -> PyResult<Option<PyObject>> {
        // For now, just accumulate the XML and use the tag_finder + xml_parser
        // This is a simplified approach that doesn't support partial streaming
        self.xml_buffer.push_str(chunk);

        // Feed to tag_finder
        let mut events = Vec::new();
        self.tag_finder
            .push(chunk, |event| {
                events.push(event);
                Ok(())
            })
            .map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Tag parsing error: {:?}", e))
            })?;

        // For Union types, we need to check if we've seen a complete tag
        if let Some(type_info) = &self.type_info {
            if type_info.kind == crate::python_types::PyTypeKind::Union {
                // First check if we have the union wrapper tag (e.g., <MyUnion type="A">)
                let union_open_tag = format!("<{}", type_info.name);
                let union_close_tag = format!("</{}>", type_info.name);

                if self.xml_buffer.contains(&union_open_tag)
                    && self.xml_buffer.contains(&union_close_tag)
                {
                    // We have a complete union wrapper element, parse it
                    let xml_events = self.xml_parser.step(&self.xml_buffer).map_err(|e| {
                        pyo3::exceptions::PyValueError::new_err(format!(
                            "XML parsing error: {:?}",
                            e
                        ))
                    })?;

                    if !xml_events.is_empty() {
                        // Convert XML events to Python object using the union type
                        // Wrap events in Ok() to match expected type
                        let wrapped_events: Vec<Result<xml::Event, crate::xml_types::XmlError>> =
                            xml_events.into_iter().map(Ok).collect();

                        let result = pyo3::Python::with_gil(|py| {
                            crate::python_types::create_instance_from_xml_events(
                                py,
                                type_info,
                                wrapped_events,
                            )
                        })?;

                        self.is_done = true;
                        return Ok(Some(result));
                    }
                }

                // Otherwise check if we have a complete element for any union member
                for member in &type_info.args {
                    let open_tag = format!("<{}", member.name);
                    let close_tag = format!("</{}>", member.name);

                    if self.xml_buffer.contains(&open_tag) && self.xml_buffer.contains(&close_tag) {
                        // We have a complete element, parse it
                        let xml_events = self.xml_parser.step(&self.xml_buffer).map_err(|e| {
                            pyo3::exceptions::PyValueError::new_err(format!(
                                "XML parsing error: {:?}",
                                e
                            ))
                        })?;

                        if !xml_events.is_empty() {
                            // Convert XML events to Python object using the union type
                            // Wrap events in Ok() to match expected type
                            let wrapped_events: Vec<
                                Result<xml::Event, crate::xml_types::XmlError>,
                            > = xml_events.into_iter().map(Ok).collect();

                            let result = pyo3::Python::with_gil(|py| {
                                crate::python_types::create_instance_from_xml_events(
                                    py,
                                    type_info,
                                    wrapped_events,
                                )
                            })?;

                            self.is_done = true;
                            return Ok(Some(result));
                        }
                    }
                }
            } else {
                // Non-union type, check for the type's tag
                let open_tag = format!("<{}", type_info.name);
                let close_tag = format!("</{}>", type_info.name);

                if self.xml_buffer.contains(&open_tag) && self.xml_buffer.contains(&close_tag) {
                    // We have a complete element, parse it
                    let xml_events = self.xml_parser.step(&self.xml_buffer).map_err(|e| {
                        pyo3::exceptions::PyValueError::new_err(format!(
                            "XML parsing error: {:?}",
                            e
                        ))
                    })?;

                    if !xml_events.is_empty() {
                        // Convert XML events to Python object
                        // Wrap events in Ok() to match expected type
                        let wrapped_events: Vec<Result<xml::Event, crate::xml_types::XmlError>> =
                            xml_events.into_iter().map(Ok).collect();

                        let result = pyo3::Python::with_gil(|py| {
                            crate::python_types::create_instance_from_xml_events(
                                py,
                                type_info,
                                wrapped_events,
                            )
                        })?;

                        self.is_done = true;
                        return Ok(Some(result));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Check if parsing is complete
    pub fn is_done(&self) -> bool {
        self.is_done
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

                // For Union types, we want to look for any of the union member tags
                // AND the union type name itself (for type aliases)
                let wanted_tags = if type_info.kind == crate::python_types::PyTypeKind::Union {
                    let mut tags: Vec<String> =
                        type_info.args.iter().map(|arg| arg.name.clone()).collect();
                    // Also add the union type name itself for type aliases
                    tags.push(type_info.name.clone());
                    tags
                } else {
                    vec![type_info.name.clone()]
                };
                debug!("[PyParser::new] wanted_tags: {:?}", wanted_tags);
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
