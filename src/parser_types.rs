use crate::json_types::JsonValue;
use crate::types::*;
use regex;
use std::collections::HashMap;
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub enum WAILAnnotation {
    Description(String),
}

#[derive(Debug, Clone)]
pub enum TemplateArgument {
    String(String),
    Number(i64),
    Float(f64),
    TypeRef(String), // For when we reference a type like "String" or "Number"
}

#[derive(Debug, Clone)]
pub struct WAILTemplateCall {
    pub template_name: String,
    pub arguments: HashMap<String, TemplateArgument>,
}

#[derive(Debug, Clone)]
pub enum MainStatement {
    Assignment {
        variable: String,
        template_call: WAILTemplateCall,
    },
    TemplateCall(WAILTemplateCall),
}

#[derive(Debug, Clone)]
pub struct WAILField<'a> {
    pub name: String,
    pub field_type: WAILType<'a>,
    pub annotations: Vec<WAILAnnotation>,
}

#[derive(Debug, Clone)]
pub struct WAILObjectDef<'a> {
    pub name: String,
    pub fields: Vec<WAILField<'a>>,
}

#[derive(Debug, Clone)]
pub struct WAILTemplateDef<'a> {
    pub name: String,
    pub inputs: Vec<WAILField<'a>>,
    pub output: WAILField<'a>,
    pub prompt_template: String,
    pub annotations: Vec<WAILAnnotation>,
}

#[derive(Debug, Clone)]
pub struct WAILMainDef<'a> {
    pub statements: Vec<MainStatement>,
    pub prompt: String,
    pub _phantom: PhantomData<&'a ()>,
}

impl TemplateArgument {
    pub fn to_string(&self) -> String {
        match self {
            TemplateArgument::String(s) => s.clone(),
            TemplateArgument::Number(n) => n.to_string(),
            TemplateArgument::Float(f) => f.to_string(),
            TemplateArgument::TypeRef(t) => t.clone(),
        }
    }
}

impl<'a> WAILMainDef<'a> {
    pub fn new(statements: Vec<MainStatement>, prompt: String) -> Self {
        WAILMainDef {
            statements,
            prompt,
            _phantom: PhantomData,
        }
    }

    pub fn interpolate_prompt(
        &self,
        template_registry: &HashMap<String, WAILTemplateDef>,
    ) -> Result<String, String> {
        let mut result = self.prompt.clone();
        let re = regex::Regex::new(r"\{\{([^}]+)\}\}").unwrap();

        for cap in re.captures_iter(&self.prompt) {
            let full_match = cap[0].to_string();
            let var_name = &cap[1];

            // Find the template call for this variable
            let template_call = self
                .statements
                .iter()
                .find_map(|stmt| match stmt {
                    MainStatement::Assignment {
                        variable,
                        template_call,
                    } if variable == var_name => Some(template_call),
                    _ => None,
                })
                .ok_or_else(|| format!("No template call found for variable: {}", var_name))?;

            // Look up the template
            let template = template_registry
                .get(&template_call.template_name)
                .ok_or_else(|| format!("Template not found: {}", template_call.template_name))?;

            // Replace the placeholder with the template's prompt
            result = result.replace(&full_match, &template.prompt_template);
        }

        Ok(result)
    }

    pub fn validate_llm_response(
        &self,
        json: &JsonValue,
        registry: &HashMap<String, WAILField<'a>>,
    ) -> Result<(), String> {
        // For each template call in statements, validate its output
        for statement in &self.statements {
            match statement {
                MainStatement::Assignment {
                    variable,
                    template_call,
                } => {
                    // Get the template's output type from registry
                    let template_output =
                        registry.get(&template_call.template_name).ok_or_else(|| {
                            format!("Template not found: {}", template_call.template_name)
                        })?;

                    // Get the corresponding value from JSON response
                    let value = match json {
                        JsonValue::Object(map) => map.get(variable).ok_or_else(|| {
                            format!("Missing output for template call: {}", variable)
                        })?,
                        _ => return Err("Expected object response from LLM".to_string()),
                    };

                    // Validate the value against the template's output type
                    template_output.field_type.validate_json(value)?;
                }
                MainStatement::TemplateCall(template_call) => {
                    // Similar validation for direct template calls
                    let template_output =
                        registry.get(&template_call.template_name).ok_or_else(|| {
                            format!("Template not found: {}", template_call.template_name)
                        })?;
                    template_output.field_type.validate_json(json)?;
                }
            }
        }
        Ok(())
    }
}

impl MainStatement {
    pub fn as_template_call(&self) -> Option<&WAILTemplateCall> {
        match self {
            MainStatement::TemplateCall(call) => Some(call),
            _ => None,
        }
    }

    pub fn as_assignment(&self) -> Option<(&String, &WAILTemplateCall)> {
        match self {
            MainStatement::Assignment {
                variable,
                template_call,
            } => Some((variable, template_call)),
            _ => None,
        }
    }
}

impl<'a> WAILTemplateDef<'a> {
    pub fn interpolate_prompt(&self) -> Result<String, String> {
        let mut prompt = self.prompt_template.clone();

        // Replace input parameters with their schema
        for input in &self.inputs {
            println!("input: {}", input.name);
            let placeholder = format!("{{{{{}}}}}", input.name);
            if !prompt.contains(&placeholder) {
                return Err(format!("Missing placeholder for input: {}", input.name));
            }
            prompt = prompt.replace(&placeholder, &input.field_type.to_schema());
        }

        // Replace return type with schema
        let return_type_schema = self.output.field_type.to_schema();
        prompt = prompt.replace("{{return_type}}", &return_type_schema);

        Ok(prompt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json_types::Number;

    #[test]
    fn test_validate_llm_response() {
        // Create Person type
        let person_fields = vec![
            WAILField {
                name: "name".to_string(),
                field_type: WAILType::Simple(WAILSimpleType::String(WAILString {
                    value: String::new(),
                    type_data: WAILTypeData {
                        json_type: JsonValue::String(String::new()),
                        type_name: "String",
                        field_definitions: None,
                        element_type: None,
                    },
                })),
                annotations: vec![],
            },
            WAILField {
                name: "age".to_string(),
                field_type: WAILType::Simple(WAILSimpleType::Number(WAILNumber::Integer(
                    WAILInteger {
                        value: 0,
                        type_data: WAILTypeData {
                            json_type: JsonValue::Number(Number::Integer(0)),
                            type_name: "Number",
                            field_definitions: None,
                            element_type: None,
                        },
                    },
                ))),
                annotations: vec![],
            },
        ];

        let person_type = WAILField {
            name: "GetPersonFromDescription".to_string(),
            field_type: WAILType::Composite(WAILCompositeType::Object(WAILObject {
                value: HashMap::new(),
                type_data: WAILTypeData {
                    json_type: JsonValue::Object(HashMap::new()),
                    type_name: "Person",
                    field_definitions: Some(person_fields),
                    element_type: None,
                },
            })),
            annotations: vec![],
        };

        // Create registry with the template
        let mut registry = HashMap::new();
        registry.insert("GetPersonFromDescription".to_string(), person_type);

        // Create main block with two template calls
        let main = WAILMainDef::new(
            vec![
                MainStatement::Assignment {
                    variable: "person1".to_string(),
                    template_call: WAILTemplateCall {
                        template_name: "GetPersonFromDescription".to_string(),
                        arguments: {
                            let mut args = HashMap::new();
                            args.insert(
                                "description".to_string(),
                                TemplateArgument::String("John is 30 years old".to_string()),
                            );
                            args
                        },
                    },
                },
                MainStatement::Assignment {
                    variable: "person2".to_string(),
                    template_call: WAILTemplateCall {
                        template_name: "GetPersonFromDescription".to_string(),
                        arguments: {
                            let mut args = HashMap::new();
                            args.insert(
                                "description".to_string(),
                                TemplateArgument::String("Jane is 25 years old".to_string()),
                            );
                            args
                        },
                    },
                },
            ],
            "Here are the people: {{person1}} and {{person2}}".to_string(),
        );

        // Test valid response
        let mut valid_response = HashMap::new();
        valid_response.insert("person1".to_string(), {
            let mut p1 = HashMap::new();
            p1.insert("name".to_string(), JsonValue::String("John".to_string()));
            p1.insert("age".to_string(), JsonValue::Number(Number::Integer(30)));
            JsonValue::Object(p1)
        });
        valid_response.insert("person2".to_string(), {
            let mut p2 = HashMap::new();
            p2.insert("name".to_string(), JsonValue::String("Jane".to_string()));
            p2.insert("age".to_string(), JsonValue::Number(Number::Integer(25)));
            JsonValue::Object(p2)
        });

        assert!(main
            .validate_llm_response(&JsonValue::Object(valid_response), &registry)
            .is_ok());

        // Test invalid response - missing person2
        let mut invalid_response = HashMap::new();
        invalid_response.insert("person1".to_string(), {
            let mut p1 = HashMap::new();
            p1.insert("name".to_string(), JsonValue::String("John".to_string()));
            p1.insert("age".to_string(), JsonValue::Number(Number::Integer(30)));
            JsonValue::Object(p1)
        });

        assert!(main
            .validate_llm_response(&JsonValue::Object(invalid_response), &registry)
            .is_err());

        // Test invalid response - wrong type for age
        let mut invalid_response = HashMap::new();
        invalid_response.insert("person1".to_string(), {
            let mut p1 = HashMap::new();
            p1.insert("name".to_string(), JsonValue::String("John".to_string()));
            p1.insert("age".to_string(), JsonValue::String("30".to_string()));
            JsonValue::Object(p1)
        });
        invalid_response.insert("person2".to_string(), {
            let mut p2 = HashMap::new();
            p2.insert("name".to_string(), JsonValue::String("Jane".to_string()));
            p2.insert("age".to_string(), JsonValue::Number(Number::Integer(25)));
            JsonValue::Object(p2)
        });

        assert!(main
            .validate_llm_response(&JsonValue::Object(invalid_response), &registry)
            .is_err());

        // Test invalid response - not an object
        assert!(main
            .validate_llm_response(&JsonValue::String("not an object".to_string()), &registry)
            .is_err());
    }
}
