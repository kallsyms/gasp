use crate::json_types::{JsonValue, Number};
use crate::types::*;
use regex;
use std::collections::HashMap;
use std::env::var;
use std::fmt::Display;
use std::marker::PhantomData;

#[derive(Debug, Clone, PartialEq)]
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

impl Display for WAILAnnotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WAILAnnotation::Description(desc) => write!(f, "Description: {}", desc),
            WAILAnnotation::Example(ex) => write!(f, "Example: {}", ex),
            WAILAnnotation::Validation(rule) => write!(f, "Validation: {}", rule),
            WAILAnnotation::Format(fmt) => write!(f, "Format: {}", fmt),
            WAILAnnotation::Important(note) => write!(f, "Important: {}", note),
            WAILAnnotation::Context(ctx) => write!(f, "Context: {}", ctx),
            WAILAnnotation::Default(def) => write!(f, "Default: {}", def),
            WAILAnnotation::Field { name, annotations } => {
                write!(f, "Field: {}\n", name)?;
                for annotation in annotations {
                    write!(f, "  {}\n", annotation)?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TemplateArgument {
    String(String),
    Number(i64),
    Float(f64),
    TypeRef(String), // For when we reference a type like "String" or "Number"
    TemplateArgRef(String),
    ObjectRef(String),
    Array(Vec<TemplateArgument>),
    Object(HashMap<String, TemplateArgument>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct WAILTemplateCall {
    pub template_name: String,
    pub arguments: HashMap<String, TemplateArgument>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WAILField {
    pub name: String,
    pub field_type: WAILType,
    pub annotations: Vec<WAILAnnotation>,
}

#[derive(Debug, Clone)]
pub struct WAILObjectDef {
    pub name: String,
    pub fields: Vec<WAILField>,
}

#[derive(Debug, Clone)]
pub struct WAILObjectInstantiation {
    pub binding_name: String,
    pub object_type: String,
    pub fields: HashMap<String, TemplateArgument>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct WAILTemplateDef {
    pub name: String,
    pub inputs: Vec<WAILField>,
    pub output: WAILField,
    pub prompt_template: String,
    pub annotations: Vec<WAILAnnotation>,
}

impl TemplateArgument {
    pub fn to_string(&self) -> String {
        match self {
            TemplateArgument::String(s) => s.clone(),
            TemplateArgument::Number(n) => n.to_string(),
            TemplateArgument::Float(f) => f.to_string(),
            TemplateArgument::TypeRef(t) => t.clone(),
            TemplateArgument::TemplateArgRef(t) => format!("${}", t),
            TemplateArgument::ObjectRef(o) => o.clone(),
            TemplateArgument::Object(o) => format!(
                "{{{}}}",
                o.iter()
                    .map(|(k, v)| format!("{k}: {}", v.to_string()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            TemplateArgument::Array(items) => format!(
                "[{}]",
                items
                    .iter()
                    .map(|it| it.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

// Helper function to get nested JSON values using dot notation
fn get_nested_value<'a>(json: &'a HashMap<String, JsonValue>, path: &str) -> Option<&'a JsonValue> {
    let parts: Vec<&str> = path
        .split('.')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if parts.is_empty() {
        return None;
    }

    let mut current = json.get(parts[0]);

    // Traverse the remaining path components
    for part in parts.iter().skip(1) {
        current = match current {
            Some(JsonValue::Object(obj)) => obj.get(*part),
            Some(JsonValue::Array(arr)) if part.parse::<usize>().is_ok() => {
                // Support array indexing if the part is a valid index
                arr.get(part.parse::<usize>().unwrap())
            }
            _ => return None,
        };
    }

    current
}

// ─── Variable / #each renderer ───────────────────────────────────────────────
use crate::template_parser::{parse_template, TemplateSegment};

fn render_segments(
    segs: &[TemplateSegment],
    out: &mut String,
    data: &HashMap<String, JsonValue>,
) -> Result<(), String> {
    for seg in segs {
        match seg {
            TemplateSegment::Text(t) => out.push_str(t),

            TemplateSegment::Variable(path) => {
                if let Some(v) = get_nested_value(data, path) {
                    out.push_str(&match v {
                        JsonValue::String(s) => s.clone(),
                        JsonValue::Number(n) => n.to_string(),
                        JsonValue::Boolean(b) => b.to_string(),
                        JsonValue::Null => "null".into(),
                        _ => v.to_string(),
                    });
                } else {
                    return Err(format!("variable not found: {path}"));
                }
            }

            TemplateSegment::EachLoop { path, body } => {
                if let Some(JsonValue::Array(items)) = get_nested_value(data, path) {
                    for (i, item) in items.iter().enumerate() {
                        // local scope: "." + object fields
                        let mut local = HashMap::<String, JsonValue>::new();
                        local.insert(".".into(), item.clone());
                        if let JsonValue::Object(obj) = item {
                            for (k, v) in obj {
                                local.insert(k.clone(), v.clone());
                            }
                        }
                        render_segments(body, out, &local)?;
                        if i < items.len() - 1 {
                            out.push('\n');
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn count_leading_whitespace(s: &str) -> usize {
    s.chars().take_while(|c| c.is_whitespace()).count()
}

impl<'a> WAILTemplateDef {
    pub fn interpolate_prompt(
        &self,
        arguments: Option<&HashMap<String, TemplateArgument>>,
        object_instances: &HashMap<String, WAILObjectInstantiation>,
    ) -> Result<String, String> {
        let mut prompt = self.prompt_template.clone();

        // Handle input parameters

        for input in &self.inputs {
            let placeholder = format!("{{{{{}}}}}", input.name);

            if !prompt.contains(&placeholder) && !object_instances.contains_key(&input.name) {
                return Err(format!("Missing placeholder for input: {}", input.name));
            }

            if let Some(arguments) = arguments {
                let argument = arguments.get(&input.name).unwrap();

                match argument {
                    TemplateArgument::String(s) => {
                        prompt = prompt.replace(&placeholder, s);
                    }
                    TemplateArgument::Number(n) => {
                        prompt = prompt.replace(&placeholder, &n.to_string());
                    }
                    TemplateArgument::Float(f) => {
                        prompt = prompt.replace(&placeholder, &f.to_string());
                    }
                    TemplateArgument::TypeRef(t) => {
                        prompt = prompt.replace(&placeholder, t);
                    }
                    TemplateArgument::TemplateArgRef(t) => {
                        prompt = prompt.replace(&placeholder, &format!("${}", t));
                    }
                    TemplateArgument::ObjectRef(o) => {
                        let object = object_instances
                            .get(o)
                            .ok_or_else(|| format!("Object not found: {}", o))?;

                        // Replace the object reference placeholder with the object's fields
                        for (name, field) in &object.fields {
                            let field_placeholder = format!("{{{{{}.{}}}}}", o, name);

                            match field {
                                TemplateArgument::String(s) => {
                                    prompt = prompt.replace(&field_placeholder, s);
                                }
                                TemplateArgument::Number(n) => {
                                    prompt = prompt.replace(&field_placeholder, &n.to_string());
                                }
                                TemplateArgument::Float(f) => {
                                    prompt = prompt.replace(&field_placeholder, &f.to_string());
                                }
                                TemplateArgument::TypeRef(t) => {
                                    prompt = prompt.replace(&field_placeholder, t);
                                }
                                TemplateArgument::TemplateArgRef(t) => {
                                    prompt = prompt.replace(&field_placeholder, &format!("${}", t));
                                }
                                TemplateArgument::Array(arr) => {
                                    prompt = prompt.replace(
                                        &placeholder,
                                        &arr.iter()
                                            .map(|it| it.to_string())
                                            .collect::<Vec<_>>()
                                            .join(", "),
                                    );
                                }
                                TemplateArgument::ObjectRef(_) => {
                                    return Err(
                                        "Nested object references are not supported".to_string()
                                    );
                                }
                                TemplateArgument::Object(inner) => {
                                    let rendered = inner
                                        .iter()
                                        .map(|(k, v)| format!("{}: {}", k, v.to_string()))
                                        .collect::<Vec<_>>()
                                        .join(", ");
                                    prompt = prompt.replace(&field_placeholder, &rendered);
                                }
                            }
                        }
                    }
                    TemplateArgument::Array(arr) => {
                        prompt = prompt.replace(
                            &placeholder,
                            &arr.iter()
                                .map(|it| it.to_string())
                                .collect::<Vec<_>>()
                                .join(", "),
                        );
                    }
                    TemplateArgument::Object(obj) => {
                        // Render the object as `key1: val1, key2: val2`
                        let rendered = obj
                            .iter()
                            .map(|(k, v)| format!("{}: {}", k, v.to_string()))
                            .collect::<Vec<_>>()
                            .join(", ");
                        prompt = prompt.replace(&placeholder, &rendered);
                    }
                }
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

            let tag = self.output.field_type.tag();
            let return_prompt = format!(
                "\nAnswer using this schema:\n{}\nWrap the value in <{}> … </{}>.",
                indented_schema, tag, tag
            );

            prompt = re.replace(&prompt, &return_prompt).to_string();
        }

        fn ta_to_json(
            arg: &TemplateArgument,
            object_instances: &HashMap<String, WAILObjectInstantiation>,
        ) -> JsonValue {
            match arg {
                TemplateArgument::Object(map) => {
                    let mut m = HashMap::new();
                    for (k, v) in map {
                        m.insert(k.clone(), JsonValue::String(v.to_string()));
                    }
                    JsonValue::Object(m)
                }
                TemplateArgument::String(s) => JsonValue::String(s.clone()),
                TemplateArgument::Number(n) => JsonValue::Number(Number::Integer(*n)),
                TemplateArgument::Float(f) => JsonValue::Number(Number::Float(*f)),
                TemplateArgument::TemplateArgRef(n) => JsonValue::String(format!("${n}")),
                TemplateArgument::TypeRef(t) => JsonValue::String(t.clone()),

                TemplateArgument::Array(items) => JsonValue::Array(
                    items
                        .iter()
                        .map(|it| ta_to_json(it, object_instances))
                        .collect(),
                ),

                TemplateArgument::ObjectRef(obj) => {
                    let inst = object_instances
                        .get(obj)
                        .expect(&format!("object '{obj}' not found"));
                    let mut map = HashMap::new();
                    for (fld, ta) in &inst.fields {
                        map.insert(fld.clone(), ta_to_json(ta, object_instances));
                    }
                    JsonValue::Object(map)
                }
            }
        }

        /* ──────────────────────────────────────────────────────────
         * Stage 2 – expand {{var}} and {{#each …}}
         * ────────────────────────────────────────────────────────── */
        {
            let mut ctx: HashMap<String, JsonValue> = HashMap::new();
            if let Some(args) = arguments {
                for (k, v) in args {
                    ctx.insert(k.clone(), ta_to_json(v, object_instances));
                }
            }
            // 2-b) run the mini renderer
            let (_, segs) =
                parse_template(&prompt).map_err(|e| format!("template-parse error: {e}"))?;
            let mut rendered = String::new();
            render_segments(&segs, &mut rendered, &ctx)?;

            prompt = rendered; // overwrite the prompt with fully-rendered output
        }

        Ok(prompt)
    }
}

#[cfg(test)]
mod tests {
    use crate::wail_parser::{WAILFileType, WAILParser};

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
        prompt { 
            {{person}} 
        }
    }"#;

        let test_dir = std::env::current_dir().unwrap();
        let parser = WAILParser::new(test_dir);
        parser
            .parse_wail_file(wail_schema.to_string(), WAILFileType::Application, true)
            .unwrap();

        // Test relaxed JSON parsing features
        let cases = vec![
            // Unquoted keys
            r#"<action>{"person": {name: "Alice", age: 25, interests: ["coding"]}}</action>"#,
            // Single quotes
            r#"<action>{'person': {'name': 'Alice', 'age': 25, 'interests': ['coding']}}</action>"#,
            // Trailing commas
            r#"<action>{"person": {"name": "Alice", "age": 25, "interests": ["coding",],}}</action>"#,
            // Mixed quotes and unquoted identifiers
            r#"<action>{"person": {name: 'Alice', "age": 25, interests: ["coding"]}}</action>"#,
        ];

        for case in cases {
            assert!(
                parser.parse_llm_output(case).is_ok(),
                "Failed to parse: {}",
                case
            );
        }
    }

    use super::*;
    use crate::json_types::JsonValue;
    use std::collections::HashMap;

    // Helper function to create a test JSON object
    fn create_test_json() -> HashMap<String, JsonValue> {
        let mut json = HashMap::new();
        let mut user = HashMap::new();
        user.insert("name".to_string(), JsonValue::String("John".to_string()));
        user.insert("age".to_string(), JsonValue::Number(Number::Integer(30)));

        let mut address = HashMap::new();
        address.insert(
            "street".to_string(),
            JsonValue::String("123 Main St".to_string()),
        );
        address.insert(
            "city".to_string(),
            JsonValue::String("Springfield".to_string()),
        );
        user.insert("address".to_string(), JsonValue::Object(address));

        let hobbies = vec![
            JsonValue::String("reading".to_string()),
            JsonValue::String("gaming".to_string()),
        ];
        user.insert("hobbies".to_string(), JsonValue::Array(hobbies));

        json.insert("user".to_string(), JsonValue::Object(user));
        json
    }

    #[test]
    fn test_get_nested_value_basic() {
        let json = create_test_json();

        // Test basic property access
        let name = get_nested_value(&json, "user.name");
        assert_eq!(
            name.and_then(|v| v.as_string()),
            Some("John".to_string()).as_ref()
        );

        // Test nested object access
        let city = get_nested_value(&json, "user.address.city");
        assert_eq!(
            city.and_then(|v| v.as_string()),
            Some("Springfield".to_string()).as_ref()
        );

        // Test array access
        let hobby = get_nested_value(&json, "user.hobbies.0");
        assert_eq!(
            hobby.and_then(|v| v.as_string()),
            Some("reading".to_string()).as_ref()
        );
    }

    #[test]
    fn test_get_nested_value_error_cases() {
        let json = create_test_json();

        // Test invalid path
        assert_eq!(
            get_nested_value(&json, "invalid.path").and_then(|v| v.as_string()),
            None
        );

        // Test empty path
        assert_eq!(
            get_nested_value(&json, "").and_then(|v| v.as_string()),
            None
        );

        // Test invalid array index
        assert_eq!(
            get_nested_value(&json, "user.hobbies.99").and_then(|v| v.as_string()),
            None
        );

        // Test path to primitive as if it were an object
        assert_eq!(
            get_nested_value(&json, "user.name.invalid").and_then(|v| v.as_string()),
            None
        );
    }

    // src/wail_parser/tests/template_loop_tests.rs

    use crate::{
        json_types::Number,
        parser_types::{
            TemplateArgument, WAILCompositeType, WAILField, WAILObject, WAILObjectInstantiation,
            WAILSimpleType, WAILTemplateDef, WAILType,
        },
    };

    /// helpers ────────────────────────────────────────────────────────────────────
    fn jstr<S: Into<String>>(s: S) -> JsonValue {
        JsonValue::String(s.into())
    }
    fn jint(n: i64) -> JsonValue {
        JsonValue::Number(Number::Integer(n))
    }

    /// #each over an *array of objects*  ─────────────────────────────────────────
    #[test]
    fn test_each_loop_nested_properties() {
        /* 1 ─── build raw JSON context ─────────────────────────────────────── */
        let pets = vec![
            JsonValue::Object(HashMap::from([
                ("name".into(), jstr("Fluffy")),
                ("type".into(), jstr("cat")),
            ])),
            JsonValue::Object(HashMap::from([
                ("name".into(), jstr("Rover")),
                ("type".into(), jstr("dog")),
            ])),
        ];
        let mut ctx = HashMap::<String, JsonValue>::new();
        ctx.insert("pets".into(), JsonValue::Array(pets));

        /* 2 ─── dummy output type (we don’t validate in this test) ─────────── */
        let dummy_out = WAILField {
            name: "Void".into(),
            field_type: WAILType::Simple(WAILSimpleType::String(Default::default())),
            annotations: vec![],
        };

        /* 3 ─── template definition ────────────────────────────────────────── */
        let tpl = WAILTemplateDef {
            name: "PetsTpl".into(),
            inputs: vec![], // no formal inputs
            output: dummy_out,
            prompt_template: "{{#each pets}}Pet: {{name}} is a {{type}}{{/each}}".into(),
            annotations: vec![],
        };

        /* 4 ─── run ─────────────────────────────────────────────────────────── */
        let rendered = tpl
            .interpolate_prompt(
                None, // no positional args
                &HashMap::<String, WAILObjectInstantiation>::new(),
                /* we stash our JSON under a *single* argument so the Stage-2
                renderer can see it */
            )
            .unwrap();

        assert_eq!(rendered, "Pet: Fluffy is a cat\nPet: Rover is a dog");
    }

    /// variable + #each combo ───────────────────────────────────────────────────
    #[test]
    fn test_complex_template() {
        /* 1 ─── JSON context identical to the old main-test helper ─────────── */
        let mut user = HashMap::<String, JsonValue>::new();
        user.insert("name".into(), jstr("John"));
        user.insert("age".into(), jint(30));
        user.insert(
            "address".into(),
            JsonValue::Object(HashMap::from([
                ("city".into(), jstr("Springfield")),
                ("street".into(), jstr("123 Main St")),
            ])),
        );
        user.insert(
            "hobbies".into(),
            JsonValue::Array(vec![jstr("reading"), jstr("gaming")]),
        );

        let mut ctx = HashMap::<String, JsonValue>::new();
        ctx.insert("user".into(), JsonValue::Object(user));

        /* 2 ─── dummy output (ignored) ─────────────────────────────────────── */
        let dummy_out = WAILField {
            name: "Void".into(),
            field_type: WAILType::Simple(WAILSimpleType::String(Default::default())),
            annotations: vec![],
        };

        /* 3 ─── template ───────────────────────────────────────────────────── */
        let tpl_body = "\
User {{user.name}} ({{user.age}}) lives in {{user.address.city}}.
Hobbies:
{{#each user.hobbies}}* {{.}}
{{/each}}";

        let tpl = WAILTemplateDef {
            name: "UserTpl".into(),
            inputs: vec![],
            output: dummy_out,
            prompt_template: tpl_body.into(),
            annotations: vec![],
        };

        /* 4 ─── run ────────────────────────────────────────────────────────── */
        let rendered = tpl
            .interpolate_prompt(None, &HashMap::<String, WAILObjectInstantiation>::new())
            .unwrap();

        let expected = "\
User John (30) lives in Springfield.
Hobbies:
* reading
* gaming
";

        assert_eq!(rendered, expected);
    }
}
