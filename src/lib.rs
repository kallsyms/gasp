use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::collections::HashMap;

mod json_types;
mod parser_types;
mod rd_json_stack_parser;
mod types;
mod wail_parser;

/// Python wrapper for WAIL validation
#[pyclass]
#[derive(Debug)]
struct WAILValidator {
    wail_content: String,
    json_content: Option<String>,
}

#[pymethods]
impl WAILValidator {
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

    /// Load JSON content to validate
    #[pyo3(text_signature = "($self, content)")]
    fn load_json(&mut self, content: String) -> PyResult<()> {
        self.json_content = Some(content);
        Ok(())
    }

    /// Validate the loaded WAIL schema and optionally the JSON content
    #[pyo3(text_signature = "($self)")]
    fn validate(&self) -> PyResult<(Vec<String>, Vec<String>)> {
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

                // TODO: If JSON content is present, validate it against the schema
                if let Some(json) = &self.json_content {
                    // Add JSON validation here
                }

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
    m.add_class::<WAILValidator>()?;
    Ok(())
}
