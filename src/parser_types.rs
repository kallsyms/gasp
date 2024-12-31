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
pub struct WAILUnionDef<'a> {
    pub name: String,
    pub members: Vec<WAILType<'a>>,
}

#[derive(Debug, Clone)]
pub enum TemplateArgument {
    String(String),
    Number(i64),
    Float(f64),
    TypeRef(String), // For when we reference a type like "String" or "Number"
    TemplateArgRef(String),
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
    Comment(String),
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
    pub template_args: HashMap<String, WAILType<'a>>,
    pub _phantom: PhantomData<&'a ()>,
}

impl TemplateArgument {
    pub fn to_string(&self) -> String {
        match self {
            TemplateArgument::String(s) => s.clone(),
            TemplateArgument::Number(n) => n.to_string(),
            TemplateArgument::Float(f) => f.to_string(),
            TemplateArgument::TypeRef(t) => t.clone(),
            TemplateArgument::TemplateArgRef(t) => format!("${}", t),
        }
    }
}

impl<'a> WAILMainDef<'a> {
    pub fn new(
        statements: Vec<MainStatement>,
        prompt: String,
        template_args: Option<HashMap<String, WAILType<'a>>>,
    ) -> Self {
        WAILMainDef {
            statements,
            prompt,
            template_args: template_args.unwrap_or_default(),
            _phantom: PhantomData,
        }
    }

    pub fn interpolate_prompt(
        &self,
        template_registry: &HashMap<String, WAILTemplateDef>,
        template_arg_values: Option<&HashMap<String, JsonValue>>,
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
            result = result.replace(
                &full_match,
                &template
                    .interpolate_prompt(Some(&template_call.arguments))
                    .unwrap(),
            );
        }

        if let Some(arg_values) = template_arg_values {
            for (name, value) in arg_values {
                let value_str = if let Some(s) = value.as_string() {
                    s.to_string()
                } else {
                    value.to_string()
                };
                result = result.replace(&format!("${}", name), &value_str);
            }
        }

        Ok(result)
    }

    pub fn validate_llm_response(
        &self,
        json: &JsonValue,
        registry: &HashMap<String, WAILTemplateDef<'a>>,
    ) -> Result<(), String> {
        // For each template call in statements, validate its output
        for statement in &self.statements {
            match statement {
                MainStatement::Assignment {
                    variable,
                    template_call,
                } => {
                    // Get the template's output type from registry
                    let template = registry.get(&template_call.template_name).ok_or_else(|| {
                        format!("Template not found: {}", template_call.template_name)
                    })?;

                    let template_output = &template.output;

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
                    let template = registry.get(&template_call.template_name).ok_or_else(|| {
                        format!("Template not found: {}", template_call.template_name)
                    })?;

                    // Get the corresponding value from JSON response
                    let value = match json {
                        JsonValue::Object(map) => {
                            map.get(&template_call.template_name).ok_or_else(|| {
                                format!(
                                    "Missing output for template call: {}",
                                    template_call.template_name
                                )
                            })?
                        }
                        _ => return Err("Expected object response from LLM".to_string()),
                    };

                    let template_output = &template.output;
                    println!("Validating: {:?}", template_output.field_type);
                    println!("Value: {:?}", value);
                    template_output.field_type.validate_json(value)?;
                }
                MainStatement::Comment(_) => {}
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

fn count_leading_whitespace(s: &str) -> usize {
    s.chars().take_while(|c| c.is_whitespace()).count()
}

impl<'a> WAILTemplateDef<'a> {
    pub fn interpolate_prompt(
        &self,
        arguments: Option<&HashMap<String, TemplateArgument>>,
    ) -> Result<String, String> {
        let mut prompt = self.prompt_template.clone();

        // Replace input parameters with their schema type or actual values
        for input in &self.inputs {
            let placeholder = format!("{{{{{}}}}}", input.name);
            if !prompt.contains(&placeholder) {
                return Err(format!("Missing placeholder for input: {}", input.name));
            }

            if let Some(arguments) = arguments {
                let argument = arguments.get(&input.name).unwrap();
                prompt = prompt.replace(&placeholder, &argument.to_string());
            } else {
                prompt = prompt.replace(&placeholder, &input.field_type.to_schema());
            }
        }

        // Handle return type with proper indentation
        let re = regex::Regex::new(r"\{\{return_type\}\}").unwrap();
        if let Some(cap) = re.find(&prompt) {
            // Get the line containing return_type
            let line_start = prompt[..cap.start()].rfind('\n').map_or(0, |i| i + 1);
            let indent = count_leading_whitespace(&prompt[line_start..cap.start()]);

            let return_type_schema = self.output.field_type.to_schema();
            let indented_schema = return_type_schema
                .lines()
                .enumerate()
                .map(|(i, line)| {
                    if i == 0 {
                        line.to_string()
                    } else {
                        format!("{}{}", " ".repeat(indent), line)
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");

            let return_prompt = format!(
                "\nReturn a JSON-like format wrapped in ```gasp fences:\n\n{}",
                indented_schema
            );

            prompt = re.replace(&prompt, &return_prompt).to_string();
        }

        Ok(prompt)
    }
}

#[cfg(test)]
mod tests {
    use crate::wail_parser::WAILParser;

    #[test]
    fn test_parse_llm_output() {
        let wail_schema = r#"
    object Person {
        name: String
        age: Number
        interests: String[]
    }

    template GetPerson(description: String) -> Person {
        prompt: """{{description}}"""
    }

    main {
        let person = GetPerson(description: "test");
        prompt { {{person}} }
    }"#;

        let parser = WAILParser::new();
        parser.parse_wail_file(wail_schema).unwrap();

        // Test relaxed JSON parsing features
        let cases = vec![
            // Unquoted keys
            r#"{"person": {name: "Alice", age: 25, interests: ["coding"]}}"#,
            // Single quotes
            r#"{'person': {'name': 'Alice', 'age': 25, 'interests': ['coding']}}"#,
            // Trailing commas
            r#"{"person": {"name": "Alice", "age": 25, "interests": ["coding",],}}"#,
            // Mixed quotes and unquoted identifiers
            r#"{"person": {name: 'Alice', "age": 25, interests: ["coding"]}}"#,
        ];

        for case in cases {
            assert!(
                parser.parse_llm_output(case).is_ok(),
                "Failed to parse: {}",
                case
            );
        }
    }
}
