use crate::json_types::{JsonError, JsonValue, Number};
use crate::parser_types::*;
use crate::rd_json_stack_parser::Parser as JsonParser;
use crate::types::*;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while1},
    character::complete::{alpha1, char, multispace0, multispace1},
    combinator::opt,
    multi::{many0, separated_list0},
    sequence::{delimited, preceded, tuple},
    IResult,
};
use std::cell::RefCell;
use std::collections::HashMap;

fn count_leading_whitespace(line: &str) -> usize {
    line.chars().take_while(|c| c.is_whitespace()).count()
}

fn adjust_indentation(content: &str, target_indent: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return String::new();
    }

    // Find initial whitespace amount from first non-empty line
    let initial_indent = lines
        .iter()
        .find(|line| !line.trim().is_empty())
        .map(|line| count_leading_whitespace(line))
        .unwrap_or(0);

    // Calculate how much to adjust by
    let indent_adjustment = initial_indent.saturating_sub(target_indent);

    // Adjust each line
    lines
        .iter()
        .map(|line| {
            let current_indent = count_leading_whitespace(line);
            if current_indent >= indent_adjustment {
                &line[indent_adjustment..]
            } else {
                line.trim_start()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug)]
pub struct WAILParser<'a> {
    registry: RefCell<HashMap<String, WAILField<'a>>>,
    template_registry: RefCell<HashMap<String, WAILTemplateDef<'a>>>,
    main: RefCell<Option<WAILMainDef<'a>>>,
}

impl<'a> WAILParser<'a> {
    pub fn new() -> Self {
        Self {
            registry: RefCell::new(HashMap::new()),
            template_registry: RefCell::new(HashMap::new()),
            main: RefCell::new(None),
        }
    }
    fn parse_json_like_segment(&'a self, input: &'a str) -> IResult<&'a str, String> {
        let (input, _) = multispace0(input)?;

        if input.is_empty() {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Eof,
            )));
        }

        // Skip any non-JSON text until we find a JSON-like start
        let (input, _) = alt((
            take_until("{"),
            take_while1(|c| c != '{'),
            preceded(multispace0, take_until("{")),
            preceded(multispace1, take_while1(|c| c != '{')),
        ))(input)?;

        let mut depth = 0;
        let mut json_chars = Vec::new();
        let mut chars = input.chars().peekable();
        let mut pos = 0;

        while let Some(&c) = chars.peek() {
            match c {
                '{' => {
                    depth += 1;
                    json_chars.push(c);
                }
                '}' => {
                    depth -= 1;
                    json_chars.push(c);
                    if depth == 0 {
                        pos += 1;
                        break;
                    }
                }
                _ => json_chars.push(c),
            }
            chars.next();
            pos += 1;
        }

        let json_str: String = json_chars.into_iter().collect();
        Ok((&input[pos..], json_str))
    }

    pub fn parse_llm_output(&'a self, input: &'a str) -> Result<JsonValue, String> {
        // First get the ordered list of variable names from main statements
        let var_names: Vec<String> = self
            .main
            .borrow()
            .as_ref()
            .unwrap()
            .statements
            .iter()
            .filter_map(|stmt| match stmt {
                MainStatement::Assignment { variable, .. } => Some(variable.clone()),
                _ => None,
            })
            .collect();

        let direct_calls: Vec<String> = self
            .main
            .borrow()
            .as_ref()
            .unwrap()
            .statements
            .iter()
            .filter_map(|stmt| match stmt {
                MainStatement::TemplateCall(call) => Some(call.template_name.clone()),
                _ => None,
            })
            .collect();

        // Parse the JSON-like segments
        let (_, segments) = many0(|input| self.parse_json_like_segment(input))(input)
            .map_err(|e| format!("Failed to parse segments: {:?}", e))?;

        if segments.len() != var_names.len() {
            return Err(format!(
                "Found {} JSON segments but expected {} based on template variables",
                segments.len(),
                var_names.len()
            ));
        }

        // Try to parse each segment and build the map
        let mut result = HashMap::new();
        for (var_name, segment) in var_names.into_iter().zip(segments) {
            let mut parser = JsonParser::new(segment.as_bytes().to_vec());
            let json_value = parser
                .parse()
                .map_err(|e| format!("Failed to parse JSON for {}: {}", var_name, e))?;
            result.insert(var_name, json_value);
        }

        Ok(JsonValue::Object(result))
    }

    pub fn prepare_prompt(&self) -> String {
        self.main
            .borrow()
            .as_ref()
            .unwrap()
            .interpolate_prompt(&self.template_registry.borrow())
            .unwrap()
    }

    pub fn parse_wail_file(&'a self, input: &'a str) -> IResult<&'a str, Vec<WAILDefinition<'a>>> {
        let (input, _) = multispace0(input)?;
        let (input, mut definitions) = separated_list0(
            multispace1,
            alt((
                |input| self.parse_definition(input),
                |input| self.parse_comment(input),
            )),
        )(input)?;
        let (input, _) = multispace0(input)?;

        // Parse optional main block at the end
        if let Ok((remaining, main_def)) = self.parse_main(input) {
            definitions.push(WAILDefinition::Main(main_def));
            Ok((remaining, definitions))
        } else {
            Ok((input, definitions))
        }
    }

    fn parse_comment(&'a self, input: &'a str) -> IResult<&'a str, WAILDefinition<'a>> {
        let (input, _) = tuple((multispace0, tag("#"), multispace0, take_until("\n")))(input)?;
        Ok((input, WAILDefinition::Comment(input.to_string())))
    }

    fn parse_object(&'a self, input: &'a str) -> IResult<&'a str, WAILField<'a>> {
        // Parse: Object Name { ... }
        let (input, _) = tuple((tag("object"), multispace1))(input)?;
        let (input, name) = self.identifier(input)?;
        let (input, _) = multispace0(input)?;
        let (input, fields) = delimited(
            char('{'),
            many0(delimited(multispace0, |i| self.parse_field(i), multispace0)),
            char('}'),
        )(input)?;

        // Convert fields into HashMap
        let mut field_map = HashMap::new();
        for field in &fields {
            field_map.insert(
                WAILString {
                    value: field.name.clone(),
                    type_data: WAILTypeData {
                        json_type: JsonValue::String(field.name.clone()),
                        type_name: "String",
                        field_definitions: None,
                        element_type: None,
                    },
                },
                field.field_type.clone(),
            );
        }

        let object = WAILObject {
            value: field_map,
            type_data: WAILTypeData {
                json_type: JsonValue::Object(HashMap::new()), // Placeholder empty object
                type_name: name,
                field_definitions: Some(fields),
                element_type: None,
            },
        };

        let field = WAILField {
            name: name.to_string(),
            field_type: WAILType::Composite(WAILCompositeType::Object(object)),
            annotations: Vec::new(),
        };

        self.registry
            .borrow_mut()
            .insert(name.to_string(), field.clone());

        Ok((input, field))
    }

    fn parse_field(&'a self, input: &'a str) -> IResult<&str, WAILField> {
        let (input, (name, _, _, (field_type, _))) = tuple((
            |i| self.identifier(i),
            char(':'),
            multispace0,
            |i| self.parse_type(i),
        ))(input)?;

        Ok((
            input,
            WAILField {
                name: name.to_string(),
                field_type,
                annotations: Vec::new(), // For now, we'll add annotation parsing later
            },
        ))
    }
    fn parse_type(&'a self, input: &'a str) -> IResult<&str, (WAILType<'a>, String)> {
        let (input, base_type) = self.identifier(input)?;
        let (input, _) = multispace0(input)?;
        let (input, is_array) = opt(tag("[]"))(input)?;

        // Create the base type
        let base_type_val = match base_type {
            "String" => WAILType::Simple(WAILSimpleType::String(WAILString {
                value: String::new(),
                type_data: WAILTypeData {
                    json_type: JsonValue::String(String::new()),
                    type_name: base_type,
                    field_definitions: None,
                    element_type: None,
                },
            })),
            "Number" => {
                WAILType::Simple(WAILSimpleType::Number(WAILNumber::Integer(WAILInteger {
                    value: 0,
                    type_data: WAILTypeData {
                        json_type: JsonValue::Number(Number::Integer(0)),
                        type_name: base_type,
                        field_definitions: None,
                        element_type: None,
                    },
                })))
            }
            // Handle both registered and unregistered types
            _ => {
                if let Some(field) = self.registry.borrow().get(base_type) {
                    field.field_type.clone()
                } else {
                    // Create a placeholder object type for unregistered types
                    WAILType::Composite(WAILCompositeType::Object(WAILObject {
                        value: HashMap::new(),
                        type_data: WAILTypeData {
                            json_type: JsonValue::Object(HashMap::new()),
                            type_name: base_type,
                            field_definitions: None,
                            element_type: None,
                        },
                    }))
                }
            }
        };

        // Handle array type wrapping
        if is_array.is_some() {
            Ok((
                input,
                (
                    WAILType::Composite(WAILCompositeType::Array(WAILArray {
                        values: vec![],
                        type_data: WAILTypeData {
                            json_type: JsonValue::Array(vec![]),
                            type_name: "Array",
                            field_definitions: None,
                            element_type: Some(base_type_val.type_data().type_name),
                        },
                    })),
                    "".to_string(),
                ),
            ))
        } else {
            Ok((input, (base_type_val, base_type.to_string())))
        }
    }

    fn identifier(&'a self, input: &'a str) -> IResult<&'a str, &'a str> {
        take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)
    }

    fn parse_template(&'a self, input: &'a str) -> IResult<&'a str, WAILTemplateDef> {
        // Parse: template Name(param: Type) -> ReturnType { prompt: """ ... """ }
        let (input, _) = tuple((tag("template"), multispace1))(input)?;

        // Parse template name
        let (input, name) = self.identifier(input)?;

        let (input, _) = multispace0(input)?;

        // Parse parameters in parentheses
        let (input, params) = delimited(
            char('('),
            preceded(
                multispace0,
                separated_list0(tuple((multispace0, char(','), multispace0)), |i| {
                    self.parse_parameter(i)
                }),
            ),
            preceded(multispace0, char(')')),
        )(input)?;

        // Parse return type
        let (input, _) = tuple((multispace0, tag("->"), multispace0))(input)?;

        let (input, (return_type, identifier)) = self.parse_type(input)?;

        let (input, _) = multispace0(input)?;
        let (input, annotations) = many0(|input| self.parse_annotation(input))(input)?;

        // Parse template body with prompt template
        let (input, _) = tuple((multispace0, char('{'), multispace0))(input)?;
        let (input, _) = tuple((tag("prompt:"), multispace0))(input)?;
        let (input, template) =
            delimited(tag(r#"""""#), take_until(r#"""""#), tag(r#"""""#))(input)?;

        let template_adjusted = adjust_indentation(&template, 0);

        let (input, _) = tuple((multispace0, char('}')))(input)?;

        // Create output field for both registered and unregistered types
        let output_field = WAILField {
            name: identifier.clone(),
            field_type: return_type,
            annotations: vec![],
        };

        let template_def = WAILTemplateDef {
            name: name.to_string(),
            inputs: params,
            output: output_field,
            prompt_template: template_adjusted,
            annotations,
        };

        self.template_registry
            .borrow_mut()
            .insert(name.to_string(), template_def.clone());

        Ok((input, template_def))
    }

    fn parse_template_call(&'a self, input: &'a str) -> IResult<&'a str, WAILTemplateCall> {
        let (input, template_name) = self.identifier(input)?;
        let (input, _) = tuple((multispace0, char('('), multispace0))(input)?;

        // Parse arguments as key-value pairs
        let (input, args) = separated_list0(tuple((multispace0, char(','), multispace0)), |i| {
            self.parse_argument(i)
        })(input)?;

        let (input, _) = tuple((multispace0, char(')')))(input)?;

        let mut arguments = HashMap::new();
        for (name, value) in args {
            arguments.insert(name, value);
        }

        Ok((
            input,
            WAILTemplateCall {
                template_name: template_name.to_string(),
                arguments,
            },
        ))
    }

    fn parse_string_literal(&'a self, input: &'a str) -> IResult<&'a str, TemplateArgument> {
        let (input, _) = char('"')(input)?;
        let (input, content) = take_until("\"")(input)?;
        let (input, _) = char('"')(input)?;
        Ok((input, TemplateArgument::String(content.to_string())))
    }

    fn parse_number(&'a self, input: &'a str) -> IResult<&'a str, TemplateArgument> {
        let (input, num_str) = take_while1(|c: char| c.is_ascii_digit())(input)?;
        let num = num_str.parse::<i64>().map_err(|_| {
            nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit))
        })?;
        Ok((input, TemplateArgument::Number(num)))
    }

    fn parse_type_ref(&'a self, input: &'a str) -> IResult<&'a str, TemplateArgument> {
        let (input, type_name) = self.identifier(input)?;
        Ok((input, TemplateArgument::TypeRef(type_name.to_string())))
    }

    fn parse_value(&'a self, input: &'a str) -> IResult<&'a str, TemplateArgument> {
        alt((
            |i| self.parse_string_literal(i),
            |i| self.parse_number(i),
            |i| self.parse_type_ref(i),
        ))(input)
    }

    fn parse_argument(&'a self, input: &'a str) -> IResult<&'a str, (String, TemplateArgument)> {
        let (input, name) = self.identifier(input)?;
        let (input, _) = tuple((multispace0, char(':'), multispace0))(input)?;
        let (input, value) = self.parse_value(input)?;
        Ok((input, (name.to_string(), value)))
    }

    fn parse_main(&'a self, input: &'a str) -> IResult<&'a str, WAILMainDef<'a>> {
        if self.main.borrow().is_some() {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Alt,
            )));
        }

        // Parse main opening
        let (input, _) = tuple((tag("main"), multispace0, char('{'), multispace0))(input)?;

        // Parse statements (assignments and template calls)
        let (input, statements) = many0(|i| {
            let (i, statement) = alt((
                |input| {
                    // Parse assignment: let var = template_call;
                    let (input, _) = tuple((multispace0, tag("let"), multispace1))(input)?;
                    let (input, var_name) = self.identifier(input)?;
                    let (input, _) = tuple((multispace0, char('='), multispace0))(input)?;
                    let (input, template_call) = self.parse_template_call(input)?;
                    let (input, _) = tuple((multispace0, char(';'), multispace0))(input)?;
                    Ok((
                        input,
                        MainStatement::Assignment {
                            variable: var_name.to_string(),
                            template_call,
                        },
                    ))
                },
                |input| {
                    // Parse regular template call: template_call;
                    let (input, template_call) = self.parse_template_call(input)?;
                    let (input, _) = tuple((multispace0, char(';'), multispace0))(input)?;
                    Ok((input, MainStatement::TemplateCall(template_call)))
                },
                |input: &'a str| {
                    // Parse comment: # comment
                    let (input, (_, _, _, comment)) =
                        tuple((multispace0, tag("#"), multispace0, take_until("\n")))(input)?;

                    Ok((input, MainStatement::Comment(comment.to_string())))
                },
            ))(i)?;
            Ok((i, statement))
        })(input)?;

        // Parse prompt block
        let (input, _) = tuple((
            multispace0,
            tag("prompt"),
            multispace0,
            char('{'),
            // multispace0,
        ))(input)?;

        // Take everything until the closing brace of prompt, handling nested braces
        let mut brace_count = 1;
        let mut prompt_end = 0;
        let chars: Vec<_> = input.chars().collect();

        for (i, &c) in chars.iter().enumerate() {
            match c {
                '{' => brace_count += 1,
                '}' => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        prompt_end = i;
                        break;
                    }
                }
                _ => {}
            }
        }

        let (prompt_str, input) = input.split_at(prompt_end);
        let (input, _) = tuple((char('}'), multispace0))(input)?;

        // Parse main's closing brace
        let (input, _) = tuple((char('}'), multispace0))(input)?;

        let prompt_str_trimmed = prompt_str
            .to_string()
            .lines()
            .map(|line| line.trim())
            .collect::<Vec<&str>>()
            .join("\n");

        let main = WAILMainDef::new(statements, prompt_str_trimmed);

        self.main.borrow_mut().replace(main.clone());

        Ok((input, main))
    }

    fn parse_annotation(&'a self, input: &'a str) -> IResult<&'a str, WAILAnnotation> {
        let (input, _) = tuple((char('@'), tag("description"), char('('), char('"')))(input)?;
        let (input, desc) = take_until("\"")(input)?;
        let (input, _) = char('"')(input)?;
        let (input, _) = char(')')(input)?;
        let (input, _) = multispace0(input)?;

        Ok((input, WAILAnnotation::Description(desc.to_string())))
    }

    fn parse_parameter(&'a self, input: &'a str) -> IResult<&'a str, WAILField> {
        let (input, (name, _, _, (param_type, _))) = tuple((
            |i| self.identifier(i),
            char(':'),
            multispace0,
            |i| self.parse_type(i),
        ))(input)?;

        Ok((
            input,
            WAILField {
                name: name.to_string(),
                field_type: param_type,
                annotations: vec![],
            },
        ))
    }

    fn parse_definition(&'a self, input: &'a str) -> IResult<&'a str, WAILDefinition<'a>> {
        if let Ok((remaining, object)) = self.parse_object(input) {
            Ok((remaining, WAILDefinition::Object(object)))
        } else if let Ok((remaining, template)) = self.parse_template(input) {
            Ok((remaining, WAILDefinition::Template(template)))
        } else {
            Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Alt,
            )))
        }
    }

    pub fn validate_json(&self, json: &str) -> Result<(), String> {
        let mut parser = JsonParser::new(json.as_bytes().to_vec());
        let value = parser.parse().map_err(|e| e.to_string())?;

        self.main
            .borrow()
            .as_ref()
            .unwrap()
            .validate_llm_response(&value, &self.template_registry.borrow())
    }

    pub fn validate(&self) -> (Vec<ValidationWarning>, Vec<ValidationError>) {
        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        let registry = self.registry.borrow();
        let template_registry = self.template_registry.borrow();

        // Check if there's a main block
        if self.main.borrow().is_none() {
            warnings.push(ValidationWarning::NoMainBlock);
        }

        // Check all templates
        for (template_name, template) in template_registry.iter() {
            // Check input parameters
            for param in &template.inputs {
                self.validate_type(
                    &param.field_type,
                    &registry,
                    template_name,
                    &mut warnings,
                    &mut errors,
                    false,
                );
            }

            // Check return type
            self.validate_type(
                &template.output.field_type,
                &registry,
                template_name,
                &mut warnings,
                &mut errors,
                true,
            );
        }

        (warnings, errors)
    }

    fn validate_type(
        &self,
        wail_type: &WAILType,
        registry: &HashMap<String, WAILField>,
        template_name: &str,
        warnings: &mut Vec<ValidationWarning>,
        errors: &mut Vec<ValidationError>,
        is_return_type: bool,
    ) {
        match wail_type {
            WAILType::Simple(_) => (), // Built-in types are always valid
            WAILType::Composite(composite) => match composite {
                WAILCompositeType::Array(array) => {
                    // Check if the element type exists if it's a custom type
                    if let Some(element_type) = &array.type_data.element_type {
                        let element_type_str = element_type.to_string();
                        if !registry.contains_key(&element_type_str)
                            && element_type_str != "String"
                            && element_type_str != "Number"
                        {
                            // For array element types in templates, undefined types are errors
                            errors.push(ValidationError::UndefinedTypeInTemplate {
                                template_name: template_name.to_string(),
                                type_name: element_type_str.clone(),
                                is_return_type,
                            });

                            // Check for possible typos
                            for known_type in registry.keys() {
                                if known_type.len() > 2
                                    && element_type_str.len() > 2
                                    && known_type
                                        .chars()
                                        .zip(element_type_str.chars())
                                        .filter(|(a, b)| a != b)
                                        .count()
                                        <= 2
                                {
                                    warnings.push(ValidationWarning::PossibleTypo {
                                        type_name: element_type_str.clone(),
                                        similar_to: known_type.to_string(),
                                        location: format!(
                                            "array element type in template {}",
                                            template_name
                                        ),
                                    });
                                }
                            }
                        }
                    }
                }
                WAILCompositeType::Object(object) => {
                    let type_name = object.type_data.type_name.to_string();
                    if !registry.contains_key(&type_name)
                        && type_name != "String"
                        && type_name != "Number"
                    {
                        // For return types and input parameters in templates, undefined types are errors
                        errors.push(ValidationError::UndefinedTypeInTemplate {
                            template_name: template_name.to_string(),
                            type_name: type_name.clone(),
                            is_return_type,
                        });

                        // Check for possible typos
                        for known_type in registry.keys() {
                            if known_type.len() > 2
                                && type_name.len() > 2
                                && known_type
                                    .chars()
                                    .zip(type_name.chars())
                                    .filter(|(a, b)| a != b)
                                    .count()
                                    <= 2
                            {
                                warnings.push(ValidationWarning::PossibleTypo {
                                    type_name: type_name.clone(),
                                    similar_to: known_type.to_string(),
                                    location: format!(
                                        "{} type in template {}",
                                        if is_return_type {
                                            "return"
                                        } else {
                                            "parameter"
                                        },
                                        template_name
                                    ),
                                });
                            }
                        }
                    }
                }
                WAILCompositeType::Tool(_) => (), // Tool types are always valid
            },
            WAILType::Value(_) => (), // Literal values are always valid
        }
    }
}

#[derive(Debug, Clone)]
pub enum WAILDefinition<'a> {
    Object(WAILField<'a>),
    Template(WAILTemplateDef<'a>),
    Main(WAILMainDef<'a>),
    Comment(String),
}

#[derive(Debug, Clone)]
pub enum ValidationWarning {
    UndefinedType {
        type_name: String,
        location: String,
    },
    PossibleTypo {
        type_name: String,
        similar_to: String,
        location: String,
    },
    NoMainBlock,
}

#[derive(Debug, Clone)]
pub enum ValidationError {
    UndefinedTypeInTemplate {
        template_name: String,
        type_name: String,
        is_return_type: bool,
    },
    // SyntaxError {
    //     statement: String,
    //     error: String,
    // },
}

// Add test that tries parsing a basic object
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_object() {
        let input = r#"object Person {
            name: String
            age: Number
        }"#;

        let parser = WAILParser::new();

        let (_, object) = parser.parse_object(input).unwrap();
        assert_eq!(
            object
                .field_type
                .type_data()
                .field_definitions
                .as_ref()
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn test_parse_template() {
        // First create a parser
        let mut parser = WAILParser::new();

        // Create and register the DateInfo type
        let date_info_fields = vec![
            WAILField {
                name: "day".to_string(),
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
            WAILField {
                name: "month".to_string(),
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
        ];

        let date_info = WAILObject {
            value: HashMap::new(),
            type_data: WAILTypeData {
                json_type: JsonValue::Object(HashMap::new()),
                type_name: "DateInfo",
                field_definitions: Some(date_info_fields),
                element_type: None,
            },
        };

        let date_info_field = WAILField {
            name: "DateInfo".to_string(),
            field_type: WAILType::Composite(WAILCompositeType::Object(date_info)),
            annotations: vec![],
        };

        parser
            .registry
            .borrow_mut()
            .insert("DateInfo".to_string(), date_info_field);

        // Now parse the template
        let input = r#"template ParseDate(date_string: String) -> DateInfo {
            prompt: """
            Extract structured date information from the following date string.
            Date:
            ---
            {{date_string}}
            ---
            Return a structured result matching: {{return_type}}
            """
        }"#;

        let (_, template) = parser.parse_template(input).unwrap();
        assert_eq!(template.name, "ParseDate");
        assert_eq!(template.inputs.len(), 1);
        assert_eq!(template.inputs[0].name, "date_string");
        assert!(template.prompt_template.contains("{{date_string}}"));
        assert!(template.prompt_template.contains("{{return_type}}"));
    }

    #[test]
    fn test_parse_complex_template() {
        let input = r#"template AnalyzeBookClub(
        discussion_log: String,
        participant_names: String[],
        book_details: BookInfo
    ) -> BookClubAnalysis @description("Analyzes book club discussion patterns") {
        prompt: """
        Analyze the following book club discussion, tracking participation and key themes.

        Book Details:
        {{book_details}}

        Participants:
        {{participant_names}}

        Discussion:
        ---
        {{discussion_log}}
        ---

        Analyze the discussion and return a structured analysis following this format: {{return_type}}

        Focus on:
        - Speaking time per participant
        - Key themes discussed
        - Questions raised
        - Book-specific insights
        """
    }"#;

        let parser = WAILParser::new();

        let (_, template) = parser.parse_template(input).unwrap();

        // Test basic properties
        assert_eq!(template.name, "AnalyzeBookClub");
        assert_eq!(template.inputs.len(), 3);

        // Test input parameters
        let inputs = &template.inputs;
        assert_eq!(inputs[0].name, "discussion_log");
        assert!(matches!(inputs[0].field_type, WAILType::Simple(_)));

        assert_eq!(inputs[1].name, "participant_names");
        assert!(matches!(
            inputs[1].field_type,
            WAILType::Composite(WAILCompositeType::Array(_))
        ));

        assert_eq!(inputs[2].name, "book_details");
        assert!(matches!(
            inputs[2].field_type,
            WAILType::Composite(WAILCompositeType::Object(_))
        ));

        // Test output type
        assert_eq!(template.output.name, "BookClubAnalysis");

        // Test annotation
        assert_eq!(template.annotations.len(), 1);
        assert!(matches!(
            template.annotations[0],
            WAILAnnotation::Description(ref s) if s == "Analyzes book club discussion patterns"
        ));

        // Test template content
        let prompt = &template.prompt_template;
        assert!(prompt.contains("{{discussion_log}}"));
        assert!(prompt.contains("{{participant_names}}"));
        assert!(prompt.contains("{{book_details}}"));
        assert!(prompt.contains("{{return_type}}"));
    }

    #[test]
    fn test_prompt_interpolation() {
        let parser = WAILParser::new();

        // Define DateInfo type
        let fields = vec![
            WAILField {
                name: "day".to_string(),
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
            WAILField {
                name: "month".to_string(),
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
        ];

        let date_info = WAILObject {
            value: HashMap::new(),
            type_data: WAILTypeData {
                json_type: JsonValue::Object(HashMap::new()),
                type_name: "DateInfo",
                field_definitions: Some(fields),
                element_type: None,
            },
        };

        let field = WAILField {
            name: "DateInfo".to_string(),
            field_type: WAILType::Composite(WAILCompositeType::Object(date_info)),
            annotations: vec![],
        };

        parser
            .registry
            .borrow_mut()
            .insert("DateInfo".to_string(), field.clone());

        let template = WAILTemplateDef {
            name: "ParseDate".to_string(),
            inputs: vec![WAILField {
                name: "date_string".to_string(),
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
            }],
            output: field,
            prompt_template: r#"Parse this date: {{date_string}}
Return in this format: {{return_type}}"#
                .to_string(),
            annotations: vec![],
        };

        let mut inputs = HashMap::new();
        inputs.insert(
            "date_string".to_string(),
            TemplateArgument::String("January 1st, 2024".to_string()),
        );

        let final_prompt = template.interpolate_prompt(Some(&inputs)).unwrap();
        println!("Final prompt:\n{}", final_prompt);
    }

    #[test]
    fn test_wail_parsing() {
        let parser = WAILParser::new();

        let input = r#"
        object Person {
            name: String
            age: Number
        }

        template GetPersonFromDescription(description: String) -> Person {
            prompt: """
            Given this description of a person: {{description}}
            Create a Person object with their name and age.
            Return in this format: {{return_type}}
            """
        }

        main {
            let person1_template = GetPersonFromDescription(description: "John Doe is 30 years old");
            let person2_template = GetPersonFromDescription(description: "Jane Smith is 25 years old");

            prompt  {
                Here is the first person you need to create: {{person1_template}}
                And here is the second person you need to create: {{person2_template}}
            }
        }
        "#;

        let (remaining, definitions) = parser.parse_wail_file(input).unwrap();
        assert!(
            remaining.trim().is_empty(),
            "Parser should consume all input"
        );
        assert_eq!(
            definitions.len(),
            3,
            "Should parse object, template and main"
        );

        // Verify Person object
        match &definitions[0] {
            WAILDefinition::Object(obj) => {
                assert_eq!(obj.name, "Person");
                if let WAILType::Composite(WAILCompositeType::Object(obj)) = &obj.field_type {
                    let fields = obj.type_data.field_definitions.as_ref().unwrap();
                    assert_eq!(fields.len(), 2);
                    assert_eq!(fields[0].name, "name");
                    assert_eq!(fields[1].name, "age");
                } else {
                    panic!("Expected Person to be an object type");
                }
            }
            _ => panic!("First definition should be an Object"),
        }

        // Verify GetPersonFromDescription template
        let _template = match &definitions[1] {
            WAILDefinition::Template(template) => {
                assert_eq!(template.name, "GetPersonFromDescription");
                assert_eq!(template.inputs.len(), 1);
                assert_eq!(template.inputs[0].name, "description");
                assert!(template.prompt_template.contains("{{description}}"));
                assert!(template.prompt_template.contains("{{return_type}}"));
                template
            }
            _ => panic!("Second definition should be a Template"),
        };

        // Verify main block
        match &definitions[2] {
            WAILDefinition::Main(main) => {
                assert_eq!(main.statements.len(), 2);

                // Check first assignment
                let (var1, call1) = main.statements[0].as_assignment().unwrap();
                assert_eq!(var1, "person1_template");
                assert_eq!(call1.template_name, "GetPersonFromDescription");
                assert_eq!(call1.arguments.len(), 1);

                // Check second assignment
                let (var2, call2) = main.statements[1].as_assignment().unwrap();
                assert_eq!(var2, "person2_template");
                assert_eq!(call2.template_name, "GetPersonFromDescription");
                assert_eq!(call2.arguments.len(), 1);

                // Test prompt interpolation
                let registry = parser.template_registry.borrow().clone();
                let interpolated = main.interpolate_prompt(&registry).unwrap();

                println!("Interpolated prompt:\n{}", interpolated);
                assert!(interpolated.contains("Given this description of a person:"));
                assert!(interpolated.contains("Create a Person object with their name and age."));
                assert!(interpolated.contains("Here is the first person you need to create:"));
                assert!(interpolated.contains("And here is the second person you need to create:"));
            }
            _ => panic!("Third definition should be Main"),
        }
    }

    #[test]
    fn test_validation() {
        let parser = WAILParser::new();

        // First parse a template with undefined types
        let input = r#"template ProcessData(
            raw_data: DataInput,
            config: ProcessConfig[]
        ) -> DataOutput {
            prompt: """
            Process the data according to the configuration.
            Input: {{raw_data}}
            Config: {{config}}
            Output format: {{return_type}}
            """
        }"#;

        let (_, _) = parser.parse_template(input).unwrap();

        // Now validate - should get errors for undefined types and warning for no main block
        let (warnings, errors) = parser.validate();

        // Should have errors for DataInput, ProcessConfig, and DataOutput
        assert_eq!(errors.len(), 3);
        let error_types: Vec<_> = errors
            .iter()
            .map(|e| match e {
                ValidationError::UndefinedTypeInTemplate { type_name, .. } => type_name.as_str(),
            })
            .collect();
        assert!(error_types.contains(&"DataInput"));
        assert!(error_types.contains(&"DataOutput"));
        assert!(error_types.contains(&"ProcessConfig"));

        // Should have warning for no main block
        assert!(warnings
            .iter()
            .any(|w| matches!(w, ValidationWarning::NoMainBlock)));

        // Now define one of the types with a similar name to test typo detection
        let type_def = r#"object DataInputs {
            field1: String
            field2: Number
        }"#;
        let (_, _) = parser.parse_object(type_def).unwrap();

        // Validate again - should now get a typo warning for DataInput vs DataInputs
        let (warnings, errors) = parser.validate();
        assert!(warnings.iter().any(|w| matches!(w,
            ValidationWarning::PossibleTypo {
                type_name,
                similar_to,
                ..
            } if type_name == "DataInput" && similar_to == "DataInputs"
        )));
    }
}
