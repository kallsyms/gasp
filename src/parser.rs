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
        let mut result = None;

        // Feed to tag_finder and reconstruct filtered XML
        let mut events = Vec::new();
        self.tag_finder
            .push(chunk, |event| {
                events.push(event.clone());

                // Build filtered XML buffer from tag events
                match &event {
                    crate::tag_finder::TagEvent::Open(name) => {
                        self.xml_buffer.push('<');
                        self.xml_buffer.push_str(name);
                        self.xml_buffer.push('>');
                    }
                    crate::tag_finder::TagEvent::Bytes(content) => {
                        self.xml_buffer.push_str(content);
                    }
                    crate::tag_finder::TagEvent::Close(name) => {
                        self.xml_buffer.push_str("</");
                        self.xml_buffer.push_str(name);
                        self.xml_buffer.push('>');
                    }
                }
                Ok(())
            })
            .map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Tag parsing error: {:?}", e))
            })?;

        // Process for incremental parsing
        if let Some(type_info) = &self.type_info {
            // Handle raw primitive types (str, int, float, bool)
            match type_info.kind {
                crate::python_types::PyTypeKind::String
                | crate::python_types::PyTypeKind::Integer
                | crate::python_types::PyTypeKind::Float
                | crate::python_types::PyTypeKind::Boolean
                | crate::python_types::PyTypeKind::None => {
                    // For raw primitives, look for any complete element
                    // Try to find a complete element in the buffer
                    let mut tag_start = None;
                    let mut tag_name = String::new();

                    // Find opening tag
                    if let Some(start_idx) = self.xml_buffer.find('<') {
                        if let Some(end_idx) = self.xml_buffer[start_idx..].find('>') {
                            let tag_content = &self.xml_buffer[start_idx + 1..start_idx + end_idx];
                            if !tag_content.starts_with('/') && !tag_content.is_empty() {
                                // Extract tag name (before any space or >)
                                tag_name = tag_content
                                    .split_whitespace()
                                    .next()
                                    .unwrap_or("")
                                    .to_string();
                                tag_start = Some(start_idx);
                            }
                        }
                    }

                    // Check if we have a complete element
                    if !tag_name.is_empty() {
                        let close_tag = format!("</{}>", tag_name);
                        if self.xml_buffer.contains(&close_tag) {
                            // We have a complete element, parse it
                            let xml_events =
                                self.xml_parser.step(&self.xml_buffer).map_err(|e| {
                                    pyo3::exceptions::PyValueError::new_err(format!(
                                        "XML parsing error: {:?}",
                                        e
                                    ))
                                })?;

                            if !xml_events.is_empty() {
                                // Extract the text content and convert to the primitive type
                                let xml_value = crate::xml_parser::events_to_xml_value(
                                    xml_events.into_iter().map(Ok).collect(),
                                )?;

                                if let crate::xml_types::XmlValue::Element(_, _, children) =
                                    xml_value
                                {
                                    // Check if children is empty (empty element)
                                    if children.is_empty() {
                                        // Handle empty element
                                        let parsed_result =
                                            pyo3::Python::with_gil(|py| match type_info.kind {
                                                crate::python_types::PyTypeKind::String => {
                                                    Ok("".to_string().into_py(py))
                                                }
                                                crate::python_types::PyTypeKind::Integer => {
                                                    Ok::<PyObject, PyErr>(py.None())
                                                }
                                                crate::python_types::PyTypeKind::Float => {
                                                    Ok::<PyObject, PyErr>(py.None())
                                                }
                                                crate::python_types::PyTypeKind::Boolean => {
                                                    Ok(false.into_py(py))
                                                }
                                                _ => Ok(py.None()),
                                            })?;

                                        self.is_done = true;
                                        return Ok(Some(parsed_result));
                                    }

                                    // Look for text content in children
                                    for child in children {
                                        if let crate::xml_types::XmlValue::Text(text) = child {
                                            // Convert text to the appropriate primitive type
                                            let parsed_result = pyo3::Python::with_gil(|py| {
                                                match type_info.kind {
                                                    crate::python_types::PyTypeKind::String => {
                                                        Ok(text.into_py(py))
                                                    }
                                                    crate::python_types::PyTypeKind::Integer => {
                                                        match text.parse::<i64>() {
                                                            Ok(val) => Ok(val.into_py(py)),
                                                            Err(_) => {
                                                                Ok::<PyObject, PyErr>(py.None())
                                                            }
                                                        }
                                                    }
                                                    crate::python_types::PyTypeKind::Float => {
                                                        match text.parse::<f64>() {
                                                            Ok(val) => Ok(val.into_py(py)),
                                                            Err(_) => {
                                                                Ok::<PyObject, PyErr>(py.None())
                                                            }
                                                        }
                                                    }
                                                    crate::python_types::PyTypeKind::Boolean => {
                                                        let val = match text.to_lowercase().as_str()
                                                        {
                                                            "true" | "1" | "yes" => true,
                                                            "false" | "0" | "no" => false,
                                                            _ => false,
                                                        };
                                                        Ok(val.into_py(py))
                                                    }
                                                    crate::python_types::PyTypeKind::None => {
                                                        Ok(py.None())
                                                    }
                                                    _ => Ok(py.None()),
                                                }
                                            })?;

                                            self.is_done = true;
                                            return Ok(Some(parsed_result));
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // If we're still collecting data, check for partial content
                    if !tag_name.is_empty() && !self.is_done {
                        let open_tag = format!("<{}", tag_name);
                        let close_tag = format!("</{}>", tag_name);

                        if self.xml_buffer.contains(&open_tag)
                            && !self.xml_buffer.contains(&close_tag)
                        {
                            // We're inside the tag, extract partial content
                            if let Some(start_idx) = self.xml_buffer.rfind('>') {
                                if start_idx < self.xml_buffer.len() - 1 {
                                    let partial_content = &self.xml_buffer[start_idx + 1..];
                                    if !partial_content.trim().is_empty()
                                        && !partial_content.contains('<')
                                    {
                                        // Return partial content as string for now
                                        result = Some(pyo3::Python::with_gil(|py| {
                                            partial_content.to_string().into_py(py)
                                        }));
                                    }
                                }
                            }
                        }
                    }

                    return Ok(result);
                }
                crate::python_types::PyTypeKind::List
                | crate::python_types::PyTypeKind::Dict
                | crate::python_types::PyTypeKind::Tuple
                | crate::python_types::PyTypeKind::Set => {
                    // For raw container types, look for complete container elements
                    let container_tag = match type_info.kind {
                        crate::python_types::PyTypeKind::List => "list",
                        crate::python_types::PyTypeKind::Dict => "dict",
                        crate::python_types::PyTypeKind::Tuple => "tuple",
                        crate::python_types::PyTypeKind::Set => "set",
                        _ => return Ok(None),
                    };

                    let open_tag = format!("<{}", container_tag);
                    let close_tag = format!("</{}>", container_tag);

                    if self.xml_buffer.contains(&open_tag) && self.xml_buffer.contains(&close_tag) {
                        // We have a complete container element, parse it
                        let xml_events = self.xml_parser.step(&self.xml_buffer).map_err(|e| {
                            pyo3::exceptions::PyValueError::new_err(format!(
                                "XML parsing error: {:?}",
                                e
                            ))
                        })?;

                        if !xml_events.is_empty() {
                            // Convert XML events to Python object
                            let wrapped_events: Vec<
                                Result<xml::Event, crate::xml_types::XmlError>,
                            > = xml_events.into_iter().map(Ok).collect();

                            let parsed_result = pyo3::Python::with_gil(|py| {
                                crate::python_types::create_instance_from_xml_events(
                                    py,
                                    type_info,
                                    wrapped_events,
                                )
                            })?;

                            self.is_done = true;
                            return Ok(Some(parsed_result));
                        }
                    }

                    return Ok(result);
                }
                _ => {
                    // Continue with existing logic for non-primitive types
                }
            }

            // Check if we're inside the main element for non-union types
            if type_info.kind != crate::python_types::PyTypeKind::Union {
                if self.xml_buffer.contains(&format!("<{}>", type_info.name))
                    || self.xml_buffer.contains(&format!("<{} ", type_info.name))
                {
                    // We're inside the main element
                    if self.partial_instance.is_none() {
                        // Create instance on first encounter
                        self.partial_instance = Some(pyo3::Python::with_gil(|py| {
                            type_info
                                .py_type
                                .as_ref()
                                .unwrap()
                                .as_ref(py)
                                .call0()
                                .unwrap()
                                .into()
                        }));
                    }

                    // Check for field tags and extract partial content
                    for (field_name, field_info) in &type_info.fields {
                        let field_start = format!("<{}", field_name);
                        let field_end = format!("</{}>", field_name);

                        if self.xml_buffer.contains(&field_start)
                            && !self.xml_buffer.contains(&field_end)
                        {
                            // We're inside a field tag, extract partial content
                            if let Some(start_idx) = self.xml_buffer.rfind('>') {
                                if start_idx < self.xml_buffer.len() - 1 {
                                    let partial_content = &self.xml_buffer[start_idx + 1..];
                                    if !partial_content.trim().is_empty()
                                        && !partial_content.contains('<')
                                    {
                                        // Update the field with partial content
                                        pyo3::Python::with_gil(|py| {
                                            if let Some(instance) = &self.partial_instance {
                                                let _ = instance
                                                    .as_ref(py)
                                                    .setattr(field_name.as_str(), partial_content);
                                            }
                                        });
                                        result = self.partial_instance.clone();
                                    }
                                }
                            }
                        } else if self.xml_buffer.contains(&field_end) {
                            // Field is complete, extract final content
                            if let Some(start_idx) = self.xml_buffer.rfind(&field_start) {
                                if let Some(content_start) = self.xml_buffer[start_idx..].find('>')
                                {
                                    let content_start_abs = start_idx + content_start + 1;
                                    if let Some(end_idx) = self.xml_buffer.rfind(&field_end) {
                                        let content = &self.xml_buffer[content_start_abs..end_idx];
                                        // Update with final content
                                        pyo3::Python::with_gil(|py| {
                                            if let Some(instance) = &self.partial_instance {
                                                let _ = instance
                                                    .as_ref(py)
                                                    .setattr(field_name.as_str(), content);
                                            }
                                        });
                                        result = self.partial_instance.clone();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // For Union types and complete element detection, keep existing logic
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

                        let parsed_result = pyo3::Python::with_gil(|py| {
                            crate::python_types::create_instance_from_xml_events(
                                py,
                                type_info,
                                wrapped_events,
                            )
                        })?;

                        self.is_done = true;
                        return Ok(Some(parsed_result));
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

                        let parsed_result = pyo3::Python::with_gil(|py| {
                            crate::python_types::create_instance_from_xml_events(
                                py,
                                type_info,
                                wrapped_events,
                            )
                        })?;

                        self.is_done = true;
                        // Store in partial_instance so it gets returned at the end
                        self.partial_instance = Some(parsed_result.clone());
                        return Ok(Some(parsed_result));
                    }
                }
            }
        }

        // Return the partial instance if we have one
        Ok(result)
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
                let wanted_tags = match type_info.kind {
                    crate::python_types::PyTypeKind::Union => {
                        let mut tags: Vec<String> =
                            type_info.args.iter().map(|arg| arg.name.clone()).collect();
                        // Also add the union type name itself for type aliases
                        tags.push(type_info.name.clone());
                        tags
                    }
                    crate::python_types::PyTypeKind::String => {
                        vec!["str".to_string()]
                    }
                    crate::python_types::PyTypeKind::Integer => {
                        vec!["int".to_string()]
                    }
                    crate::python_types::PyTypeKind::Float => {
                        vec!["float".to_string()]
                    }
                    crate::python_types::PyTypeKind::Boolean => {
                        vec!["bool".to_string(), "boolean".to_string()]
                    }
                    crate::python_types::PyTypeKind::None => {
                        vec!["None".to_string(), "null".to_string()]
                    }
                    crate::python_types::PyTypeKind::List => {
                        vec!["list".to_string()]
                    }
                    crate::python_types::PyTypeKind::Dict => {
                        vec!["dict".to_string()]
                    }
                    crate::python_types::PyTypeKind::Tuple => {
                        vec!["tuple".to_string()]
                    }
                    crate::python_types::PyTypeKind::Set => {
                        vec!["set".to_string()]
                    }
                    _ => vec![type_info.name.clone()],
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
