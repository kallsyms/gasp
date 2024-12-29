use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

mod json_types;
mod parser_types;
mod rd_json_stack_parser;
mod types;
mod wail_parser;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while1},
    character::complete::{alpha1, char, multispace0, multispace1},
    combinator::opt,
    multi::{many0, separated_list0},
    sequence::{delimited, preceded, tuple},
    IResult,
};

use pyo3::types::{PyDict, PyList};
use pyo3::Python;

use crate::json_types::{JsonValue, Number};

use rd_json_stack_parser::Parser as JsonParser;

fn json_value_to_py_object(py: Python, value: &JsonValue) -> PyObject {
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

/// Python wrapper for WAIL validation
#[pyclass]
#[derive(Debug)]
struct WAILGenerator {
    wail_content: String,
}

#[pymethods]
impl WAILGenerator {
    #[new]
    fn new() -> Self {
        Self {
            wail_content: String::new(),
        }
    }

    /// Load WAIL schema content
    #[pyo3(text_signature = "($self, content)")]
    fn load_wail(&mut self, content: String) -> PyResult<()> {
        self.wail_content = content;
        Ok(())
    }

    #[pyo3(text_signature = "($self)")]
    fn get_prompt(&self) -> PyResult<(Option<String>, Vec<String>, Vec<String>)> {
        let parser = wail_parser::WAILParser::new();

        // First parse and validate the WAIL schema
        match parser.parse_wail_file(&self.wail_content) {
            Ok(_) => {
                let (warnings, errors) = parser.validate();

                // Convert warnings to strings
                let warning_strs: Vec<String> = warnings
                    .iter()
                    .map(|w| match w {
                        wail_parser::ValidationWarning::UndefinedType {
                            type_name,
                            location,
                        } => format!("Undefined type '{}' at {}", type_name, location),
                        wail_parser::ValidationWarning::PossibleTypo {
                            type_name,
                            similar_to,
                            location,
                        } => format!(
                            "Possible typo: '{}' might be '{}' at {}",
                            type_name, similar_to, location
                        ),
                        wail_parser::ValidationWarning::NoMainBlock => {
                            "No main block found in WAIL schema".to_string()
                        }
                    })
                    .collect();

                // Convert errors to strings
                let error_strs: Vec<String> = errors
                    .iter()
                    .map(|e| match e {
                        wail_parser::ValidationError::UndefinedTypeInTemplate {
                            template_name,
                            type_name,
                            is_return_type,
                        } => {
                            let type_kind = if *is_return_type {
                                "return type"
                            } else {
                                "parameter type"
                            };
                            format!(
                                "Undefined {} '{}' in template '{}'",
                                type_kind, type_name, template_name
                            )
                        }
                    })
                    .collect();

                if errors.is_empty() {
                    Ok((Some(parser.prepare_prompt()), warning_strs, error_strs))
                } else {
                    Ok((None, warning_strs, error_strs))
                }
            }
            Err(e) => Err(PyValueError::new_err(format!(
                "Failed to parse WAIL schema: {:?}",
                e
            ))),
        }
    }

    #[pyo3(text_signature = "($self, llm_output)")]
    fn parse_llm_output(&self, llm_output: String) -> PyResult<PyObject> {
        // Do all JSON parsing and validation outside the GIL
        let parser = wail_parser::WAILParser::new();

        // Parse WAIL schema first
        if let Err(e) = parser.parse_wail_file(&self.wail_content) {
            return Err(PyValueError::new_err(format!(
                "Failed to parse WAIL schema: {:?}",
                e
            )));
        }

        // Parse and validate the LLM output
        let parsed_output = parser
            .parse_llm_output(&llm_output)
            .map_err(|e| PyValueError::new_err(format!("Failed to parse LLM output: {:?}", e)))?;

        parser
            .validate_json(&parsed_output.to_string())
            .map_err(|e| {
                PyValueError::new_err(format!("Failed to validate LLM output: {:?}", e))
            })?;

        // Only acquire the GIL when we need to create Python objects
        Python::with_gil(|py| Ok(json_value_to_py_object(py, &parsed_output)))
    }

    /// Validate the loaded WAIL schema and the LLM output against the schema
    #[pyo3(text_signature = "($self)")]
    fn validate_wail(&self) -> PyResult<(Vec<String>, Vec<String>)> {
        let parser = wail_parser::WAILParser::new();

        // First parse and validate the WAIL schema
        match parser.parse_wail_file(&self.wail_content) {
            Ok(_) => {
                let (warnings, errors) = parser.validate();

                // Convert warnings to strings
                let warning_strs: Vec<String> = warnings
                    .iter()
                    .map(|w| match w {
                        wail_parser::ValidationWarning::UndefinedType {
                            type_name,
                            location,
                        } => format!("Undefined type '{}' at {}", type_name, location),
                        wail_parser::ValidationWarning::PossibleTypo {
                            type_name,
                            similar_to,
                            location,
                        } => format!(
                            "Possible typo: '{}' might be '{}' at {}",
                            type_name, similar_to, location
                        ),
                        wail_parser::ValidationWarning::NoMainBlock => {
                            "No main block found in WAIL schema".to_string()
                        }
                    })
                    .collect();

                // Convert errors to strings
                let error_strs: Vec<String> = errors
                    .iter()
                    .map(|e| match e {
                        wail_parser::ValidationError::UndefinedTypeInTemplate {
                            template_name,
                            type_name,
                            is_return_type,
                        } => {
                            let type_kind = if *is_return_type {
                                "return type"
                            } else {
                                "parameter type"
                            };
                            format!(
                                "Undefined {} '{}' in template '{}'",
                                type_kind, type_name, template_name
                            )
                        }
                    })
                    .collect();

                Ok((warning_strs, error_strs))
            }
            Err(e) => Err(PyValueError::new_err(format!(
                "Failed to parse WAIL schema: {:?}",
                e
            ))),
        }
    }
}

/// A Python module for working with WAIL files
#[pymodule]
fn gasp(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<WAILGenerator>()?;
    Ok(())
}
