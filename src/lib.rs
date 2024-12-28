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

use rd_json_stack_parser::Parser as JsonParser;

/// Python wrapper for WAIL validation
#[pyclass]
#[derive(Debug)]
struct WAILGenerator {
    wail_content: String,
    json_content: Option<String>,
}

#[pymethods]
impl WAILGenerator {
    #[new]
    fn new() -> Self {
        Self {
            wail_content: String::new(),
            json_content: None,
        }
    }

    /// Load WAIL schema content
    #[pyo3(text_signature = "($self, content)")]
    fn load_wail(&mut self, content: String) -> PyResult<()> {
        self.wail_content = content;
        Ok(())
    }

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
    fn validate_llm_output(&self, llm_output: String) -> PyResult<(Vec<String>, Vec<String>)> {
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

                let parsed_output = parser.parse_llm_output(&llm_output).map_err(|e| {
                    PyValueError::new_err(format!("Failed to parse LLM output: {:?}", e))
                })?;

                parser
                    .validate_json(&parsed_output.to_string())
                    .map_err(|e| {
                        PyValueError::new_err(format!("Failed to validate LLM output: {:?}", e))
                    })?;

                Ok((warning_strs, error_strs))
            }
            Err(e) => Err(PyValueError::new_err(format!(
                "Failed to parse WAIL schema: {:?}",
                e
            ))),
        }
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
