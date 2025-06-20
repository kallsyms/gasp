use log::debug;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
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
    Set,
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
    pub fn any() -> Self {
        Self {
            kind: PyTypeKind::Any,
            name: "Any".to_string(),
            module: Some("typing".to_string()),
            origin: Some("Any".to_string()),
            args: Vec::new(),
            fields: HashMap::new(),
            is_optional: false,
            py_type: None,
        }
    }

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

    pub fn is_primitive(&self) -> bool {
        matches!(
            self.kind,
            PyTypeKind::String
                | PyTypeKind::Integer
                | PyTypeKind::Float
                | PyTypeKind::Boolean
                | PyTypeKind::None
        )
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
        debug!("extract_from_python: py_type = {:?}", py_type.repr()?);

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
            let extracted_name = name.extract::<String>()?;
            debug!("Extracted __name__: {}", extracted_name);
            extracted_name
        } else {
            // For typing objects like List, Optional, etc.
            let repr = py_type.repr()?.extract::<String>()?;
            debug!("No __name__, using repr: {}", repr);
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

            // For List types, use the builtin list type, not the typing construct
            let list_type = py_type.py().eval("list", None, None)?;
            return Ok(PyTypeInfo::new(PyTypeKind::List, "list".to_string())
                .with_module("typing".to_string())
                .with_origin("list".to_string())
                .with_args(type_args)
                .with_py_type(list_type.into_py(py_type.py())));
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
            let origin_repr = origin.repr()?.extract::<String>()?;
            debug!("Origin repr: {}", origin_repr);

            // Extract the actual type name from the origin
            let origin_name = if let Ok(name) = origin.getattr("__name__") {
                name.extract::<String>()?
            } else {
                origin.str()?.extract::<String>()?
            };
            debug!("Origin name: {}", origin_name);

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
                    // For List types, use the builtin list type, not the typing construct
                    let list_type = py_type.py().eval("list", None, None)?;
                    return Ok(PyTypeInfo::new(PyTypeKind::List, "list".to_string())
                        .with_module(module_name.unwrap_or_else(|| "builtins".to_string()))
                        .with_origin(origin_name)
                        .with_args(type_args)
                        .with_py_type(list_type.into_py(py_type.py())));
                }
                "tuple" => {
                    return Ok(PyTypeInfo::new(PyTypeKind::Tuple, "tuple".to_string())
                        .with_module(module_name.unwrap_or_else(|| "builtins".to_string()))
                        .with_origin(origin_name)
                        .with_args(type_args)
                        .with_py_type(py_type_ref));
                }
                "dict" => {
                    debug!("Creating Dict type with origin: {}", origin_name);
                    return Ok(PyTypeInfo::new(PyTypeKind::Dict, "dict".to_string())
                        .with_module(module_name.unwrap_or_else(|| "builtins".to_string()))
                        .with_origin(origin_name)
                        .with_args(type_args)
                        .with_py_type(py_type_ref));
                }
                "set" => {
                    debug!("Creating Set type with origin: {}", origin_name);
                    return Ok(PyTypeInfo::new(PyTypeKind::Set, "set".to_string())
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
            "tuple" => {
                return Ok(PyTypeInfo::new(PyTypeKind::Tuple, "tuple".to_string())
                    .with_module(module_name.unwrap_or_else(|| "builtins".to_string()))
                    .with_py_type(py_type_ref));
            }
            "set" => {
                return Ok(PyTypeInfo::new(PyTypeKind::Set, "set".to_string())
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

                        for child in children {
                            if let XmlValue::Element(item_name, item_attrs, item_children) = child {
                                if item_name == "item" && item_children.len() == 1 {
                                    if let Some(XmlValue::Text(text)) = item_children.get(0) {
                                        let py_value = if let Some(elem_type) = element_type {
                                            // Use the typed argument
                                            xml_to_python(
                                                py,
                                                &XmlValue::Text(text.clone()),
                                                Some(elem_type),
                                            )?
                                        } else if let Some(type_attr) = item_attrs.get("type") {
                                            // Use type attribute if no args specified
                                            match type_attr.as_str() {
                                                "int" => {
                                                    text.parse::<i64>().unwrap_or(0).into_py(py)
                                                }
                                                "float" => {
                                                    text.parse::<f64>().unwrap_or(0.0).into_py(py)
                                                }
                                                "bool" | "boolean" => {
                                                    let val = match text.to_lowercase().as_str() {
                                                        "true" | "1" | "yes" => true,
                                                        "false" | "0" | "no" => false,
                                                        _ => false,
                                                    };
                                                    val.into_py(py)
                                                }
                                                "str" | "string" => text.into_py(py),
                                                _ => text.into_py(py), // Default to string
                                            }
                                        } else {
                                            // No type info, default to string
                                            text.into_py(py)
                                        };

                                        list.append(py_value)?;
                                    }
                                } else {
                                    // Handle non-text items
                                    list.append(xml_to_python(py, child, element_type)?)?;
                                }
                            }
                        }
                        return Ok(list.into());
                    }
                    PyTypeKind::Dict => {
                        debug!("Type is dict");
                        let dict = PyDict::new(py);
                        // For dict, we expect items with a "key" attribute
                        for child in children {
                            if let XmlValue::Element(item_name, item_attrs, item_children) = child {
                                if item_name == "item" {
                                    if let Some(key) = item_attrs.get("key") {
                                        // Get the value from the item's children
                                        if item_children.len() == 1 {
                                            if let Some(XmlValue::Text(value_text)) =
                                                item_children.get(0)
                                            {
                                                // Get value type if available
                                                let value_type = ti.args.get(1);
                                                let py_value = xml_to_python(
                                                    py,
                                                    &XmlValue::Text(value_text.clone()),
                                                    value_type,
                                                )?;
                                                dict.set_item(key, py_value)?;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        return Ok(dict.into());
                    }
                    PyTypeKind::Tuple => {
                        debug!("Type is tuple");
                        let items = PyList::empty(py);

                        // Check if this is a homogeneous tuple (Tuple[int, ...])
                        let is_homogeneous = ti.args.len() == 2
                            && ti
                                .args
                                .get(1)
                                .map(|t| t.name == "Ellipsis")
                                .unwrap_or(false);

                        // For typed tuples, match items with their expected types
                        for (i, child) in children.iter().enumerate() {
                            if let XmlValue::Element(item_name, item_attrs, item_children) = child {
                                if item_name == "item" {
                                    // Get the expected type for this position
                                    let expected_type = if is_homogeneous {
                                        // For Tuple[int, ...], use first arg for all positions
                                        ti.args.get(0)
                                    } else {
                                        // For fixed tuples, get type for specific position
                                        ti.args.get(i)
                                    };

                                    // Check if this is a complex type (has child elements)
                                    if !item_children.is_empty()
                                        && item_children
                                            .iter()
                                            .any(|c| matches!(c, XmlValue::Element(_, _, _)))
                                    {
                                        // Complex type - need to handle differently based on expected type
                                        if let Some(expected) = expected_type {
                                            if expected.kind == PyTypeKind::Class {
                                                // Create instance of the class
                                                if let Some(py_type) = &expected.py_type {
                                                    // Check if the type has __gasp_from_partial__ method
                                                    let instance = if py_type
                                                        .as_ref(py)
                                                        .hasattr("__gasp_from_partial__")?
                                                    {
                                                        let empty_dict =
                                                            pyo3::types::PyDict::new(py);
                                                        py_type.as_ref(py).call_method1(
                                                            "__gasp_from_partial__",
                                                            (empty_dict,),
                                                        )?
                                                    } else {
                                                        py_type.as_ref(py).call0()?
                                                    };

                                                    // Set fields from item children
                                                    for field_child in item_children {
                                                        if let XmlValue::Element(
                                                            field_name,
                                                            _,
                                                            field_children,
                                                        ) = field_child
                                                        {
                                                            if let Some(field_info) =
                                                                expected.fields.get(field_name)
                                                            {
                                                                if field_children.len() == 1 {
                                                                    if let Some(XmlValue::Text(
                                                                        text,
                                                                    )) = field_children.get(0)
                                                                    {
                                                                        let py_value =
                                                                            xml_to_python(
                                                                                py,
                                                                                &XmlValue::Text(
                                                                                    text.clone(),
                                                                                ),
                                                                                Some(field_info),
                                                                            )?;
                                                                        instance.setattr(
                                                                            field_name.as_str(),
                                                                            py_value,
                                                                        )?;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }

                                                    items.append(instance)?;
                                                } else {
                                                    // Fallback to generic parsing
                                                    let py_value =
                                                        xml_to_python(py, child, expected_type)?;
                                                    items.append(py_value)?;
                                                }
                                            } else {
                                                // Non-class complex type
                                                let py_value =
                                                    xml_to_python(py, child, expected_type)?;
                                                items.append(py_value)?;
                                            }
                                        } else {
                                            // No expected type
                                            let py_value = xml_to_python(py, child, None)?;
                                            items.append(py_value)?;
                                        }
                                    } else if item_children.len() == 1 {
                                        if let Some(XmlValue::Text(text)) = item_children.get(0) {
                                            let py_value = if let Some(elem_type) = expected_type {
                                                // Use the typed argument
                                                xml_to_python(
                                                    py,
                                                    &XmlValue::Text(text.clone()),
                                                    Some(elem_type),
                                                )?
                                            } else if let Some(type_attr) = item_attrs.get("type") {
                                                // Use type attribute if no args specified
                                                match type_attr.as_str() {
                                                    "int" => {
                                                        text.parse::<i64>().unwrap_or(0).into_py(py)
                                                    }
                                                    "float" => text
                                                        .parse::<f64>()
                                                        .unwrap_or(0.0)
                                                        .into_py(py),
                                                    "bool" | "boolean" => {
                                                        let val = match text.to_lowercase().as_str()
                                                        {
                                                            "true" | "1" | "yes" => true,
                                                            "false" | "0" | "no" => false,
                                                            _ => false,
                                                        };
                                                        val.into_py(py)
                                                    }
                                                    "str" | "string" => text.into_py(py),
                                                    _ => text.into_py(py), // Default to string
                                                }
                                            } else {
                                                // No type info, default to string
                                                text.into_py(py)
                                            };

                                            items.append(py_value)?;
                                        }
                                    }
                                }
                            }
                        }

                        // Convert list to tuple
                        let tuple = items.to_tuple();
                        return Ok(tuple.into());
                    }
                    PyTypeKind::Set => {
                        debug!("Type is set");
                        // For set, we need to use PySet type
                        let py_set = py.eval("set()", None, None)?;
                        let add_method = py_set.getattr("add")?;

                        // Process items
                        for child in children {
                            if let XmlValue::Element(item_name, _, item_children) = child {
                                if item_name == "item" && item_children.len() == 1 {
                                    if let Some(XmlValue::Text(text)) = item_children.get(0) {
                                        // Get element type if available
                                        let element_type = ti.args.get(0);
                                        let py_value = xml_to_python(
                                            py,
                                            &XmlValue::Text(text.clone()),
                                            element_type,
                                        )?;
                                        add_method.call1((py_value,))?;
                                    }
                                }
                            }
                        }
                        return Ok(py_set.into());
                    }
                    PyTypeKind::Optional => {
                        debug!("Type is optional in xml_to_python");
                        // For Optional types, use the inner type
                        if let Some(inner_type) = ti.args.get(0) {
                            debug!("Optional inner type: {:?}", inner_type.name);
                            return xml_to_python(py, value, Some(inner_type));
                        } else {
                            // No inner type specified, treat as any
                            return xml_to_python(py, value, None);
                        }
                    }
                    PyTypeKind::Union => {
                        debug!("Type is union in xml_to_python");
                        // Check for type attribute to determine which union member to use
                        if let Some(type_attr) = attrs.get("type") {
                            debug!("Union has type attribute: {}", type_attr);

                            // Find the matching union member
                            for arg in &ti.args {
                                if &arg.name == type_attr {
                                    debug!("Found matching union member: {}", arg.name);

                                    // Create instance of this specific type
                                    if let Some(py_type) = &arg.py_type {
                                        // Check if the type has __gasp_from_partial__ method
                                        let instance = if py_type
                                            .as_ref(py)
                                            .hasattr("__gasp_from_partial__")?
                                        {
                                            let empty_dict = pyo3::types::PyDict::new(py);
                                            py_type.as_ref(py).call_method1(
                                                "__gasp_from_partial__",
                                                (empty_dict,),
                                            )?
                                        } else {
                                            py_type.as_ref(py).call0()?
                                        };

                                        // Set fields from children
                                        for child in children {
                                            if let XmlValue::Element(
                                                field_name,
                                                _,
                                                field_children,
                                            ) = child
                                            {
                                                if let Some(field_info) = arg.fields.get(field_name)
                                                {
                                                    if field_children.len() == 1 {
                                                        if let Some(XmlValue::Text(text)) =
                                                            field_children.get(0)
                                                        {
                                                            let py_value = xml_to_python(
                                                                py,
                                                                &XmlValue::Text(text.clone()),
                                                                Some(field_info),
                                                            )?;
                                                            instance.setattr(
                                                                field_name.as_str(),
                                                                py_value,
                                                            )?;
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        return Ok(instance.into());
                                    }
                                }
                            }
                        }

                        // Fallback: try tag name discrimination
                        for arg in &ti.args {
                            if &arg.name == name {
                                debug!("Matched union member by tag name: {}", arg.name);
                                if let Some(py_type) = &arg.py_type {
                                    // Check if the type has __gasp_from_partial__ method
                                    let instance =
                                        if py_type.as_ref(py).hasattr("__gasp_from_partial__")? {
                                            let empty_dict = pyo3::types::PyDict::new(py);
                                            py_type.as_ref(py).call_method1(
                                                "__gasp_from_partial__",
                                                (empty_dict,),
                                            )?
                                        } else {
                                            py_type.as_ref(py).call0()?
                                        };
                                    for child in children {
                                        if let XmlValue::Element(field_name, _, field_children) =
                                            child
                                        {
                                            if let Some(field_info) =
                                                arg.fields.get(field_name.as_str())
                                            {
                                                if field_children.len() == 1 {
                                                    if let Some(XmlValue::Text(text)) =
                                                        field_children.get(0)
                                                    {
                                                        let py_value = xml_to_python(
                                                            py,
                                                            &XmlValue::Text(text.clone()),
                                                            Some(field_info),
                                                        )?;
                                                        instance.setattr(
                                                            field_name.as_str(),
                                                            py_value,
                                                        )?;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    return Ok(instance.into());
                                }
                            }
                        }
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
                        let val = match s.to_lowercase().as_str() {
                            "true" | "1" | "yes" => true,
                            "false" | "0" | "no" => false,
                            _ => false,
                        };
                        return Ok(val.into_py(py));
                    }
                    PyTypeKind::None => return Ok(py.None()),
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
    // Check if the type has __gasp_from_partial__ method (Deserializable classes)
    let instance = if py_type.hasattr("__gasp_from_partial__")? {
        // Use __gasp_from_partial__ to create instance with proper initialization
        let empty_dict = pyo3::types::PyDict::new(py);
        py_type.call_method1("__gasp_from_partial__", (empty_dict,))?
    } else {
        // Fall back to regular instantiation for Pydantic and other classes
        py_type.call0()?
    };
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
                    // Use xml_to_python which has proper list handling
                    let py_value = xml_to_python(py, child, Some(field_info))?;
                    instance.setattr(child_name.as_str(), py_value)?;
                } else if field_info.kind == PyTypeKind::Optional {
                    // Handle Optional fields - check the inner type
                    debug!("Field '{}' is Optional type", child_name);
                    if let Some(inner_type) = field_info.args.get(0) {
                        debug!("Optional inner type: {:?}", inner_type.name);

                        // Special handling for Optional[List[...]]
                        if inner_type.kind == PyTypeKind::List {
                            // For lists, we need to parse the children as list items
                            let py_list = PyList::empty(py);
                            let element_type = inner_type.args.get(0);

                            for grand_child in grand_children {
                                if let XmlValue::Element(item_name, item_attrs, item_children) =
                                    grand_child
                                {
                                    if item_name == "item" && item_children.len() == 1 {
                                        if let Some(XmlValue::Text(text)) = item_children.get(0) {
                                            let py_value = xml_to_python(
                                                py,
                                                &XmlValue::Text(text.clone()),
                                                element_type,
                                            )?;
                                            py_list.append(py_value)?;
                                        }
                                    }
                                }
                            }

                            instance.setattr(child_name.as_str(), py_list)?;
                        } else {
                            // For other Optional types, parse normally
                            let py_value = xml_to_python(py, child, Some(inner_type))?;
                            instance.setattr(child_name.as_str(), py_value)?;
                        }
                    } else {
                        // Fallback to generic parsing
                        let py_value = xml_to_python(py, child, Some(field_info))?;
                        instance.setattr(child_name.as_str(), py_value)?;
                    }
                } else if field_info.kind == PyTypeKind::Union {
                    // Handle Union fields
                    debug!("Field '{}' is a Union type", child_name);
                    debug!("Union has {} args", field_info.args.len());
                    for (i, arg) in field_info.args.iter().enumerate() {
                        debug!("  Union arg[{}]: name={}, kind={:?}", i, arg.name, arg.kind);
                    }

                    println!(
                        "DEBUG: Processing Union field '{}' with attrs: {:?}",
                        child_name, child_attrs
                    );
                    println!("DEBUG: Grand children: {:?}", grand_children);

                    // Check for type attribute to determine which union member to use
                    if let Some(type_attr) = child_attrs.get("type") {
                        debug!("Union field has type attribute: {}", type_attr);

                        // Find the matching union member
                        let mut found_match = false;
                        for arg in &field_info.args {
                            debug!(
                                "Checking union member: {} against type_attr: {}",
                                arg.name, type_attr
                            );
                            if &arg.name == type_attr {
                                debug!("Found matching union member: {}", arg.name);

                                // Create instance of this specific type
                                if let Some(py_type) = &arg.py_type {
                                    debug!("Creating instance of {}", arg.name);
                                    // Check if the type has __gasp_from_partial__ method
                                    let union_instance =
                                        if py_type.as_ref(py).hasattr("__gasp_from_partial__")? {
                                            let empty_dict = pyo3::types::PyDict::new(py);
                                            py_type.as_ref(py).call_method1(
                                                "__gasp_from_partial__",
                                                (empty_dict,),
                                            )?
                                        } else {
                                            py_type.as_ref(py).call0()?
                                        };

                                    // Set fields from grand_children
                                    debug!(
                                        "Setting fields from {} grand_children",
                                        grand_children.len()
                                    );
                                    for grand_child in grand_children {
                                        if let XmlValue::Element(field_name, _, field_children) =
                                            grand_child
                                        {
                                            debug!("Processing field: {}", field_name);
                                            if let Some(field_type_info) =
                                                arg.fields.get(field_name.as_str())
                                            {
                                                debug!("Found field type info for {}", field_name);
                                                if field_children.len() == 1 {
                                                    if let Some(XmlValue::Text(text)) =
                                                        field_children.get(0)
                                                    {
                                                        let py_value = xml_to_python(
                                                            py,
                                                            &XmlValue::Text(text.clone()),
                                                            Some(field_type_info),
                                                        )?;
                                                        union_instance.setattr(
                                                            field_name.as_str(),
                                                            py_value,
                                                        )?;
                                                        debug!("Set field {} to value", field_name);
                                                    }
                                                }
                                            } else {
                                                debug!("No field type info for {}", field_name);
                                            }
                                        }
                                    }

                                    instance.setattr(child_name.as_str(), union_instance)?;
                                    debug!("Set union field {} on parent instance", child_name);
                                    found_match = true;
                                    break;
                                } else {
                                    debug!("No py_type for union member {}", arg.name);
                                }
                            }
                        }

                        if !found_match {
                            debug!(
                                "No matching union member found for type_attr: {}",
                                type_attr
                            );
                            // Fall back to generic parsing
                            let py_value = xml_to_python(py, child, Some(field_info))?;
                            instance.setattr(child_name.as_str(), py_value)?;
                        }
                    } else {
                        debug!("No type attribute, falling back to generic parsing");
                        // No type attribute, fall back to generic parsing
                        let py_value = xml_to_python(py, child, Some(field_info))?;
                        instance.setattr(child_name.as_str(), py_value)?;
                    }
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
    debug!(
        "create_instance_from_xml_events: type_info.name = {}, kind = {:?}",
        type_info.name, type_info.kind
    );

    // Handle Dict types
    if type_info.kind == PyTypeKind::Dict {
        debug!("Handling Dict type");
        let xml_value = crate::xml_parser::events_to_xml_value(events)?;

        if let XmlValue::Element(tag_name, _, children) = xml_value {
            if tag_name == "dict" {
                let py_dict = PyDict::new(py);

                // Process each item
                for child in children {
                    match child {
                        XmlValue::Element(item_name, item_attrs, item_children) => {
                            if item_name == "item" {
                                if let Some(key) = item_attrs.get("key") {
                                    // Get the value - it should be in item_children
                                    if item_children.len() == 1 {
                                        if let Some(XmlValue::Text(text)) = item_children.get(0) {
                                            // Get value type if available
                                            let py_value = if let Some(value_type) =
                                                type_info.args.get(1)
                                            {
                                                xml_to_python(
                                                    py,
                                                    &XmlValue::Text(text.clone()),
                                                    Some(value_type),
                                                )?
                                            } else if let Some(type_attr) = item_attrs.get("type") {
                                                // Use type attribute if no type args
                                                match type_attr.as_str() {
                                                    "int" => {
                                                        text.parse::<i64>().unwrap_or(0).into_py(py)
                                                    }
                                                    "float" => text
                                                        .parse::<f64>()
                                                        .unwrap_or(0.0)
                                                        .into_py(py),
                                                    "bool" | "boolean" => {
                                                        let val = match text.to_lowercase().as_str()
                                                        {
                                                            "true" | "1" | "yes" => true,
                                                            "false" | "0" | "no" => false,
                                                            _ => false,
                                                        };
                                                        val.into_py(py)
                                                    }
                                                    "str" | "string" => text.into_py(py),
                                                    _ => text.into_py(py), // Default to string
                                                }
                                            } else {
                                                // No type info, default to string
                                                text.into_py(py)
                                            };
                                            py_dict.set_item(key, py_value)?;
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }

                return Ok(py_dict.into());
            }
        }

        // Fallback: return empty dict
        return Ok(PyDict::new(py).into());
    }

    // Handle Tuple types
    if type_info.kind == PyTypeKind::Tuple {
        debug!("Handling Tuple type");
        let xml_value = crate::xml_parser::events_to_xml_value(events)?;

        if let XmlValue::Element(tag_name, _, children) = xml_value {
            if tag_name == "tuple" {
                let items = PyList::empty(py);

                // Check if this is a homogeneous tuple (Tuple[int, ...])
                let is_homogeneous = type_info.args.len() == 2
                    && type_info
                        .args
                        .get(1)
                        .map(|t| t.name == "Ellipsis")
                        .unwrap_or(false);

                // Process each item with its expected type
                for (i, child) in children.iter().enumerate() {
                    if let XmlValue::Element(item_name, item_attrs, item_children) = child {
                        if item_name == "item" {
                            // Get the expected type for this position
                            let expected_type = if is_homogeneous {
                                // For Tuple[int, ...], use first arg for all positions
                                type_info.args.get(0)
                            } else {
                                // For fixed tuples, get type for specific position
                                type_info.args.get(i)
                            };

                            // Check if this is a complex type (has child elements)
                            if !item_children.is_empty()
                                && item_children
                                    .iter()
                                    .any(|c| matches!(c, XmlValue::Element(_, _, _)))
                            {
                                // Complex type - parse as XML element
                                let py_value = xml_to_python(py, child, expected_type)?;
                                items.append(py_value)?;
                            } else if item_children.len() == 1 {
                                if let Some(XmlValue::Text(text)) = item_children.get(0) {
                                    let py_value = if let Some(elem_type) = expected_type {
                                        // Use the typed argument
                                        xml_to_python(
                                            py,
                                            &XmlValue::Text(text.clone()),
                                            Some(elem_type),
                                        )?
                                    } else if let Some(type_attr) = item_attrs.get("type") {
                                        // Use type attribute if no args specified
                                        match type_attr.as_str() {
                                            "int" => text.parse::<i64>().unwrap_or(0).into_py(py),
                                            "float" => {
                                                text.parse::<f64>().unwrap_or(0.0).into_py(py)
                                            }
                                            "bool" | "boolean" => {
                                                let val = match text.to_lowercase().as_str() {
                                                    "true" | "1" | "yes" => true,
                                                    "false" | "0" | "no" => false,
                                                    _ => false,
                                                };
                                                val.into_py(py)
                                            }
                                            "str" | "string" => text.into_py(py),
                                            _ => text.into_py(py), // Default to string
                                        }
                                    } else {
                                        // No type info, default to string
                                        text.into_py(py)
                                    };

                                    items.append(py_value)?;
                                }
                            }
                        }
                    }
                }

                // Convert list to tuple
                let tuple = items.to_tuple();
                return Ok(tuple.into());
            }
        }

        // Fallback: return empty tuple
        return Ok(PyList::empty(py).to_tuple().into());
    }

    // Handle Set types
    if type_info.kind == PyTypeKind::Set {
        debug!("Handling Set type");
        let xml_value = crate::xml_parser::events_to_xml_value(events)?;

        if let XmlValue::Element(tag_name, _, children) = xml_value {
            if tag_name == "set" {
                // Create a new set
                let py_set = py.eval("set()", None, None)?;
                let add_method = py_set.getattr("add")?;

                // Process each item
                for child in children {
                    if let XmlValue::Element(item_name, item_attrs, item_children) = child {
                        if item_name == "item" {
                            // Get the value - it should be in item_children
                            if item_children.len() == 1 {
                                if let Some(XmlValue::Text(text)) = item_children.get(0) {
                                    // Get element type if available
                                    let py_value = if let Some(element_type) = type_info.args.get(0)
                                    {
                                        xml_to_python(
                                            py,
                                            &XmlValue::Text(text.clone()),
                                            Some(element_type),
                                        )?
                                    } else if let Some(type_attr) = item_attrs.get("type") {
                                        // Use type attribute if no type args
                                        match type_attr.as_str() {
                                            "int" => text.parse::<i64>().unwrap_or(0).into_py(py),
                                            "float" => {
                                                text.parse::<f64>().unwrap_or(0.0).into_py(py)
                                            }
                                            "bool" | "boolean" => {
                                                let val = match text.to_lowercase().as_str() {
                                                    "true" | "1" | "yes" => true,
                                                    "false" | "0" | "no" => false,
                                                    _ => false,
                                                };
                                                val.into_py(py)
                                            }
                                            "str" | "string" => text.into_py(py),
                                            _ => text.into_py(py), // Default to string
                                        }
                                    } else {
                                        // No type info, default to string
                                        text.into_py(py)
                                    };
                                    add_method.call1((py_value,))?;
                                }
                            }
                        }
                    }
                }

                return Ok(py_set.into());
            }
        }

        // Fallback: return empty set
        return Ok(py.eval("set()", None, None)?.into());
    }

    // Handle List types
    if type_info.kind == PyTypeKind::List {
        debug!("Handling List type");

        // For List types, we need to parse the XML differently
        // Convert events to XmlValue first
        let xml_value = crate::xml_parser::events_to_xml_value(events)?;

        if let XmlValue::Element(tag_name, _, children) = xml_value {
            if tag_name == "list" {
                // Create an empty list
                let py_list = PyList::empty(py);

                // Get the element type
                let element_type = type_info.args.get(0);

                // Process each child item
                for child in children {
                    if let XmlValue::Element(item_name, item_attrs, item_children) = child {
                        if item_name == "item" {
                            // Check if we have a single text child
                            if item_children.len() == 1 {
                                if let Some(XmlValue::Text(text)) = item_children.get(0) {
                                    let py_value = if let Some(elem_type) = element_type {
                                        // Use the typed argument
                                        xml_to_python(
                                            py,
                                            &XmlValue::Text(text.clone()),
                                            Some(elem_type),
                                        )?
                                    } else if let Some(type_attr) = item_attrs.get("type") {
                                        // Use type attribute if no args specified
                                        match type_attr.as_str() {
                                            "int" => text.parse::<i64>().unwrap_or(0).into_py(py),
                                            "float" => {
                                                text.parse::<f64>().unwrap_or(0.0).into_py(py)
                                            }
                                            "bool" | "boolean" => {
                                                let val = match text.to_lowercase().as_str() {
                                                    "true" | "1" | "yes" => true,
                                                    "false" | "0" | "no" => false,
                                                    _ => false,
                                                };
                                                val.into_py(py)
                                            }
                                            "str" | "string" => text.into_py(py),
                                            _ => text.into_py(py), // Default to string
                                        }
                                    } else {
                                        // No type info, default to string
                                        text.into_py(py)
                                    };

                                    py_list.append(py_value)?;
                                    continue;
                                }
                            }

                            // Handle more complex items
                            // For Union element types, check the type attribute
                            if let Some(elem_type) = element_type {
                                if elem_type.kind == PyTypeKind::Union {
                                    // Look for type attribute
                                    if let Some(type_val) = item_attrs.get("type") {
                                        // Find the matching union member
                                        for arg in &elem_type.args {
                                            if &arg.name == type_val {
                                                // Create instance of this specific type
                                                if let Some(py_type) = &arg.py_type {
                                                    // Check if the type has __gasp_from_partial__ method
                                                    let instance = if py_type
                                                        .as_ref(py)
                                                        .hasattr("__gasp_from_partial__")?
                                                    {
                                                        let empty_dict =
                                                            pyo3::types::PyDict::new(py);
                                                        py_type.as_ref(py).call_method1(
                                                            "__gasp_from_partial__",
                                                            (empty_dict,),
                                                        )?
                                                    } else {
                                                        py_type.as_ref(py).call0()?
                                                    };

                                                    // Set fields from item children
                                                    for item_child in &item_children {
                                                        if let XmlValue::Element(
                                                            field_name,
                                                            field_attrs,
                                                            field_children,
                                                        ) = item_child
                                                        {
                                                            if let Some(field_info) =
                                                                arg.fields.get(field_name)
                                                            {
                                                                // Check if this is a simple scalar field with single text child
                                                                if field_children.len() == 1
                                                                    && matches!(
                                                                        field_info.kind,
                                                                        PyTypeKind::String
                                                                            | PyTypeKind::Integer
                                                                            | PyTypeKind::Float
                                                                            | PyTypeKind::Boolean
                                                                    )
                                                                {
                                                                    if let Some(XmlValue::Text(
                                                                        text,
                                                                    )) = field_children.get(0)
                                                                    {
                                                                        let py_value =
                                                                            xml_to_python(
                                                                                py,
                                                                                &XmlValue::Text(
                                                                                    text.clone(),
                                                                                ),
                                                                                Some(field_info),
                                                                            )?;
                                                                        instance.setattr(
                                                                            field_name.as_str(),
                                                                            py_value,
                                                                        )?;
                                                                    }
                                                                } else {
                                                                    // For complex fields like lists, unions, classes, use xml_to_python with the full element
                                                                    let py_value = xml_to_python(
                                                                        py,
                                                                        &XmlValue::Element(
                                                                            field_name.clone(),
                                                                            field_attrs.clone(),
                                                                            field_children.clone(),
                                                                        ),
                                                                        Some(field_info),
                                                                    )?;
                                                                    instance.setattr(
                                                                        field_name.as_str(),
                                                                        py_value,
                                                                    )?;
                                                                }
                                                            }
                                                        }
                                                    }

                                                    py_list.append(instance)?;
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    // Non-union element type
                                    if elem_type.kind == PyTypeKind::Class {
                                        // For class types, we need to create an instance
                                        if let Some(py_type) = &elem_type.py_type {
                                            // Check if the type has __gasp_from_partial__ method
                                            let instance = if py_type
                                                .as_ref(py)
                                                .hasattr("__gasp_from_partial__")?
                                            {
                                                let empty_dict = pyo3::types::PyDict::new(py);
                                                py_type.as_ref(py).call_method1(
                                                    "__gasp_from_partial__",
                                                    (empty_dict,),
                                                )?
                                            } else {
                                                py_type.as_ref(py).call0()?
                                            };

                                            // Set fields from item children
                                            for item_child in &item_children {
                                                if let XmlValue::Element(
                                                    field_name,
                                                    _,
                                                    field_children,
                                                ) = item_child
                                                {
                                                    if let Some(field_info) =
                                                        elem_type.fields.get(field_name)
                                                    {
                                                        if field_children.len() == 1 {
                                                            if let Some(XmlValue::Text(text)) =
                                                                field_children.get(0)
                                                            {
                                                                let py_value = xml_to_python(
                                                                    py,
                                                                    &XmlValue::Text(text.clone()),
                                                                    Some(field_info),
                                                                )?;
                                                                instance.setattr(
                                                                    field_name.as_str(),
                                                                    py_value,
                                                                )?;
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            py_list.append(instance)?;
                                        }
                                    } else if item_children.is_empty() {
                                        // Empty item, append empty string or None
                                        let py_item = xml_to_python(
                                            py,
                                            &XmlValue::Text("".to_string()),
                                            element_type,
                                        )?;
                                        py_list.append(py_item)?;
                                    }
                                }
                            }
                        }
                    }
                }

                return Ok(py_list.into());
            }
        }

        // Fallback: return empty list
        return Ok(PyList::empty(py).into());
    }

    // Handle Union types
    if type_info.kind == PyTypeKind::Union {
        debug!("Handling Union type with {} members", type_info.args.len());
        debug!(
            "Union args: {:?}",
            type_info.args.iter().map(|a| &a.name).collect::<Vec<_>>()
        );

        // Look at the first event to determine which union member to use
        let mut tag_name = String::new();
        let mut type_attr = None;

        debug!("Examining events to find tag name and type attribute");
        for (i, event) in events.iter().enumerate() {
            debug!("Event {}: {:?}", i, event);
            if let Ok(Event::ElementStart(tag)) = event {
                tag_name = tag.name.clone();
                debug!("Found ElementStart with tag name: {}", tag_name);
                // Check for type attribute
                for ((attr_name, _), attr_value) in &tag.attributes {
                    debug!("  Attribute: {} = {}", attr_name, attr_value);
                    if attr_name == "type" {
                        type_attr = Some(attr_value.clone());
                        break;
                    }
                }
                break;
            }
        }

        debug!(
            "Union discrimination: tag_name = '{}', type_attr = {:?}",
            tag_name, type_attr
        );

        // Try to match against union members
        for arg in &type_info.args {
            debug!("Checking union member: {} (kind: {:?})", arg.name, arg.kind);

            // Match by tag name
            if arg.name == tag_name {
                debug!("Matched union member by tag name: {}", arg.name);
                return create_instance_from_xml_events(py, arg, events);
            }

            // Match by type attribute
            if let Some(ref type_val) = type_attr {
                if &arg.name == type_val {
                    debug!("Matched union member by type attribute: {}", arg.name);
                    return create_instance_from_xml_events(py, arg, events);
                }
            }
        }

        // If no exact match, try to match the union type name itself
        if tag_name == type_info.name {
            // The tag matches the union type name (e.g., <MyUnion>)
            // Use the type attribute to determine which member
            if let Some(type_val) = type_attr {
                for arg in &type_info.args {
                    if arg.name == type_val {
                        debug!(
                            "Matched union member by type attribute in union-named tag: {}",
                            arg.name
                        );
                        return create_instance_from_xml_events(py, arg, events);
                    }
                }
            }
        }

        debug!("No union member matched, returning None");
        return Ok(py.None());
    }

    debug!(
        "create_instance_from_xml_events: type_info.fields = {:?}",
        type_info.fields.keys().collect::<Vec<_>>()
    );

    // For non-list and non-union types, convert events to XmlValue first
    let xml_value = crate::xml_parser::events_to_xml_value(events)?;

    if let XmlValue::Element(tag_name, attrs, children) = xml_value {
        if tag_name == type_info.name {
            // Create instance
            let instance = type_info.py_type.as_ref().unwrap().as_ref(py).call0()?;

            // Set attributes
            for (k, v) in &attrs {
                if let Some(field_info) = type_info.fields.get(k) {
                    let py_value = xml_to_python(py, &XmlValue::Text(v.clone()), Some(field_info))?;
                    instance.setattr(k.as_str(), py_value)?;
                }
            }

            // Process children
            for child in children {
                if let XmlValue::Element(child_name, child_attrs, child_children) = child {
                    if let Some(field_info) = type_info.fields.get(&child_name) {
                        if field_info.kind == PyTypeKind::List {
                            // Handle list fields
                            let py_list = PyList::empty(py);
                            let element_type = field_info.args.get(0);

                            // Process each item in the list
                            for item in child_children {
                                if let XmlValue::Element(item_name, item_attrs, item_children) =
                                    item
                                {
                                    if item_name == "item" {
                                        if let Some(elem_type) = element_type {
                                            if elem_type.kind == PyTypeKind::Class {
                                                // Create instance of the class
                                                if let Some(py_type) = &elem_type.py_type {
                                                    let item_instance =
                                                        py_type.as_ref(py).call0()?;

                                                    // Set fields from item children
                                                    for item_child in item_children {
                                                        if let XmlValue::Element(
                                                            field_name,
                                                            _,
                                                            field_children,
                                                        ) = item_child
                                                        {
                                                            if let Some(field_info) =
                                                                elem_type.fields.get(&field_name)
                                                            {
                                                                if field_children.len() == 1 {
                                                                    if let Some(XmlValue::Text(
                                                                        text,
                                                                    )) = field_children.get(0)
                                                                    {
                                                                        let py_value =
                                                                            xml_to_python(
                                                                                py,
                                                                                &XmlValue::Text(
                                                                                    text.clone(),
                                                                                ),
                                                                                Some(field_info),
                                                                            )?;
                                                                        item_instance.setattr(
                                                                            field_name.as_str(),
                                                                            py_value,
                                                                        )?;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }

                                                    py_list.append(item_instance)?;
                                                }
                                            } else {
                                                // For non-class types (like str, int, etc.)
                                                // Extract the text content from the item
                                                if item_children.len() == 1 {
                                                    if let Some(XmlValue::Text(text)) =
                                                        item_children.get(0)
                                                    {
                                                        let py_item = xml_to_python(
                                                            py,
                                                            &XmlValue::Text(text.clone()),
                                                            element_type,
                                                        )?;
                                                        py_list.append(py_item)?;
                                                    }
                                                } else if item_children.is_empty() {
                                                    // Empty item, append empty string or None
                                                    let py_item = xml_to_python(
                                                        py,
                                                        &XmlValue::Text("".to_string()),
                                                        element_type,
                                                    )?;
                                                    py_list.append(py_item)?;
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            instance.setattr(child_name.as_str(), py_list)?;
                        } else if field_info.kind == PyTypeKind::Union {
                            // Handle Union fields
                            debug!(
                                "Processing Union field '{}' in create_instance_from_xml_events",
                                child_name
                            );
                            let py_value = xml_to_python(
                                py,
                                &XmlValue::Element(child_name.clone(), child_attrs, child_children),
                                Some(field_info),
                            )?;
                            instance.setattr(child_name.as_str(), py_value)?;
                        } else if child_children.len() == 1 {
                            // Handle scalar fields with single text child
                            if let Some(XmlValue::Text(text)) = child_children.get(0) {
                                let py_value = xml_to_python(
                                    py,
                                    &XmlValue::Text(text.clone()),
                                    Some(field_info),
                                )?;
                                instance.setattr(child_name.as_str(), py_value)?;
                            }
                        } else {
                            // Handle nested objects
                            let py_value = xml_to_python(
                                py,
                                &XmlValue::Element(child_name.clone(), child_attrs, child_children),
                                Some(field_info),
                            )?;
                            instance.setattr(child_name.as_str(), py_value)?;
                        }
                    }
                }
            }

            return Ok(instance.into());
        }
    }

    // Fallback
    Ok(py.None())
}
