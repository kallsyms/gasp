use log::debug;
use pyo3::prelude::*;
use pyo3::types::PyString;

use crate::python_types::PyTypeInfo;
use crate::tag_finder::{Tag, TagFinder};

#[derive(Debug, Clone)]
enum StackFrame {
    List {
        tag_name: String,
        items: Vec<PyObject>,
        item_type: PyTypeInfo,
    },
    Dict {
        tag_name: String,
        entries: Vec<(PyObject, PyObject)>,
        key_type: Option<PyTypeInfo>,
        value_type: Option<PyTypeInfo>,
        current_key: Option<PyObject>,
    },
    Set {
        tag_name: String,
        items: Vec<PyObject>,
        item_type: PyTypeInfo,
    },
    Tuple {
        tag_name: String,
        items: Vec<PyObject>,
        types: Vec<PyTypeInfo>,
    },
    Object {
        type_info: PyTypeInfo,
        instance: PyObject,
        current_field: Option<String>,
    },
    Field {
        name: String,
        content: String,
        type_info: PyTypeInfo,
    },
}

/// Wrapper for the StreamParser that handles typed conversions
#[derive(Debug)]
pub struct TypedStreamParser {
    tag_finder: TagFinder,
    type_info: Option<PyTypeInfo>,
    is_done: bool,
    stack: Vec<StackFrame>,
    stack_based_result: Option<PyObject>,
}

impl TypedStreamParser {
    pub fn new(wanted_tags: Vec<String>, ignored_tags: Vec<String>) -> Self {
        Self {
            tag_finder: TagFinder::new_with_filter(wanted_tags, ignored_tags),
            type_info: None,
            is_done: false,
            stack: Vec::new(),
            stack_based_result: None,
        }
    }

    pub fn with_type(
        type_info: PyTypeInfo,
        wanted_tags: Vec<String>,
        ignored_tags: Vec<String>,
    ) -> Self {
        Self {
            tag_finder: TagFinder::new_with_filter(wanted_tags, ignored_tags),
            type_info: Some(type_info),
            is_done: false,
            stack: Vec::new(),
            stack_based_result: None,
        }
    }

    fn should_use_stack(&self) -> bool {
        if let Some(type_info) = &self.type_info {
            matches!(
                type_info.kind,
                crate::python_types::PyTypeKind::List
                    | crate::python_types::PyTypeKind::Dict
                    | crate::python_types::PyTypeKind::Set
                    | crate::python_types::PyTypeKind::Tuple
                    | crate::python_types::PyTypeKind::Class
                    | crate::python_types::PyTypeKind::Union
            )
        } else {
            false
        }
    }

    fn frame_to_pyobject(&self, frame: StackFrame) -> PyResult<PyObject> {
        pyo3::Python::with_gil(|py| {
            match frame {
                StackFrame::List { items, .. } => Ok(pyo3::types::PyList::new(py, &items).into()),
                StackFrame::Dict { entries, .. } => {
                    let dict = pyo3::types::PyDict::new(py);
                    for (key, value) in entries {
                        dict.set_item(key, value)?;
                    }
                    Ok(dict.into())
                }
                StackFrame::Set { items, .. } => {
                    let set = pyo3::types::PySet::new(py, &items)?;
                    Ok(set.into())
                }
                StackFrame::Tuple { items, .. } => {
                    let tuple = pyo3::types::PyTuple::new(py, &items);
                    Ok(tuple.into())
                }
                StackFrame::Object { instance, .. } => Ok(instance),
                StackFrame::Field {
                    content, type_info, ..
                } => {
                    // Convert content to the appropriate primitive type
                    match type_info.kind {
                        crate::python_types::PyTypeKind::String => {
                            // Decode HTML entities for strings
                            let decoded = content
                                .replace("&amp;", "&")
                                .replace("&lt;", "<")
                                .replace("&gt;", ">")
                                .replace("&quot;", "\"")
                                .replace("&#39;", "'")
                                .replace("&apos;", "'");
                            Ok(decoded.into_py(py))
                        }
                        crate::python_types::PyTypeKind::Integer => match content.parse::<i64>() {
                            Ok(val) => Ok(val.into_py(py)),
                            Err(_) => Ok(py.None()),
                        },
                        crate::python_types::PyTypeKind::Float => match content.parse::<f64>() {
                            Ok(val) => Ok(val.into_py(py)),
                            Err(_) => Ok(py.None()),
                        },
                        crate::python_types::PyTypeKind::Boolean => {
                            let val =
                                matches!(content.to_lowercase().as_str(), "true" | "1" | "yes");
                            Ok(val.into_py(py))
                        }
                        _ => Ok(py.None()),
                    }
                }
            }
        })
    }

    fn push_frame_for_type(&mut self, type_info: &PyTypeInfo, tag_name: &str) -> PyResult<()> {
        let frame = pyo3::Python::with_gil(|py| match type_info.kind {
            crate::python_types::PyTypeKind::List => {
                let item_type = type_info
                    .args
                    .get(0)
                    .cloned()
                    .unwrap_or_else(PyTypeInfo::any);
                Ok(Some(StackFrame::List {
                    tag_name: tag_name.to_string(),
                    items: Vec::new(),
                    item_type,
                }))
            }
            crate::python_types::PyTypeKind::Dict => {
                let key_type = type_info.args.get(0).cloned();
                let value_type = type_info.args.get(1).cloned();
                Ok(Some(StackFrame::Dict {
                    tag_name: tag_name.to_string(),
                    entries: Vec::new(),
                    key_type,
                    value_type,
                    current_key: None,
                }))
            }
            crate::python_types::PyTypeKind::Set => {
                let item_type = type_info
                    .args
                    .get(0)
                    .cloned()
                    .unwrap_or_else(PyTypeInfo::any);
                Ok(Some(StackFrame::Set {
                    tag_name: tag_name.to_string(),
                    items: Vec::new(),
                    item_type,
                }))
            }
            crate::python_types::PyTypeKind::Tuple => Ok(Some(StackFrame::Tuple {
                tag_name: tag_name.to_string(),
                items: Vec::new(),
                types: type_info.args.clone(),
            })),
            crate::python_types::PyTypeKind::Class => {
                let instance = if let Some(py_type) = &type_info.py_type {
                    if py_type
                        .as_ref(py)
                        .hasattr("__gasp_from_partial__")
                        .unwrap_or(false)
                    {
                        let empty_dict = pyo3::types::PyDict::new(py);
                        py_type
                            .as_ref(py)
                            .call_method1("__gasp_from_partial__", (empty_dict,))?
                    } else {
                        py_type.as_ref(py).call0()?
                    }
                } else {
                    return Err(pyo3::exceptions::PyTypeError::new_err(
                        "Cannot instantiate class without py_type",
                    ));
                };
                Ok(Some(StackFrame::Object {
                    type_info: type_info.clone(),
                    instance: instance.into(),
                    current_field: None,
                }))
            }
            crate::python_types::PyTypeKind::Union => {
                // For unions, we don't create a frame immediately.
                // We wait for a specific member tag to be identified.
                Ok(None)
            }
            _ => Err(pyo3::exceptions::PyTypeError::new_err(format!(
                "Cannot create frame for primitive type {:?}",
                type_info.kind
            ))),
        })?;
        if let Some(frame) = frame {
            self.stack.push(frame);
        }
        Ok(())
    }

    fn create_type_info_from_string(&self, type_str: &str) -> PyResult<PyTypeInfo> {
        let (kind, name) = match type_str {
            "int" => (crate::python_types::PyTypeKind::Integer, "int"),
            "str" | "string" => (crate::python_types::PyTypeKind::String, "str"),
            "float" => (crate::python_types::PyTypeKind::Float, "float"),
            "bool" | "boolean" => (crate::python_types::PyTypeKind::Boolean, "bool"),
            "list" => (crate::python_types::PyTypeKind::List, "list"),
            "dict" => (crate::python_types::PyTypeKind::Dict, "dict"),
            "set" => (crate::python_types::PyTypeKind::Set, "set"),
            "tuple" => (crate::python_types::PyTypeKind::Tuple, "tuple"),
            _ => {
                // Default to string for unknown types
                (crate::python_types::PyTypeKind::String, "str")
            }
        };

        let type_info = PyTypeInfo::new(kind, name.to_string()).with_module("builtins".to_string());

        Ok(type_info)
    }

    fn build_current_intermediate_state(&self) -> PyResult<Option<PyObject>> {
        if self.stack.is_empty() {
            return Ok(None);
        }

        let mut temp_stack = self.stack.clone();

        while temp_stack.len() > 1 {
            let top_frame = temp_stack.pop().unwrap();
            let py_object = self.frame_to_pyobject(top_frame)?;

            if let Some(parent_frame) = temp_stack.last_mut() {
                match parent_frame {
                    StackFrame::List { items, .. } => items.push(py_object),
                    StackFrame::Set { items, .. } => items.push(py_object),
                    StackFrame::Tuple { items, .. } => items.push(py_object),
                    StackFrame::Object {
                        instance,
                        current_field,
                        ..
                    } => {
                        if let Some(field_name) = current_field {
                            pyo3::Python::with_gil(|py| {
                                let _ = instance.as_ref(py).setattr(field_name.as_str(), py_object);
                            });
                        }
                    }
                    _ => {}
                }
            }
        }

        self.frame_to_pyobject(temp_stack[0].clone()).map(Some)
    }

    fn is_inside_container(&self) -> bool {
        // Check if the current context is directly inside a container (List, Set, Tuple, or Dict)
        if let Some(frame) = self.stack.last() {
            matches!(
                frame,
                StackFrame::List { .. }
                    | StackFrame::Set { .. }
                    | StackFrame::Tuple { .. }
                    | StackFrame::Dict { .. }
            )
        } else {
            false
        }
    }

    fn handle_stack_tag_open(&mut self, tag: &Tag) -> PyResult<()> {
        let tag_name = &tag.name;
        debug!(
            "handle_stack_tag_open: tag_name={}, stack_len={}",
            tag_name,
            self.stack.len()
        );

        // Debug: print current stack state
        for (i, frame) in self.stack.iter().enumerate() {
            match frame {
                StackFrame::Object {
                    type_info,
                    current_field,
                    ..
                } => {
                    debug!(
                        "  Stack[{}]: Object(type={}, current_field={:?})",
                        i, type_info.name, current_field
                    );
                }
                StackFrame::Field { name, .. } => {
                    debug!("  Stack[{}]: Field(name={})", i, name);
                }
                _ => {
                    debug!("  Stack[{}]: {:?}", i, frame);
                }
            }
        }

        // Determine what type of frame to create based on the current stack top.
        // This is done by peeking at the stack without a long-lived mutable borrow.
        let mut next_type_info = if let Some(frame) = self.stack.last() {
            match frame {
                StackFrame::Object { type_info, .. } => {
                    if let Some(field_info) = type_info.fields.get(tag_name) {
                        // If the field is Optional, convert it to Union[T, None]
                        let field_info =
                            if field_info.kind == crate::python_types::PyTypeKind::Optional {
                                // Convert Optional[T] to Union[T, None]
                                let inner_type = field_info
                                    .args
                                    .get(0)
                                    .cloned()
                                    .unwrap_or_else(|| PyTypeInfo::any());
                                let none_type = PyTypeInfo::new(
                                    crate::python_types::PyTypeKind::None,
                                    "None".to_string(),
                                )
                                .with_module("builtins".to_string());

                                PyTypeInfo::new(
                                    crate::python_types::PyTypeKind::Union,
                                    "Union".to_string(),
                                )
                                .with_module("typing".to_string())
                                .with_args(vec![inner_type, none_type])
                            } else {
                                field_info.clone()
                            };

                        // If the field is a union type, check the type attribute
                        if field_info.kind == crate::python_types::PyTypeKind::Union {
                            if let Some(type_attr) = tag.attributes.get("type") {
                                field_info
                                    .args
                                    .iter()
                                    .find(|t| &t.name == type_attr)
                                    .cloned()
                                    .or(Some(field_info))
                            } else {
                                Some(field_info)
                            }
                        } else {
                            Some(field_info)
                        }
                    } else if type_info.kind == crate::python_types::PyTypeKind::Union {
                        type_info.args.iter().find(|t| t.name == *tag_name).cloned()
                    } else {
                        None
                    }
                }
                StackFrame::List { item_type, .. } if tag_name == "item" => {
                    // Check if the item_type is a Union and if so, use the type attribute
                    if item_type.kind == crate::python_types::PyTypeKind::Union {
                        if let Some(type_attr) = tag.attributes.get("type") {
                            item_type
                                .args
                                .iter()
                                .find(|t| &t.name == type_attr)
                                .cloned()
                        } else {
                            Some(item_type.clone())
                        }
                    } else {
                        Some(item_type.clone())
                    }
                }
                StackFrame::Set { item_type, .. } if tag_name == "item" => {
                    // Check if the item_type is a Union and if so, use the type attribute
                    if item_type.kind == crate::python_types::PyTypeKind::Union {
                        if let Some(type_attr) = tag.attributes.get("type") {
                            item_type
                                .args
                                .iter()
                                .find(|t| &t.name == type_attr)
                                .cloned()
                        } else {
                            Some(item_type.clone())
                        }
                    } else {
                        Some(item_type.clone())
                    }
                }
                StackFrame::Tuple { items, types, .. } if tag_name == "item" => {
                    // Check if this is a homogeneous tuple (Tuple[T, ...])
                    if types.len() == 2
                        && types.get(1).map(|t| t.name == "Ellipsis").unwrap_or(false)
                    {
                        // For homogeneous tuples, always use the first type
                        types.get(0).cloned()
                    } else {
                        // For fixed tuples, get the type for the current position
                        types.get(items.len()).cloned()
                    }
                }
                StackFrame::Dict { value_type, .. } if tag_name == "item" => value_type.clone(),
                _ => None,
            }
        } else {
            // Stack is empty, this is the root.
            self.type_info.clone()
        };

        // If we don't have type info or the type is Any, and we're handling an item in a container,
        // check if the tag has a type attribute we can use
        if tag_name == "item" && self.is_inside_container() {
            if next_type_info.is_none()
                || (next_type_info
                    .as_ref()
                    .map(|t| t.kind == crate::python_types::PyTypeKind::Any)
                    .unwrap_or(false))
            {
                if let Some(type_attr) = tag.attributes.get("type") {
                    // Create PyTypeInfo based on the type attribute
                    next_type_info = Some(self.create_type_info_from_string(type_attr)?);
                }
            }
        }

        let mut pushed_new_frame = false;
        if let Some(type_info) = next_type_info {
            let mut should_push = false;
            let actual_type = if type_info.kind == crate::python_types::PyTypeKind::Union {
                // For union types, check the type attribute to determine which member to use
                if let Some(type_attr) = tag.attributes.get("type") {
                    type_info
                        .args
                        .iter()
                        .find(|t| &t.name == type_attr)
                        .cloned()
                        .unwrap_or(type_info.clone())
                } else if type_info.args.iter().any(|t| t.name == *tag_name) {
                    // If no type attribute but tag name matches a union member, use that
                    type_info
                        .args
                        .iter()
                        .find(|t| t.name == *tag_name)
                        .cloned()
                        .unwrap_or(type_info.clone())
                } else {
                    type_info.clone()
                }
            } else {
                type_info.clone()
            };

            if self.stack.is_empty() {
                // For root level, check if the tag matches the expected type (case-insensitive)
                let type_name_lower = actual_type.name.to_lowercase();
                let tag_name_lower = tag_name.to_lowercase();
                if type_name_lower == tag_name_lower
                    || (tag_name_lower == "list"
                        && actual_type.kind == crate::python_types::PyTypeKind::List)
                {
                    should_push = true;
                }
            } else {
                // For nested tags (like items in a list), always push if we have a type
                should_push = true;
            }

            if should_push {
                // Push the new frame.
                if actual_type.is_primitive() {
                    self.stack.push(StackFrame::Field {
                        name: tag_name.clone(),
                        content: String::new(),
                        type_info: actual_type,
                    });
                    pushed_new_frame = true;
                } else {
                    self.push_frame_for_type(&actual_type, tag_name)?;
                    pushed_new_frame = true;
                }
            }
        }

        // If we pushed a new frame, we need to update the parent frame's context
        if pushed_new_frame && self.stack.len() > 1 {
            // Look at the frame right before the one we just pushed (the parent)
            let parent_idx = self.stack.len() - 2;
            match self.stack.get_mut(parent_idx) {
                Some(StackFrame::Object {
                    current_field,
                    type_info,
                    ..
                }) => {
                    // Only set current_field if this tag corresponds to a field of the object
                    if type_info.fields.contains_key(tag_name) {
                        debug!(
                            "Setting current_field '{}' on Object frame at index {}",
                            tag_name, parent_idx
                        );
                        *current_field = Some(tag_name.clone());
                    }
                }
                Some(StackFrame::Dict { current_key, .. }) if tag_name == "item" => {
                    // For dict items, store the key from the tag attributes
                    if let Some(key_attr) = tag.attributes.get("key") {
                        pyo3::Python::with_gil(|py| {
                            *current_key = Some(key_attr.clone().into_py(py));
                        });
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn handle_stack_tag_close(&mut self, tag_name: &str) -> PyResult<()> {
        debug!("handle_stack_tag_close: tag_name={}", tag_name);

        // Debug: print current stack state before processing
        debug!("Stack state before processing close tag:");
        for (i, frame) in self.stack.iter().enumerate() {
            match frame {
                StackFrame::Object {
                    type_info,
                    current_field,
                    ..
                } => {
                    debug!(
                        "  Stack[{}]: Object(type={}, current_field={:?})",
                        i, type_info.name, current_field
                    );
                }
                StackFrame::Field { name, .. } => {
                    debug!("  Stack[{}]: Field(name={})", i, name);
                }
                _ => {
                    debug!("  Stack[{}]: {:?}", i, frame);
                }
            }
        }

        let mut child_object = None;

        // Check if the closing tag matches the top of the stack (case-insensitive).
        let tag_name_lower = tag_name.to_lowercase();
        let should_pop = if let Some(top_frame) = self.stack.last() {
            match top_frame {
                StackFrame::Field { name, .. } => name.to_lowercase() == tag_name_lower,
                StackFrame::Object { type_info, .. } => {
                    // For objects, check if the tag matches the type name
                    // OR if this is a field closing tag and the parent has that field
                    type_info.name.to_lowercase() == tag_name_lower
                        || (self.stack.len() > 1
                            && match &self.stack[self.stack.len() - 2] {
                                StackFrame::Object {
                                    type_info: parent_type,
                                    ..
                                } => parent_type.fields.contains_key(tag_name),
                                StackFrame::List { .. }
                                | StackFrame::Set { .. }
                                | StackFrame::Tuple { .. } => tag_name_lower == "item",
                                _ => false,
                            })
                }
                StackFrame::List {
                    tag_name: list_tag, ..
                } => list_tag.to_lowercase() == tag_name_lower,
                StackFrame::Set {
                    tag_name: set_tag, ..
                } => set_tag.to_lowercase() == tag_name_lower,
                StackFrame::Tuple {
                    tag_name: tuple_tag,
                    ..
                } => tuple_tag.to_lowercase() == tag_name_lower,
                StackFrame::Dict {
                    tag_name: dict_tag, ..
                } => dict_tag.to_lowercase() == tag_name_lower,
            }
        } else {
            false
        };

        if should_pop {
            let child_frame = self.stack.pop().unwrap();
            child_object = Some(self.frame_to_pyobject(child_frame)?);
        }

        // If we popped a frame and got an object, we need to add it to its parent.
        if let Some(obj) = child_object {
            if let Some(parent_frame) = self.stack.last_mut() {
                match parent_frame {
                    StackFrame::List { items, .. } => items.push(obj),
                    StackFrame::Set { items, .. } => items.push(obj),
                    StackFrame::Tuple { items, .. } => items.push(obj),
                    StackFrame::Dict {
                        entries,
                        current_key,
                        ..
                    } => {
                        // For dict items, use the current_key that was stored when the item was opened
                        if let Some(key) = current_key.take() {
                            entries.push((key, obj));
                        } else {
                            debug!("Dict item closed but no key was stored");
                        }
                    }
                    StackFrame::Object {
                        instance,
                        current_field,
                        ..
                    } => {
                        // Use the `current_field` that was set when the tag was opened.
                        if let Some(field_name) = current_field.take() {
                            debug!("Setting field '{}' on object", field_name);
                            pyo3::Python::with_gil(|py| {
                                let _ = instance.as_ref(py).setattr(field_name.as_str(), obj);
                            });
                        }
                    }
                    _ => {}
                }
            } else {
                // No parent, this is the root object.
                self.stack_based_result = Some(obj);
                self.is_done = true;
            }
        } else if self.stack.len() == 1 {
            // This handles the case where the root object itself is closing.
            let top_frame_name = if let Some(frame) = self.stack.last() {
                match frame {
                    StackFrame::Object { type_info, .. } => Some(type_info.name.clone()),
                    _ => None,
                }
            } else {
                None
            };

            if let Some(frame_name) = top_frame_name {
                if frame_name.to_lowercase() == tag_name.to_lowercase() {
                    if let Some(frame) = self.stack.pop() {
                        self.stack_based_result = Some(self.frame_to_pyobject(frame)?);
                        self.is_done = true;
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_stack_bytes(&mut self, content: &str) -> PyResult<()> {
        if let Some(StackFrame::Field {
            content: field_content,
            ..
        }) = self.stack.last_mut()
        {
            field_content.push_str(content);
        }
        Ok(())
    }

    pub fn step(&mut self, chunk: &str) -> PyResult<Option<PyObject>> {
        let mut events = Vec::new();
        let events_ref = &mut events;
        self.tag_finder
            .push(chunk, |event| {
                debug!("Callback received event: {:?}", event);
                events_ref.push(event);
                Ok(())
            })
            .map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Tag parsing error: {:?}", e))
            })?;

        debug!("step: chunk={:?}, collected events={:?}", chunk, events);

        if self.should_use_stack() {
            for event in &events {
                match event {
                    crate::tag_finder::TagEvent::Open(tag) => self.handle_stack_tag_open(tag)?,
                    crate::tag_finder::TagEvent::Close(name) => {
                        self.handle_stack_tag_close(name)?
                    }
                    crate::tag_finder::TagEvent::Bytes(content) => {
                        self.handle_stack_bytes(content)?
                    }
                }
            }
            if self.is_done {
                return Ok(self.stack_based_result.clone());
            }
            return self.build_current_intermediate_state();
        }

        // Handle primitive types that don't use the stack
        if let Some(type_info) = &self.type_info {
            if type_info.is_primitive() {
                // For primitive types, we need a simple tag + content structure
                for event in &events {
                    match event {
                        crate::tag_finder::TagEvent::Open(tag) => {
                            if tag.name.to_lowercase() == type_info.name.to_lowercase()
                                && self.stack.is_empty()
                            {
                                // Start collecting content for this primitive
                                self.stack.push(StackFrame::Field {
                                    name: tag.name.clone(),
                                    content: String::new(),
                                    type_info: type_info.clone(),
                                });
                            }
                        }
                        crate::tag_finder::TagEvent::Bytes(content) => {
                            if let Some(StackFrame::Field {
                                content: field_content,
                                ..
                            }) = self.stack.last_mut()
                            {
                                field_content.push_str(content);
                            }
                        }
                        crate::tag_finder::TagEvent::Close(name) => {
                            if name.to_lowercase() == type_info.name.to_lowercase()
                                && !self.stack.is_empty()
                            {
                                if let Some(frame) = self.stack.pop() {
                                    let result = self.frame_to_pyobject(frame)?;
                                    self.stack_based_result = Some(result.clone());
                                    self.is_done = true;
                                    return Ok(Some(result));
                                }
                            }
                        }
                    }
                }

                // Return partial results for primitives
                if !self.stack.is_empty() {
                    if let Some(StackFrame::Field {
                        content, type_info, ..
                    }) = self.stack.last()
                    {
                        // Build a partial result from the current content
                        let partial = self.frame_to_pyobject(StackFrame::Field {
                            name: type_info.name.clone(),
                            content: content.clone(),
                            type_info: type_info.clone(),
                        })?;
                        return Ok(Some(partial));
                    }
                }
            }
        }

        Ok(None)
    }

    pub fn is_done(&self) -> bool {
        self.is_done
    }
}

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

                let wanted_tags = match type_info.kind {
                    crate::python_types::PyTypeKind::Union => {
                        // For unions, collect all member type names
                        let mut tags: Vec<String> =
                            type_info.args.iter().map(|arg| arg.name.clone()).collect();
                        tags.push(type_info.name.clone());

                        // Also add lowercase versions to handle case-insensitive matching
                        let lowercase_tags: Vec<String> =
                            tags.iter().map(|s| s.to_lowercase()).collect();
                        tags.extend(lowercase_tags);
                        tags.sort();
                        tags.dedup();
                        tags
                    }
                    _ => {
                        // For non-union types, include both original and lowercase versions
                        let mut tags = vec![type_info.name.clone()];
                        let lowercase = type_info.name.to_lowercase();
                        if lowercase != type_info.name {
                            tags.push(lowercase);
                        }
                        tags
                    }
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

    #[staticmethod]
    #[pyo3(text_signature = "(pydantic_model)")]
    fn from_pydantic(py: Python, pydantic_model: &PyAny) -> PyResult<Self> {
        let mut type_info = PyTypeInfo::extract_from_python(pydantic_model)?;
        if type_info.py_type.is_none() {
            type_info.py_type = Some(pydantic_model.into_py(py));
        }

        // Include both original and lowercase versions for case-insensitive matching
        let mut wanted_tags = vec![type_info.name.clone()];
        let lowercase = type_info.name.to_lowercase();
        if lowercase != type_info.name {
            wanted_tags.push(lowercase);
        }
        let typed_stream_parser = TypedStreamParser::with_type(type_info, wanted_tags, Vec::new());

        Ok(Self {
            parser: typed_stream_parser,
            result: None,
        })
    }

    #[pyo3(text_signature = "($self, chunk)")]
    fn feed(&mut self, _py: Python, chunk: &str) -> PyResult<Option<PyObject>> {
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
    fn get_partial(&mut self, _py: Python) -> PyResult<Option<PyObject>> {
        Ok(self.result.clone())
    }

    #[pyo3(text_signature = "($self)")]
    fn validate(&mut self, _py: Python) -> PyResult<Option<PyObject>> {
        self.get_partial(_py)
    }
}
