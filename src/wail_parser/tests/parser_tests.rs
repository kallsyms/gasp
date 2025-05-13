// Add test that tries parsing a basic object
use crate::json_parser::Parser as JsonParser;
use crate::json_types::{JsonValue, Number};
use crate::parser_types::*;
use crate::types::*;
use crate::wail_parser::{GuessConfidence, WAILParser, *};

#[test]
fn test_parse_basic_object() {
    let input = r#"object Person {
            name: String
            age: Number
      }"#;

    let test_dir = std::env::current_dir().unwrap();
    let parser = WAILParser::new(test_dir);
    let mut incremental = parser.incremental_parser(input.to_string());

    let object_def = incremental.parse_object().unwrap();

    match object_def {
        WAILDefinition::Object(object) => {
            assert_eq!(
                object
                    .field_type
                    .type_data()
                    .field_definitions
                    .as_ref()
                    .unwrap()
                    .len(),
                3 // 2 for fields above and then implicity _type metadata field
            );
        }
        _ => panic!("Expected object definition"),
    }
}

#[test]
fn test_array_with_annotations() {
    let input = r#"
        object Test {
            items: String[] @description("An array of strings")
            mix: Number[] @description("Array with numbers") #Comment
        }
        "#;

    let test_dir = std::env::current_dir().unwrap();
    let parser = WAILParser::new(test_dir);

    match parser.parse_wail_file(input.to_string(), WAILFileType::Library, true) {
        Ok(definitions) => {
            match &definitions[0] {
                WAILDefinition::Object(field) => {
                    if let WAILType::Composite(WAILCompositeType::Object(obj)) = &field.field_type {
                        let fields = obj.type_data.field_definitions.as_ref().unwrap();

                        // Add debug prints
                        println!("Number of fields: {}", fields.len());
                        for (i, field) in fields.iter().enumerate() {
                            println!(
                                "Field {}: {} annotations: {}",
                                i,
                                field.name,
                                field.annotations.len()
                            );
                            for ann in &field.annotations {
                                println!("  Annotation: {:?}", ann);
                            }
                        }

                        // First field test
                        assert_eq!(fields[0].name, "items");
                        assert!(matches!(
                            fields[0].field_type,
                            WAILType::Composite(WAILCompositeType::Array(_))
                        ));
                        assert_eq!(
                            fields[0].annotations.len(),
                            1,
                            "items field should have 1 annotation"
                        );

                        // Second field test
                        assert_eq!(fields[1].name, "mix");
                        assert!(matches!(
                            fields[1].field_type,
                            WAILType::Composite(WAILCompositeType::Array(_))
                        ));
                        assert_eq!(
                            fields[1].annotations.len(),
                            1,
                            "mix field should have 1 annotation"
                        );
                    } else {
                        panic!("Expected object type");
                    }
                }
                _ => panic!("Expected object definition"),
            }
        }
        Err(e) => panic!("Failed to parse: {:?}", e),
    }
}

#[test]
fn test_parse_template() {
    // First create a parser
    let test_dir = std::env::current_dir().unwrap();
    let parser = WAILParser::new(test_dir);

    // Create and register the DateInfo type
    let date_info_fields = vec![
        WAILField {
            name: "day".to_string(),
            field_type: WAILType::Simple(WAILSimpleType::Number(WAILNumber::Integer(
                WAILInteger {
                    value: 0,
                    type_data: WAILTypeData {
                        json_type: JsonValue::Number(Number::Integer(0)),
                        type_name: "Number".to_string(),
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
                    type_name: "String".to_string(),
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
            type_name: "DateInfo".to_string(),
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

    let mut incremental = parser.incremental_parser(input.to_string());

    let template_def = incremental.parse_template().unwrap();

    match template_def {
        WAILDefinition::Template(template) => {
            assert_eq!(template.name, "ParseDate");
            assert_eq!(template.inputs.len(), 1);
            assert_eq!(template.inputs[0].name, "date_string");
            assert!(template.prompt_template.contains("{{date_string}}"));
            assert!(template.prompt_template.contains("{{return_type}}"));
            assert!(template.output.name == "DateInfo");
        }
        _ => panic!("Expected template definition"),
    }
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

    let test_dir = std::env::current_dir().unwrap();
    let parser = WAILParser::new(test_dir);

    let mut incremental = parser.incremental_parser(input.to_string());
    let template_def = incremental.parse_template().unwrap();

    match template_def {
        WAILDefinition::Template(template) => {
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
        _ => panic!("Expected template definition"),
    }
}

#[test]
fn test_prompt_interpolation() {
    let test_dir = std::env::current_dir().unwrap();
    let parser = WAILParser::new(test_dir);

    // Define DateInfo type
    let fields = vec![
        WAILField {
            name: "day".to_string(),
            field_type: WAILType::Simple(WAILSimpleType::Number(WAILNumber::Integer(
                WAILInteger {
                    value: 0,
                    type_data: WAILTypeData {
                        json_type: JsonValue::Number(Number::Integer(0)),
                        type_name: "Number".to_string(),
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
                    type_name: "String".to_string(),
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
            type_name: "DateInfo".to_string(),
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
                    type_name: "String".to_string(),
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

    let final_prompt = template
        .interpolate_prompt(Some(&inputs), &parser.object_instances.borrow().clone())
        .unwrap();
    println!("Final prompt:\n{}", final_prompt);
}

#[test]
fn test_wail_parsing() {
    let test_dir = std::env::current_dir().unwrap();
    let parser = WAILParser::new(test_dir);

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

    let definitions = parser
        .parse_wail_file(input.to_string(), WAILFileType::Application, true)
        .unwrap();

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
                assert_eq!(fields.len(), 3); // 3 because objects have added _type fields
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
}

#[test]
fn test_circular_imports() {
    use std::fs;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();

    // Create a.lib.wail that imports from b
    fs::write(
        tmp.path().join("a.lib.wail"),
        r#"import { B } from "b.lib.wail"
        object A { field: String }"#,
    )
    .unwrap();

    // Create b.lib.wail that imports from a
    fs::write(
        tmp.path().join("b.lib.wail"),
        r#"import { A } from "a.lib.wail"
        object B { field: String }"#,
    )
    .unwrap();

    let parser = WAILParser::new(tmp.path().to_path_buf());

    // Try to parse a file that starts the circular chain
    let input = r#"import { A } from "a.lib.wail"
    main { prompt { } }"#;

    let result = parser.parse_wail_file(input.to_string(), WAILFileType::Application, true);
    println!("{:?}", result);
    assert!(matches!(result, Err(WAILParseError::CircularImport { .. })));
}

#[test]
fn test_union_types() {
    use crate::wail_parser::{WAILFileType, WAILParser};
    use std::collections::HashMap;

    // ── 1. the WAIL program  ────────────────────────────────────────────
    // (no `main` block any more, so we treat the file as a Library)
    let input = r#"
        object ErrorResult  { error: String  code: Number }
        object SuccessResult{ data : String }

        union ApiResponse = ErrorResult | SuccessResult | String;

        template TestNamedUnionArray(test: String) -> ApiResponse[] {
            prompt: """
            Process this test case: {{test}}
            {{return_type}}
            """
        }

        template TestNamedUnion(test: String) -> ApiResponse {
            prompt: """
            Process this test case: {{test}}
            {{return_type}}
            """
        }

        template TestInlineUnion(test: String) -> ErrorResult | String {
            prompt: """
            Process this inline test: {{test}}
            {{return_type}}
            """
        }
    "#;

    // ── 2. parse  ───────────────────────────────────────────────────────
    let parser = WAILParser::new(std::env::current_dir().unwrap());
    parser
        .parse_wail_file(input.to_string(), WAILFileType::Library, true)
        .expect("WAIL parse failed");

    // ── 3. render any one of the templates without arguments
    //       (we only need its schema block for the assertions)
    let tpl = parser
        .template_registry
        .borrow()
        .get("TestNamedUnion") // names are not namespaced in a single-file test
        .expect("template not found")
        .clone();

    let prompt = tpl
        .interpolate_prompt(None, &HashMap::new())
        .expect("render failed");

    println!("Generated prompt:\n{prompt}");

    // ── 4. same assertions as before  ───────────────────────────────────
    assert!(prompt.contains("Any of these JSON-like formats:"));
    assert!(prompt.contains("Format 1:"));
    assert!(prompt.contains("Format 2:"));
    assert!(prompt.contains("ErrorResult"));
    assert!(prompt.contains("SuccessResult"));
    assert!(prompt.contains("string"));
    assert!(prompt.contains("-- OR --"));

    // ── 5. type-level validation still passes  ──────────────────────────
    let (_warn, errors) = parser.validate();
    assert!(
        errors.is_empty(),
        "Unexpected validation errors: {errors:?}",
    );
}

#[test]
fn test_validation() {
    let test_dir = std::env::current_dir().unwrap();
    let parser = WAILParser::new(test_dir);

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

    // Use incremental parser for template
    let mut incremental = parser.incremental_parser(input.to_string());
    incremental.parse_template().unwrap();

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

    // Now define one of the types with a similar name to test typo detection
    let type_def = r#"object DataInputs {
            field1: String
            field2: Number
      }"#;

    // Use incremental parser for object
    let mut incremental = parser.incremental_parser(type_def.to_string());
    incremental.parse_object().unwrap();

    // Validate again - should now get a typo warning for DataInput vs DataInputs
    let (warnings, _errors) = parser.validate();
    assert!(warnings.iter().any(|w| matches!(w,
          ValidationWarning::PossibleTypo {
             type_name,
             similar_to,
             ..
          } if type_name == "DataInput" && similar_to == "DataInputs"
    )));
}

#[test]
fn test_template_args_interpolation() {
    use std::collections::HashMap;

    use crate::{
        json_types::{JsonValue, Number},
        parser_types::{TemplateArgument, WAILField, WAILTemplateDef},
        wail_parser::{WAILFileType, WAILParser},
    };

    /* ── 1. WAIL program (library only) ─────────────────────────────── */

    let program = r#"
        object Person { name: String  age: Number }

        template Echo(
            str_arg : String,
            num_arg : Number,
            bool_arg: Boolean,
            arr_arg : String[],
            obj_arg : Person,
            null_arg: String
        ) -> String {
            prompt: """
            String arg: {{str_arg}}
            Number arg: {{num_arg}}
            Boolean arg: {{bool_arg}}
            Array arg: {{arr_arg}}
            Object arg: {{obj_arg}}
            Null arg: {{null_arg}}
            """
        }
    "#;

    let parser = WAILParser::new(std::env::current_dir().unwrap());
    parser
        .parse_wail_file(program.to_string(), WAILFileType::Library, true)
        .expect("WAIL parse failed");

    /* ── 2. Build call-site arguments ───────────────────────────────── */

    // obj_arg  → TemplateArgument::Object { name, age }
    let mut obj_inner = HashMap::new();
    obj_inner.insert(
        "name".to_string(),
        TemplateArgument::String("John".to_string()),
    );
    obj_inner.insert("age".to_string(), TemplateArgument::Number(30));

    let mut args: HashMap<String, TemplateArgument> = HashMap::new();
    args.insert("str_arg".into(), TemplateArgument::String("hello".into()));
    args.insert("num_arg".into(), TemplateArgument::Number(42));
    args.insert("bool_arg".into(), TemplateArgument::String("true".into())); // Boolean→string in prompt
    args.insert(
        "arr_arg".into(),
        TemplateArgument::Array(vec![
            TemplateArgument::String("one".into()),
            TemplateArgument::String("two".into()),
        ]),
    );
    args.insert("obj_arg".into(), TemplateArgument::Object(obj_inner));
    args.insert("null_arg".into(), TemplateArgument::String("null".into()));

    /* ── 3. Render the template ─────────────────────────────────────── */

    let tpl = parser
        .template_registry
        .borrow()
        .get("Echo")
        .expect("template not found")
        .clone();

    let rendered = tpl
        .interpolate_prompt(Some(&args), &HashMap::new())
        .expect("render failed");

    println!("Rendered prompt:\n{rendered}");

    /* ── 4. Assertions  ─────────────────────────────────────────────── */

    assert!(rendered.contains("String arg: hello"));
    assert!(rendered.contains("Number arg: 42"));
    assert!(rendered.contains("Boolean arg: true"));
    assert!(
        rendered.contains("Array arg: one, two")
            || rendered.contains("Array arg: [\"one\", \"two\"]")
    ); // depending on join impl
    assert!(
        rendered.contains("Object arg: {name: John, age: 30}")
            || rendered.contains("Object arg: {age: 30, name: John}")
    );
    assert!(rendered.contains("Null arg: null"));
}

#[test]
fn test_json_segment_parsing() {
    let schema_single = r#"
            template Test() -> String { prompt: """Test""" }
            main {
                let result = Test();
                prompt { {{result}} }
            }
        "#;

    let test_dir = std::env::current_dir().unwrap();
    let parser = WAILParser::new(test_dir);
    parser
        .parse_wail_file(schema_single.to_string(), WAILFileType::Application, true)
        .unwrap();

    let good_output = r#"
            Some chatter …
            <action>"hello"</action>
            More chatter …
        "#;
    assert!(parser.parse_llm_output(good_output).is_ok());

    /* -------------------------------------------------
     * 4. Number / array payloads still OK
     * ------------------------------------------------*/
    let number_schema = r#"
            template Test() -> Number { prompt: """Test""" }
            main { let n = Test(); prompt { {{n}} } }
        "#;
    parser
        .parse_wail_file(number_schema.to_string(), WAILFileType::Application, true)
        .unwrap();

    let num_out = r#"<action>43</action>"#;
    assert!(parser.parse_llm_output(num_out).is_ok());

    let array_schema = r#"
            template Test() -> Number[] { prompt: """Test""" }
            main { let arr = Test(); prompt { {{arr}} } }
        "#;
    parser
        .parse_wail_file(array_schema.to_string(), WAILFileType::Application, true)
        .unwrap();

    let arr_out = r#"<action>[1, 2, 3]</action>"#;
    assert!(parser.parse_llm_output(arr_out).is_ok());
}

#[test]
fn test_parse_object_as_argument_to_func() {
    use std::collections::HashMap;

    use crate::{
        json_types::JsonValue,
        parser_types::{TemplateArgument, WAILTemplateDef},
        wail_parser::{WAILFileType, WAILParser},
    };

    /* ── 1. WAIL source – library-only ─────────────────────────────── */

    let prog = r#"
        object Article          { content: String  url: String }
        object ArticleMetadata  {
            authors     : String[]
            headline    : String
            publishDate : String
            categories  : String[]
            keywords    : String[]
            summary     : String
            sentiment   : String
        }
        object ProcessedArticle {
            article  : Article
            metadata : ArticleMetadata
        }

        template ExtractInformation(article: Article) -> ProcessedArticle {
            prompt: """
            You are an AI assistant specialized in analyzing news articles.

            Article URL: {{article.url}}

            Content to analyze:
            {{article.content}}

            Extract:
              • Categories / topics
              • Keywords
              • Summary (2-3 sentences)
              • Sentiment (positive / negative / neutral)

            {{return_type}}
            """
        }
    "#;

    let parser = WAILParser::new(std::env::current_dir().unwrap());
    parser
        .parse_wail_file(prog.into(), WAILFileType::Library, true)
        .expect("WAIL parse failed");

    /* ── 2. Build call-site arguments ──────────────────────────────── */

    // Article object argument
    let mut article_obj = HashMap::new();
    article_obj.insert(
        "content".into(),
        TemplateArgument::String("I am an article".into()),
    );
    article_obj.insert(
        "url".into(),
        TemplateArgument::String("www.example.com".into()),
    );

    let mut args = HashMap::new();
    args.insert("article".into(), TemplateArgument::Object(article_obj));

    /* ── 3. Render the template ────────────────────────────────────── */

    let tpl: WAILTemplateDef = parser
        .template_registry
        .borrow()
        .get("ExtractInformation")
        .expect("template not found")
        .clone();

    let prompt = tpl
        .interpolate_prompt(Some(&args), &HashMap::new())
        .expect("interpolation failed");

    println!("Generated prompt:\n{prompt}");

    /* ── 4. Quick sanity assertions ───────────────────────────────── */

    assert!(prompt.contains("www.example.com"));
    assert!(prompt.contains("I am an article"));
    // make sure return-type instructions appear
    assert!(prompt.contains("Wrap the value"));
}

#[test]
fn test_parse_ideal_wail() {
    let test_dir = std::env::current_dir().unwrap();
    let parser = WAILParser::new(test_dir);
    let input = r#"object RealtimeEvent {
   type: String  # response.create, response.chunk, response.end
   response: {
      modalities: String[]  # ["text", "speech"]
      instructions: String
      voice: String
   }
}

object Message {
   content: String
   role: String  # user or assistant
}

object Conversation {
   messages: Message[]
   model: String
   temperature: Number
   max_tokens: Number
}

template GeneratePrompt(conversation: Conversation) -> RealtimeEvent {
   prompt: """
   You are a helpful AI assistant engaging in a conversation with the user.
   Previous messages:
   {{#each conversation.messages}}
   {{this.role}}: {{this.content}}
   {{/each}}

   Respond naturally and conversationally. Your response will be delivered through both text and speech.
   Use the voice specified in the response object.

   {{return_type}}
   """
}

main {
   template_args {
      messages: Message[],
      model: String,
      temperature: Number,
      max_tokens: Number
   }

   let conversation = Conversation(
      messages: $messages,
      model: $model,
      temperature: $temperature,
      max_tokens: $max_tokens
   );

   let prompt_stmnt = GeneratePrompt(conversation: conversation);

   prompt {
      Create a response that will be delivered through both text and speech:
      {{prompt_stmnt}}
   }
}"#;

    let result = parser.parse_wail_file(input.to_string(), WAILFileType::Application, true);
    assert!(
        result.is_ok(),
        "Failed to parse ideal.wail: {:?}",
        result.err()
    );

    let definitions = result.unwrap();

    // Verify RealtimeEvent object
    let realtime_event = match &definitions[0] {
        WAILDefinition::Object(obj) => obj,
        _ => panic!("First definition should be RealtimeEvent object"),
    };
    assert_eq!(realtime_event.name, "RealtimeEvent");
    if let WAILType::Composite(WAILCompositeType::Object(obj)) = &realtime_event.field_type {
        let fields = obj.type_data.field_definitions.as_ref().unwrap();
        assert_eq!(fields.len(), 3); // type and response fields + _type metadata
        assert_eq!(fields[0].name, "type");
        assert_eq!(fields[1].name, "response");
    } else {
        panic!("RealtimeEvent should be an object type");
    }

    // Verify Message object
    let message = match &definitions[1] {
        WAILDefinition::Object(obj) => obj,
        _ => panic!("Second definition should be Message object"),
    };
    assert_eq!(message.name, "Message");
    if let WAILType::Composite(WAILCompositeType::Object(obj)) = &message.field_type {
        let fields = obj.type_data.field_definitions.as_ref().unwrap();
        assert_eq!(fields.len(), 3); // content and role fields + implicit _type metadata
        assert_eq!(fields[0].name, "content");
        assert_eq!(fields[1].name, "role");
    } else {
        panic!("Message should be an object type");
    }

    // Verify Conversation object
    let conversation = match &definitions[2] {
        WAILDefinition::Object(obj) => obj,
        _ => panic!("Third definition should be Conversation object"),
    };
    assert_eq!(conversation.name, "Conversation");
    if let WAILType::Composite(WAILCompositeType::Object(obj)) = &conversation.field_type {
        let fields = obj.type_data.field_definitions.as_ref().unwrap();
        assert_eq!(fields.len(), 5); // messages, model, temperature, max_tokens + implicit _type field
        assert_eq!(fields[0].name, "messages");
        assert_eq!(fields[1].name, "model");
        assert_eq!(fields[2].name, "temperature");
        assert_eq!(fields[3].name, "max_tokens");
    } else {
        panic!("Conversation should be an object type");
    }

    // Verify GeneratePrompt template
    let template = match &definitions[3] {
        WAILDefinition::Template(template) => template,
        _ => panic!("Fourth definition should be GeneratePrompt template"),
    };
    assert_eq!(template.name, "GeneratePrompt");
    assert_eq!(template.inputs.len(), 1); // conversation parameter
    assert_eq!(template.inputs[0].name, "conversation");
    assert!(template
        .prompt_template
        .contains("{{#each conversation.messages}}"));
    assert!(template
        .prompt_template
        .contains("{{this.role}}: {{this.content}}"));
    assert!(template.prompt_template.contains("{{return_type}}"));
}

#[test]
fn test_parse_imports() {
    let input = r#"
    import { Person, Address, Thing } from "types.lib.wail"
    import { GetPerson } from "templates.lib.wail"

    object Config {
        name: String
    }

    main {
        prompt { }
    }
    "#;

    use std::fs;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();

    // Create a.lib.wail that imports from b
    fs::write(
        tmp.path().join("types.lib.wail"),
        r#"
            object Person {
                name: String
            }
            object Address {
                street: String
            }

            union Thing = Person | Address;
        "#,
    )
    .unwrap();

    fs::write(
        tmp.path().join("templates.lib.wail"),
        r#"
            template GetPerson(name: String) -> String {
                prompt: """
                {{return_type}}
                """
            }"#,
    )
    .unwrap();

    let parser = WAILParser::new(tmp.path().to_path_buf());
    let definitions = parser
        .parse_wail_file(input.to_string(), WAILFileType::Application, true)
        .unwrap();

    // First two definitions should be imports
    println!("{:?}", definitions);
    match &definitions[0] {
        WAILDefinition::Import(import) => {
            assert_eq!(import.items, vec!["Person", "Address", "Thing"]);
            assert_eq!(import.path, "types.lib.wail");
        }
        _ => panic!("Expected first definition to be import"),
    }

    match &definitions[1] {
        WAILDefinition::Import(import) => {
            assert_eq!(import.items, vec!["GetPerson"]);
            assert_eq!(import.path, "templates.lib.wail");
        }
        _ => panic!("Expected second definition to be import"),
    }
}

#[test]
fn test_wail_errors() {
    let test_dir = std::env::current_dir().unwrap();
    let parser = WAILParser::new(test_dir);

    // Test duplicate object definition (first wins)
    let input = r#"
object Person {
    name: String
}
object Person {
    age: Number
}
main {
    prompt { }
}
"#;

    let _result = parser
        .parse_wail_file(input.to_string(), WAILFileType::Application, true)
        .unwrap();

    // Ensure the first definition was used
    let registry = parser.registry.borrow();
    let person = registry.get("Person").expect("Person not found");

    if let WAILType::Composite(WAILCompositeType::Object(obj)) = &person.field_type {
        let fields = obj.type_data.field_definitions.as_ref().unwrap();
        let field_names: Vec<_> = fields.iter().map(|f| f.name.as_str()).collect();
        assert!(field_names.contains(&"name"));
        assert!(!field_names.contains(&"age")); // second definition was ignored
    } else {
        panic!("Person should be an object type");
    }

    drop(registry);
    // Test missing main block
    let input = r#"
      object Person {
            name: String
      }
      "#;

    let err = parser
        .parse_wail_file(input.to_string(), WAILFileType::Application, true)
        .unwrap_err();

    println!("HERER");
    assert!(matches!(err, WAILParseError::MissingMainBlock));

    // Test unexpected token
    let input = r#"
      object Person {
            name: String
            age: @ Number
      }
      main {
            prompt { }
      }
      "#;

    let err = parser
        .parse_wail_file(input.to_string(), WAILFileType::Application, true)
        .unwrap_err();

    assert!(matches!(err, WAILParseError::UnexpectedToken { found, .. } if found == "@"));

    //     // Test EOF
    //     let input = r#"
    //     object Person {
    //         name: String
    // "#;

    //     let err = parser.parse_wail_file(input).unwrap_err();
    //     println!("{:?}", err);
    //     assert!(matches!(err, WAILParseError::UnexpectedEOF { .. }));
}
#[test]
fn test_resolve_imports_comprehensive() {
    use std::fs;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();

    // Create types.lib.wail with objects and a union
    fs::write(
        tmp.path().join("types.lib.wail"),
        r#"
        object Person {
            name: String
            age: Number
        }
        
        object Address {
            street: String
            city: String
            country: String
        }
        
        union ContactInfo = Person | Address;
        "#,
    )
    .unwrap();

    // Create templates.lib.wail with templates that use the types
    fs::write(
        tmp.path().join("templates.lib.wail"),
        r#"
        import { Person, Address, ContactInfo } from "types.lib.wail"
        
        template GetPerson(name: String, age: Number) -> Person {
            prompt: """
            Create a person with name {{name}} and age {{age}}.
            {{return_type}}
            """
        }
        
        template GetContactInfo(info: String) -> ContactInfo {
            prompt: """
            Parse this info: {{info}}
            Return in this format: {{return_type}}
            """
        }
        "#,
    )
    .unwrap();

    // Create nested.lib.wail that imports from templates.lib.wail
    fs::write(
        tmp.path().join("nested.lib.wail"),
        r#"
        import { GetPerson } from "templates.lib.wail"
        
        object ExtendedPerson {
            person: Person
            notes: String
        }
        "#,
    )
    .unwrap();

    // Create main file that imports from all libraries
    let input = r#"
    import { Person, ContactInfo } from "types.lib.wail"
    import { GetContactInfo } from "templates.lib.wail"
    import { ExtendedPerson } from "nested.lib.wail"
    
    main {
        let person_info = GetContactInfo(info: "John Doe, 30 years old");
        
        prompt {
            Parse this information: {{person_info}}
        }
    }
    "#;

    let parser = WAILParser::new(tmp.path().to_path_buf());
    let result = parser.parse_wail_file(input.to_string(), WAILFileType::Application, true);
    assert!(
        result.is_ok(),
        "Failed to parse imports: {:?}",
        result.err()
    );

    // Verify all types were imported correctly
    let registry = parser.registry.borrow();
    let template_registry = parser.template_registry.borrow();

    println!("{:?}", registry.keys());

    // Check objects
    assert!(
        registry.contains_key("types.lib.wail.Person"),
        "Person object not imported"
    );
    assert!(
        registry.contains_key("types.lib.wail.ContactInfo"),
        "ContactInfo union not imported"
    );
    assert!(
        registry.contains_key("nested.lib.wail.ExtendedPerson"),
        "ExtendedPerson not imported"
    );

    // Check that Person has the right fields
    if let WAILType::Composite(WAILCompositeType::Object(obj)) =
        &registry.get("types.lib.wail.Person").unwrap().field_type
    {
        let fields = obj.type_data.field_definitions.as_ref().unwrap();
        assert_eq!(fields.len(), 3); // name, age, _type

        let field_names: Vec<String> = fields.iter().map(|f| f.name.clone()).collect();
        assert!(field_names.contains(&"name".to_string()));
        assert!(field_names.contains(&"age".to_string()));
    } else {
        panic!("Person should be an object type");
    }

    // Check that ContactInfo is a union with Person and Address
    if let WAILType::Composite(WAILCompositeType::Union(union)) = &registry
        .get("types.lib.wail.ContactInfo")
        .unwrap()
        .field_type
    {
        assert_eq!(union.members.len(), 2);

        // Verify the union members (could be in any order)
        let member_types: Vec<String> = union
            .members
            .iter()
            .map(|m| match &m.field_type {
                WAILType::Composite(WAILCompositeType::Object(obj)) => {
                    obj.type_data.type_name.to_string()
                }
                _ => "unknown".to_string(),
            })
            .collect();

        assert!(
            member_types.contains(&"Person".to_string()),
            "Union doesn't contain Person"
        );
        assert!(
            member_types.contains(&"Address".to_string()),
            "Union doesn't contain Address"
        );
    } else {
        panic!("ContactInfo should be a union type");
    }

    // Check that ExtendedPerson references Person
    if let WAILType::Composite(WAILCompositeType::Object(obj)) = &registry
        .get("nested.lib.wail.ExtendedPerson")
        .unwrap()
        .field_type
    {
        let fields = obj.type_data.field_definitions.as_ref().unwrap();
        assert_eq!(fields.len(), 3); // person, notes, _type

        // Check that person field is of type Person
        let person_field = fields
            .iter()
            .find(|f| f.name == "person")
            .expect("No person field found");
        if let WAILType::Composite(WAILCompositeType::Object(pers_obj)) = &person_field.field_type {
            assert_eq!(pers_obj.type_data.type_name, "Person");
        } else {
            panic!("person field should be of type Person");
        }
    } else {
        panic!("ExtendedPerson should be an object type");
    }

    // Check templates
    assert!(
        template_registry.contains_key("templates.lib.wail.GetContactInfo"),
        "GetContactInfo template not imported"
    );

    assert!(
        !template_registry.contains_key("GetPerson"),
        "GetPerson template should not be imported directly"
    );

    // Check that GetContactInfo returns ContactInfo
    let template = template_registry
        .get("templates.lib.wail.GetContactInfo")
        .unwrap();

    println!("{:?}", template.output.field_type);
    if let WAILType::Composite(WAILCompositeType::Union(_)) = &template.output.field_type {
        // Good, it's a union
    } else {
        panic!("GetContactInfo should return a union type");
    }

    // Check import chain was handled correctly
    let import_chain = parser.import_chain.borrow();
    assert!(
        import_chain.stack.is_empty(),
        "Import stack should be empty after parsing"
    );
}
