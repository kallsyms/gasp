use log::debug;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple, PyType, PyUnicode};
use std::collections::HashMap;
use xml::Event;

use crate::xml_types::XmlValue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PyTypeKind {
    String,
    Integer,
    Float,
    Boolean,
    Dict,
    List,
    Tuple,
    Union,
    Class,
    Any,
    None,
    Optional,
}

#[derive(Debug, Clone)]
pub struct PyTypeInfo {
    pub kind: PyTypeKind,
    pub name: String,
    pub module: Option<String>,
    pub origin: Option<String>,
    pub args: Vec<PyTypeInfo>,
    pub fields: HashMap<String, PyTypeInfo>,
    pub is_optional: bool,
    pub py_type: Option<Py<PyAny>>, // Store the original Python type object
}

impl PyTypeInfo {
    pub fn new(kind: PyTypeKind, name: String) -> Self {
        Self {
            kind,
            name,
            module: None,
            origin: None,
            args: Vec::new(),
            fields: HashMap::new(),
            is_optional: false,
            py_type: None,
        }
    }

    pub fn with_module(mut self, module: String) -> Self {
        self.module = Some(module);
        self
    }

    pub fn with_py_type(mut self, py_type: Py<PyAny>) -> Self {
        self.py_type = Some(py_type);
        self
    }

    pub fn with_origin(mut self, origin: String) -> Self {
        self.origin = Some(origin);
        self
    }

    pub fn with_args(mut self, args: Vec<PyTypeInfo>) -> Self {
        self.args = args;
        self
    }

    pub fn with_fields(mut self, fields: HashMap<String, PyTypeInfo>) -> Self {
        self.fields = fields;
        self
    }

    pub fn with_optional(mut self, is_optional: bool) -> Self {
        self.is_optional = is_optional;
        self
    }

    /// Check if a XmlValue matches this type
    pub fn matches(&self, value: &XmlValue) -> bool {
        match (&self.kind, value) {
            (PyTypeKind::Any, _) => true,
            (PyTypeKind::String, XmlValue::Text(_)) => true,
            (PyTypeKind::Integer, XmlValue::Text(s)) => s.parse::<i64>().is_ok(),
            (PyTypeKind::Float, XmlValue::Text(s)) => s.parse::<f64>().is_ok(),
            (PyTypeKind::Boolean, XmlValue::Text(s)) => s.parse::<bool>().is_ok(),
            (PyTypeKind::List, XmlValue::Element(_, _, children)) => {
                if self.args.is_empty() {
                    true
                } else {
                    children.iter().all(|item| self.args[0].matches(item))
                }
            }
            (PyTypeKind::Class, XmlValue::Element(_, _, _)) => true,
            _ => false,
        }
    }

    pub fn extract_from_python(py_type: &PyAny) -> PyResult<Self> {
        // Store reference to the original Python type
        let py_type_ref = py_type.into_py(py_type.py());

        // Check if this is a type alias (created with 'type' statement)
        if let Ok(value_attr) = py_type.getattr("__value__") {
            // Check if the __value__ is a Union type
            if let Ok(origin) = value_attr.getattr("__origin__") {
                let origin_str = origin.str()?.extract::<String>()?;
                if origin_str == "typing.Union" || origin_str.ends_with(".Union") {
                    // This is a union type alias
                    // Get the alias name
                    let alias_name = if let Ok(name) = py_type.getattr("__name__") {
                        name.extract::<String>()?
                    } else {
                        "Union".to_string()
                    };

                    // Extract the Union type arguments
                    let type_args = if let Ok(args) = value_attr.getattr("__args__") {
                        let args_seq = args.extract::<Vec<&PyAny>>()?;
                        let mut type_infos = Vec::new();
                        for arg in args_seq {
                            type_infos.push(PyTypeInfo::extract_from_python(arg)?);
                        }
                        type_infos
                    } else {
                        Vec::new()
                    };

                    // Return as a Union type with the alias name
                    return Ok(PyTypeInfo::new(PyTypeKind::Union, alias_name)
                        .with_module("typing".to_string())
                        .with_origin("Union".to_string())
                        .with_args(type_args)
                        .with_py_type(py_type_ref));
                }
            }
        }

        // Get type name
        let type_name = if let Ok(name) = py_type.getattr("__name__") {
            name.extract::<String>()?
        } else {
            // For typing objects like List, Optional, etc.
            let repr = py_type.repr()?.extract::<String>()?;
            if let Some(idx) = repr.find('[') {
                repr[..idx].trim().to_string()
            } else {
                repr
            }
        };

        // Get module name
        let module_name = if let Ok(module) = py_type.getattr("__module__") {
            Some(module.extract::<String>()?)
        } else {
            None
        };

        // Special handling for typing constructs that might not have __origin__ in some Python versions
        // Check the string representation first
        let repr_str = py_type.repr()?.extract::<String>()?;
        if repr_str.starts_with("typing.List[") || repr_str.starts_with("list[") {
            // Get type arguments
            let type_args = if let Ok(args) = py_type.getattr("__args__") {
                let args_seq = args.extract::<Vec<&PyAny>>()?;
                let mut type_infos = Vec::new();
                for arg in args_seq {
                    type_infos.push(PyTypeInfo::extract_from_python(arg)?);
                }
                type_infos
            } else {
                Vec::new()
            };

            return Ok(PyTypeInfo::new(PyTypeKind::List, "list".to_string())
                .with_module("typing".to_string())
                .with_origin("list".to_string())
                .with_args(type_args)
                .with_py_type(py_type_ref));
        }

        if repr_str.starts_with("typing.Tuple[") || repr_str.starts_with("tuple[") {
            // Get type arguments
            let type_args = if let Ok(args) = py_type.getattr("__args__") {
                let args_seq = args.extract::<Vec<&PyAny>>()?;
                let mut type_infos = Vec::new();
                for arg in args_seq {
                    type_infos.push(PyTypeInfo::extract_from_python(arg)?);
                }
                type_infos
            } else {
                Vec::new()
            };

            return Ok(PyTypeInfo::new(PyTypeKind::Tuple, "tuple".to_string())
                .with_module("typing".to_string())
                .with_origin("tuple".to_string())
                .with_args(type_args)
                .with_py_type(py_type_ref));
        }

        // Check if this is a typing module construct (List, Dict, Optional, etc.)
        if let Ok(origin) = py_type.getattr("__origin__") {
            let origin_name = origin.str()?.extract::<String>()?;

            // Get type arguments
            let type_args = if let Ok(args) = py_type.getattr("__args__") {
                let args_seq = args.extract::<Vec<&PyAny>>()?;
                let mut type_infos = Vec::new();
                for arg in args_seq {
                    type_infos.push(PyTypeInfo::extract_from_python(arg)?);
                }
                type_infos
            } else {
                Vec::new()
            };

            // Handle specific typing constructs
            // Strip module prefix if present
            let base_origin = if let Some(pos) = origin_name.rfind('.') {
                &origin_name[pos + 1..]
            } else {
                &origin_name
            };

            match base_origin {
                "list" => {
                    return Ok(PyTypeInfo::new(PyTypeKind::List, "list".to_string())
                        .with_module(module_name.unwrap_or_else(|| "builtins".to_string()))
                        .with_origin(origin_name)
                        .with_args(type_args)
                        .with_py_type(py_type_ref));
                }
                "tuple" => {
                    return Ok(PyTypeInfo::new(PyTypeKind::Tuple, "tuple".to_string())
                        .with_module(module_name.unwrap_or_else(|| "builtins".to_string()))
                        .with_origin(origin_name)
                        .with_args(type_args)
                        .with_py_type(py_type_ref));
                }
                "dict" => {
                    return Ok(PyTypeInfo::new(PyTypeKind::Dict, "dict".to_string())
                        .with_module(module_name.unwrap_or_else(|| "builtins".to_string()))
                        .with_origin(origin_name)
                        .with_args(type_args)
                        .with_py_type(py_type_ref));
                }
                "Union" => {
                    // Get type arguments with proper py_type references
                    let type_args = if let Ok(args) = py_type.getattr("__args__") {
                        let args_seq = args.extract::<Vec<&PyAny>>()?;
                        let mut type_infos = Vec::new();
                        for arg in args_seq {
                            // Extract each arg with its proper py_type reference
                            let mut arg_info = PyTypeInfo::extract_from_python(arg)?;
                            // Ensure the py_type is set
                            if arg_info.py_type.is_none() {
                                arg_info.py_type = Some(arg.into_py(py_type.py()));
                            }
                            type_infos.push(arg_info);
                        }
                        type_infos
                    } else {
                        Vec::new()
                    };

                    // Check if this is Optional (Union[T, None])
                    let is_optional = type_args.iter().any(|arg| arg.kind == PyTypeKind::None);

                    if is_optional && type_args.len() == 2 {
                        // Find the non-None type
                        let non_none_type = type_args
                            .iter()
                            .find(|arg| arg.kind != PyTypeKind::None)
                            .cloned()
                            .unwrap_or_else(|| PyTypeInfo::new(PyTypeKind::Any, "Any".to_string()));

                        return Ok(
                            PyTypeInfo::new(PyTypeKind::Optional, "Optional".to_string())
                                .with_module(module_name.unwrap_or_else(|| "typing".to_string()))
                                .with_origin(origin_name.clone())
                                .with_args(vec![non_none_type])
                                .with_optional(true)
                                .with_py_type(py_type_ref),
                        );
                    } else {
                        return Ok(PyTypeInfo::new(PyTypeKind::Union, "Union".to_string())
                            .with_module(module_name.unwrap_or_else(|| "typing".to_string()))
                            .with_origin(origin_name.clone())
                            .with_args(type_args)
                            .with_py_type(py_type_ref));
                    }
                }
                _ => {
                    // Generic container type
                    return Ok(PyTypeInfo::new(PyTypeKind::Any, origin_name.clone())
                        .with_module(module_name.unwrap_or_else(|| "typing".to_string()))
                        .with_origin(origin_name)
                        .with_args(type_args)
                        .with_py_type(py_type_ref));
                }
            }
        }

        // Handle built-in types
        match type_name.as_str() {
            "str" => {
                return Ok(PyTypeInfo::new(PyTypeKind::String, "str".to_string())
                    .with_module(module_name.unwrap_or_else(|| "builtins".to_string()))
                    .with_py_type(py_type_ref));
            }
            "int" => {
                return Ok(PyTypeInfo::new(PyTypeKind::Integer, "int".to_string())
                    .with_module(module_name.unwrap_or_else(|| "builtins".to_string()))
                    .with_py_type(py_type_ref));
            }
            "float" => {
                return Ok(PyTypeInfo::new(PyTypeKind::Float, "float".to_string())
                    .with_module(module_name.unwrap_or_else(|| "builtins".to_string()))
                    .with_py_type(py_type_ref));
            }
            "bool" => {
                return Ok(PyTypeInfo::new(PyTypeKind::Boolean, "bool".to_string())
                    .with_module(module_name.unwrap_or_else(|| "builtins".to_string()))
                    .with_py_type(py_type_ref));
            }
            "list" => {
                return Ok(PyTypeInfo::new(PyTypeKind::List, "list".to_string())
                    .with_module(module_name.unwrap_or_else(|| "builtins".to_string()))
                    .with_py_type(py_type_ref));
            }
            "dict" => {
                return Ok(PyTypeInfo::new(PyTypeKind::Dict, "dict".to_string())
                    .with_module(module_name.unwrap_or_else(|| "builtins".to_string()))
                    .with_py_type(py_type_ref));
            }
            "NoneType" => {
                return Ok(PyTypeInfo::new(PyTypeKind::None, "None".to_string())
                    .with_module(module_name.unwrap_or_else(|| "builtins".to_string()))
                    .with_py_type(py_type_ref));
            }
            "type" => {
                // This is a class type
                return Ok(PyTypeInfo::new(PyTypeKind::Class, type_name)
                    .with_module(module_name.unwrap_or_else(|| "builtins".to_string()))
                    .with_py_type(py_type_ref));
            }
            _ => {
                // Check if this is a class by seeing if it has __annotations__
                if let Ok(annotations) = py_type.getattr("__annotations__") {
                    let annotations_dict = annotations.downcast::<PyDict>()?;
                    let mut fields = HashMap::new();

                    for (key, value) in annotations_dict.iter() {
                        let field_name = key.extract::<String>()?;
                        let field_type = PyTypeInfo::extract_from_python(value)?;
                        fields.insert(field_name, field_type);
                    }

                    return Ok(PyTypeInfo::new(PyTypeKind::Class, type_name)
                        .with_module(module_name.unwrap_or_else(|| "builtins".to_string()))
                        .with_fields(fields)
                        .with_py_type(py_type_ref));
                } else if let Ok(_bases) = py_type.getattr("__bases__") {
                    // This is likely a class type too
                    return Ok(PyTypeInfo::new(PyTypeKind::Class, type_name)
                        .with_module(module_name.unwrap_or_else(|| "builtins".to_string()))
                        .with_py_type(py_type_ref));
                }

                // Fallback to Any
                return Ok(PyTypeInfo::new(PyTypeKind::Any, type_name)
                    .with_module(module_name.unwrap_or_else(|| "builtins".to_string()))
                    .with_py_type(py_type_ref));
            }
        }
    }
}

/// Convert a XmlValue to a Python object based on type info
pub fn xml_to_python(
    py: Python,
    value: &XmlValue,
    type_info: Option<&PyTypeInfo>,
) -> PyResult<PyObject> {
    debug!(
        "xml_to_python called with value: {:?} and type_info: {:?}",
        value,
        type_info.map(|ti| &ti.name)
    );
    match value {
        XmlValue::Element(name, attrs, children) => {
            if let Some(ti) = type_info {
                match ti.kind {
                    PyTypeKind::Class => {
                        debug!("Type is class, calling create_instance_from_xml");
                        let class_obj = ti
                            .py_type
                            .as_ref()
                            .ok_or_else(|| {
                                PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                                    "Type has no associated Python class",
                                )
                            })?
                            .as_ref(py);
                        return create_instance_from_xml(
                            py, class_obj, name, attrs, children, &ti.fields,
                        );
                    }
                    PyTypeKind::List => {
                        debug!("Type is list");
                        let list = PyList::empty(py);
                        let element_type = ti.args.get(0);
                        for item in children {
                            list.append(xml_to_python(py, item, element_type)?)?;
                        }
                        return Ok(list.into());
                    }
                    _ => {}
                }
            }
            // Fallback for untyped or mismatched types
            debug!("Fallback to dict");
            let dict = PyDict::new(py);
            dict.set_item("name", name.clone())?;
            let py_attrs = PyDict::new(py);
            for (k, v) in attrs {
                py_attrs.set_item(k, v.clone())?;
            }
            dict.set_item("attrs", py_attrs)?;
            let py_children = PyList::empty(py);
            for child in children {
                py_children.append(xml_to_python(py, child, None)?)?;
            }
            dict.set_item("children", py_children)?;
            Ok(dict.into())
        }
        XmlValue::Text(s) => {
            if let Some(ti) = type_info {
                match ti.kind {
                    PyTypeKind::Integer => return Ok(s.parse::<i64>().unwrap_or(0).into_py(py)),
                    PyTypeKind::Float => return Ok(s.parse::<f64>().unwrap_or(0.0).into_py(py)),
                    PyTypeKind::Boolean => {
                        return Ok(s.parse::<bool>().unwrap_or(false).into_py(py))
                    }
                    _ => {}
                }
            }
            Ok(s.to_string().into_py(py))
        }
    }
}

// Creates a Python instance from a XML map with proper type conversion
pub fn create_instance_from_xml(
    py: Python,
    py_type: &PyAny,
    name: &str,
    attrs: &HashMap<String, String>,
    children: &Vec<XmlValue>,
    fields: &HashMap<String, PyTypeInfo>,
) -> PyResult<PyObject> {
    debug!(
        "Creating instance of type: {}",
        py_type.getattr("__name__")?.to_string()
    );
    let instance = py_type.call0()?;
    for (k, v) in attrs {
        if let Some(field_info) = fields.get(k) {
            debug!("Found field '{}' in attributes", k);
            let py_value = xml_to_python(py, &XmlValue::Text(v.clone()), Some(field_info))?;
            instance.setattr(k.as_str(), py_value)?;
        } else {
            instance.setattr(k.as_str(), v)?;
        }
    }

    for child in children {
        if let XmlValue::Element(child_name, child_attrs, grand_children) = child {
            if let Some(field_info) = fields.get(child_name) {
                debug!("Found field '{}' in children", child_name);
                if field_info.kind == PyTypeKind::List {
                    let list = PyList::empty(py);
                    for grand_child in grand_children {
                        if let XmlValue::Element(_, _, text_children) = grand_child {
                            if text_children.len() == 1 {
                                if let Some(XmlValue::Text(text)) = text_children.get(0) {
                                    let py_value = xml_to_python(
                                        py,
                                        &XmlValue::Text(text.clone()),
                                        field_info.args.get(0),
                                    )?;
                                    list.append(py_value)?;
                                }
                            }
                        }
                    }
                    instance.setattr(child_name.as_str(), list)?;
                } else if grand_children.len() == 1 {
                    if let Some(XmlValue::Text(text)) = grand_children.get(0) {
                        let py_value =
                            xml_to_python(py, &XmlValue::Text(text.clone()), Some(field_info))?;
                        instance.setattr(child_name.as_str(), py_value)?;
                    }
                } else {
                    let py_value = xml_to_python(py, child, Some(field_info))?;
                    instance.setattr(child_name.as_str(), py_value)?;
                }
            }
        }
    }

    Ok(instance.into())
}

pub fn create_instance_from_xml_events(
    py: Python,
    type_info: &PyTypeInfo,
    events: Vec<Result<Event, crate::xml_types::XmlError>>,
) -> PyResult<Py<PyAny>> {
    let instance = type_info.py_type.as_ref().unwrap().as_ref(py).call0()?;
    let mut current_field: Option<String> = None;

    for event in events {
        match event? {
            Event::ElementStart(tag) => {
                current_field = Some(tag.name.clone());
            }
            Event::Characters(text) => {
                if let Some(field_name) = &current_field {
                    if let Some(field_info) = type_info.fields.get(field_name) {
                        let py_value =
                            xml_to_python(py, &XmlValue::Text(text.to_string()), Some(field_info))?;
                        instance.setattr(field_name.as_str(), py_value)?;
                    }
                }
            }
            _ => {}
        }
    }

    Ok(instance.into())
}
