use crate::python_types::{PyTypeInfo, PyTypeKind};
use pyo3::PyResult;

/// Parse a type string like "list[str]" or "tuple[str, int, Optional[float]]"
pub fn parse_type_string(type_str: &str) -> PyResult<PyTypeInfo> {
    let trimmed = type_str.trim();

    // Check if it has brackets (generic type)
    if let Some(bracket_pos) = trimmed.find('[') {
        let base_type = trimmed[..bracket_pos].trim();
        let args_end = trimmed.rfind(']').ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Unclosed bracket in type: {}",
                type_str
            ))
        })?;
        let args_str = &trimmed[bracket_pos + 1..args_end];

        // Parse the base type
        let (kind, name) = match base_type {
            "list" | "List" => (PyTypeKind::List, "list"),
            "dict" | "Dict" => (PyTypeKind::Dict, "dict"),
            "set" | "Set" => (PyTypeKind::Set, "set"),
            "tuple" | "Tuple" => (PyTypeKind::Tuple, "tuple"),
            "Optional" => (PyTypeKind::Optional, "Optional"),
            "Union" => (PyTypeKind::Union, "Union"),
            _ => {
                // Unknown generic type, treat as Any
                (PyTypeKind::Any, base_type)
            }
        };

        // Parse the arguments
        let args = parse_type_args(args_str)?;

        let mut type_info = PyTypeInfo::new(kind.clone(), name.to_string());
        type_info = type_info.with_args(args);

        // Set appropriate module
        match kind {
            PyTypeKind::List | PyTypeKind::Dict | PyTypeKind::Set | PyTypeKind::Tuple => {
                type_info = type_info.with_module("builtins".to_string());
            }
            PyTypeKind::Optional | PyTypeKind::Union => {
                type_info = type_info.with_module("typing".to_string());
            }
            _ => {}
        }

        Ok(type_info)
    } else {
        // Simple type without brackets
        let (kind, name) = match trimmed {
            "int" => (PyTypeKind::Integer, "int"),
            "str" | "string" => (PyTypeKind::String, "str"),
            "float" => (PyTypeKind::Float, "float"),
            "bool" | "boolean" => (PyTypeKind::Boolean, "bool"),
            "list" | "List" => (PyTypeKind::List, "list"),
            "dict" | "Dict" => (PyTypeKind::Dict, "dict"),
            "set" | "Set" => (PyTypeKind::Set, "set"),
            "tuple" | "Tuple" => (PyTypeKind::Tuple, "tuple"),
            "None" => (PyTypeKind::None, "None"),
            "Ellipsis" | "..." => (PyTypeKind::Any, "Ellipsis"), // Special case for Tuple[T, ...]
            _ => {
                // Unknown type, could be a class name
                (PyTypeKind::Class, trimmed)
            }
        };

        let type_info = PyTypeInfo::new(kind, name.to_string()).with_module("builtins".to_string());

        Ok(type_info)
    }
}

/// Parse comma-separated type arguments, handling nested brackets
fn parse_type_args(args_str: &str) -> PyResult<Vec<PyTypeInfo>> {
    let mut args = Vec::new();
    let mut current_arg = String::new();
    let mut bracket_depth = 0;
    let mut in_quotes = false;
    let mut escape_next = false;

    for ch in args_str.chars() {
        if escape_next {
            current_arg.push(ch);
            escape_next = false;
            continue;
        }

        match ch {
            '\\' => {
                escape_next = true;
                current_arg.push(ch);
            }
            '"' | '\'' => {
                in_quotes = !in_quotes;
                current_arg.push(ch);
            }
            '[' if !in_quotes => {
                bracket_depth += 1;
                current_arg.push(ch);
            }
            ']' if !in_quotes => {
                bracket_depth -= 1;
                current_arg.push(ch);
            }
            ',' if bracket_depth == 0 && !in_quotes => {
                // End of current argument
                let arg_str = current_arg.trim();
                if !arg_str.is_empty() {
                    args.push(parse_type_string(arg_str)?);
                }
                current_arg.clear();
            }
            _ => {
                current_arg.push(ch);
            }
        }
    }

    // Don't forget the last argument
    let arg_str = current_arg.trim();
    if !arg_str.is_empty() {
        args.push(parse_type_string(arg_str)?);
    }

    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_types() {
        let int_type = parse_type_string("int").unwrap();
        assert_eq!(int_type.name, "int");
        assert_eq!(int_type.kind, PyTypeKind::Integer);

        let str_type = parse_type_string("str").unwrap();
        assert_eq!(str_type.name, "str");
        assert_eq!(str_type.kind, PyTypeKind::String);
    }

    #[test]
    fn test_parse_list_type() {
        let list_type = parse_type_string("list[str]").unwrap();
        assert_eq!(list_type.name, "list");
        assert_eq!(list_type.kind, PyTypeKind::List);
        assert_eq!(list_type.args.len(), 1);
        assert_eq!(list_type.args[0].name, "str");
    }

    #[test]
    fn test_parse_dict_type() {
        let dict_type = parse_type_string("dict[str, int]").unwrap();
        assert_eq!(dict_type.name, "dict");
        assert_eq!(dict_type.kind, PyTypeKind::Dict);
        assert_eq!(dict_type.args.len(), 2);
        assert_eq!(dict_type.args[0].name, "str");
        assert_eq!(dict_type.args[1].name, "int");
    }

    #[test]
    fn test_parse_tuple_type() {
        let tuple_type = parse_type_string("tuple[str, int, float]").unwrap();
        assert_eq!(tuple_type.name, "tuple");
        assert_eq!(tuple_type.kind, PyTypeKind::Tuple);
        assert_eq!(tuple_type.args.len(), 3);
        assert_eq!(tuple_type.args[0].name, "str");
        assert_eq!(tuple_type.args[1].name, "int");
        assert_eq!(tuple_type.args[2].name, "float");
    }

    #[test]
    fn test_parse_nested_type() {
        let nested_type = parse_type_string("list[dict[str, int]]").unwrap();
        assert_eq!(nested_type.name, "list");
        assert_eq!(nested_type.kind, PyTypeKind::List);
        assert_eq!(nested_type.args.len(), 1);

        let inner_dict = &nested_type.args[0];
        assert_eq!(inner_dict.name, "dict");
        assert_eq!(inner_dict.kind, PyTypeKind::Dict);
        assert_eq!(inner_dict.args.len(), 2);
    }

    #[test]
    fn test_parse_optional_type() {
        let opt_type = parse_type_string("Optional[str]").unwrap();
        assert_eq!(opt_type.name, "Optional");
        assert_eq!(opt_type.kind, PyTypeKind::Optional);
        assert_eq!(opt_type.args.len(), 1);
        assert_eq!(opt_type.args[0].name, "str");
    }

    #[test]
    fn test_parse_complex_tuple() {
        let tuple_type = parse_type_string("tuple[str, int, Optional[float]]").unwrap();
        assert_eq!(tuple_type.name, "tuple");
        assert_eq!(tuple_type.kind, PyTypeKind::Tuple);
        assert_eq!(tuple_type.args.len(), 3);
        assert_eq!(tuple_type.args[0].name, "str");
        assert_eq!(tuple_type.args[1].name, "int");
        assert_eq!(tuple_type.args[2].name, "Optional");
        assert_eq!(tuple_type.args[2].kind, PyTypeKind::Optional);
    }
}
