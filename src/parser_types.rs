use crate::json_types::JsonValue;
use crate::types::*;
use regex;
use std::collections::HashMap;
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub enum WAILAnnotation {
    Description(String), // Detailed explanation of purpose/meaning
    Example(String),     // Concrete examples of valid values/usage
    Validation(String),  // Rules about what makes a valid value
    Format(String),      // Expected text format or structure
    Important(String),   // Critical information the LLM should pay special attention to
    Context(String),     // Additional context about where/how this is used
    Default(String),     // Default/fallback value if not specified
    Field {
        // Field level annotations
        name: String,
        annotations: Vec<WAILAnnotation>,
    },
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

        // Handle input parameters
        for input in &self.inputs {
            let placeholder = format!("{{{{{}}}}}", input.name);
            if !prompt.contains(&placeholder) {
                return Err(format!("Missing placeholder for input: {}", input.name));
            }

            if let Some(arguments) = arguments {
                let argument = arguments.get(&input.name).unwrap();
                prompt = prompt.replace(&placeholder, &argument.to_string());
            } else {
                let mut param_info = String::new();

                // Add schema
                param_info.push_str(&input.field_type.to_schema());

                // Group annotations by field
                let mut field_annotations: HashMap<String, Vec<&WAILAnnotation>> = HashMap::new();
                let mut general_annotations = Vec::new();

                for annotation in &input.annotations {
                    match annotation {
                        WAILAnnotation::Field { name, annotations } => {
                            field_annotations
                                .entry(name.clone())
                                .or_default()
                                .extend(annotations.iter());
                        }
                        _ => general_annotations.push(annotation),
                    }
                }

                // Add general annotations
                if !general_annotations.is_empty() {
                    param_info.push_str("\n# General:\n");
                    for annotation in &general_annotations {
                        match annotation {
                            WAILAnnotation::Description(desc) => {
                                param_info.push_str(&format!("# {}\n", desc));
                            }
                            WAILAnnotation::Example(ex) => {
                                param_info.push_str(&format!("# Example: {}\n", ex));
                            }
                            WAILAnnotation::Validation(rule) => {
                                param_info.push_str(&format!("# Validation: {}\n", rule));
                            }
                            WAILAnnotation::Format(fmt) => {
                                param_info.push_str(&format!("# Format: {}\n", fmt));
                            }
                            WAILAnnotation::Important(note) => {
                                param_info.push_str(&format!("# Important: {}\n", note));
                            }
                            WAILAnnotation::Context(ctx) => {
                                param_info.push_str(&format!("# Context: {}\n", ctx));
                            }
                            WAILAnnotation::Default(def) => {
                                param_info.push_str(&format!("# Default: {}\n", def));
                            }
                            WAILAnnotation::Field { .. } => unreachable!(),
                        }
                    }
                }

                // Add field-specific annotations
                if !field_annotations.is_empty() {
                    param_info.push_str("\n# Field Requirements:\n");
                    for (field_name, annotations) in field_annotations {
                        param_info.push_str(&format!("# For {}:\n", field_name));
                        for annotation in annotations {
                            match annotation {
                                WAILAnnotation::Description(desc) => {
                                    param_info.push_str(&format!("#   {}\n", desc));
                                }
                                WAILAnnotation::Example(ex) => {
                                    param_info.push_str(&format!("#   Example: {}\n", ex));
                                }
                                WAILAnnotation::Validation(rule) => {
                                    param_info.push_str(&format!("#   Validation: {}\n", rule));
                                }
                                WAILAnnotation::Format(fmt) => {
                                    param_info.push_str(&format!("#   Format: {}\n", fmt));
                                }
                                WAILAnnotation::Important(note) => {
                                    param_info.push_str(&format!("#   Important: {}\n", note));
                                }
                                WAILAnnotation::Context(ctx) => {
                                    param_info.push_str(&format!("#   Context: {}\n", ctx));
                                }
                                WAILAnnotation::Default(def) => {
                                    param_info.push_str(&format!("#   Default: {}\n", def));
                                }
                                WAILAnnotation::Field { .. } => unreachable!(),
                            }
                        }
                    }
                }

                prompt = prompt.replace(&placeholder, &param_info);
            }
        }

        // Handle return type with proper indentation
        let re = regex::Regex::new(r"\{\{return_type\}\}").unwrap();
        if let Some(cap) = re.find(&prompt) {
            // Get the line containing return_type
            let line_start = prompt[..cap.start()].rfind('\n').map_or(0, |i| i + 1);
            let indent = count_leading_whitespace(&prompt[line_start..cap.start()]);

            let mut return_info = String::new();
            return_info.push_str(&self.output.field_type.to_schema());

            // Group annotations by field for return type
            let mut field_annotations: HashMap<String, Vec<&WAILAnnotation>> = HashMap::new();
            let mut general_annotations = Vec::new();

            for annotation in &self.output.annotations {
                match annotation {
                    WAILAnnotation::Field { name, annotations } => {
                        field_annotations
                            .entry(name.clone())
                            .or_default()
                            .extend(annotations.iter());
                    }
                    _ => general_annotations.push(annotation),
                }
            }

            // Add general annotations for return type
            if !general_annotations.is_empty() {
                return_info.push_str("\n# General:\n");
                for annotation in &general_annotations {
                    match annotation {
                        WAILAnnotation::Description(desc) => {
                            return_info.push_str(&format!("# {}\n", desc));
                        }
                        WAILAnnotation::Example(ex) => {
                            return_info.push_str(&format!("# Example: {}\n", ex));
                        }
                        WAILAnnotation::Validation(rule) => {
                            return_info.push_str(&format!("# Validation: {}\n", rule));
                        }
                        WAILAnnotation::Format(fmt) => {
                            return_info.push_str(&format!("# Format: {}\n", fmt));
                        }
                        WAILAnnotation::Important(note) => {
                            return_info.push_str(&format!("# Important: {}\n", note));
                        }
                        WAILAnnotation::Context(ctx) => {
                            return_info.push_str(&format!("# Context: {}\n", ctx));
                        }
                        WAILAnnotation::Default(def) => {
                            return_info.push_str(&format!("# Default: {}\n", def));
                        }
                        WAILAnnotation::Field { .. } => unreachable!(),
                    }
                }
            }

            // Add field-specific annotations for return type
            if !field_annotations.is_empty() {
                return_info.push_str("\n# Field Requirements:\n");
                for (field_name, annotations) in field_annotations {
                    return_info.push_str(&format!("# For {}:\n", field_name));
                    for annotation in annotations {
                        match annotation {
                            WAILAnnotation::Description(desc) => {
                                return_info.push_str(&format!("#   {}\n", desc));
                            }
                            WAILAnnotation::Example(ex) => {
                                return_info.push_str(&format!("#   Example: {}\n", ex));
                            }
                            WAILAnnotation::Validation(rule) => {
                                return_info.push_str(&format!("#   Validation: {}\n", rule));
                            }
                            WAILAnnotation::Format(fmt) => {
                                return_info.push_str(&format!("#   Format: {}\n", fmt));
                            }
                            WAILAnnotation::Important(note) => {
                                return_info.push_str(&format!("#   Important: {}\n", note));
                            }
                            WAILAnnotation::Context(ctx) => {
                                return_info.push_str(&format!("#   Context: {}\n", ctx));
                            }
                            WAILAnnotation::Default(def) => {
                                return_info.push_str(&format!("#   Default: {}\n", def));
                            }
                            WAILAnnotation::Field { .. } => unreachable!(),
                        }
                    }
                }
            }

            // Apply indentation to all lines including annotations
            let indented_schema = return_info
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
