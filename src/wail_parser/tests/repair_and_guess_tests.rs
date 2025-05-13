use crate::json_types::{JsonValue, Number};
use crate::wail_parser::{GuessConfidence, WAILCompositeType, WAILFileType, WAILParser, WAILType};

use serde_json::json; // ← NEW

/* -------------- helpers -------------------------------------------------- */
fn sv(v: serde_json::Value) -> JsonValue {
    // NEW ─ serde → our JsonValue
    match v {
        serde_json::Value::Null => JsonValue::Null,
        serde_json::Value::Bool(b) => JsonValue::Boolean(b),
        serde_json::Value::Number(n) if n.is_i64() => {
            JsonValue::Number(Number::Integer(n.as_i64().unwrap()))
        }
        serde_json::Value::Number(n) => JsonValue::Number(Number::Float(n.as_f64().unwrap())),
        serde_json::Value::String(s) => JsonValue::String(s),
        serde_json::Value::Array(a) => JsonValue::Array(a.into_iter().map(sv).collect()),
        serde_json::Value::Object(o) => {
            JsonValue::Object(o.into_iter().map(|(k, v)| (k, sv(v))).collect())
        }
    }
}

fn typename(t: &WAILType) -> String {
    match t {
        WAILType::Composite(WAILCompositeType::Object(o)) => o.type_data.type_name.clone(),
        _ => "<non-object>".into(),
    }
}

/// Person & Address in registry
fn parser_pa() -> WAILParser {
    let p = WAILParser::new(std::env::current_dir().unwrap());
    p.incremental_parser(r#"object Person  { name:String  age:Number }"#.into())
        .parse_object()
        .unwrap();
    p.incremental_parser(r#"object Address { street:String city:String }"#.into())
        .parse_object()
        .unwrap();
    p
}

/// Person, Address, Company (shares `name` with Person)
fn parser_pac() -> WAILParser {
    let p = parser_pa();
    p.incremental_parser(r#"object Company { name:String industry:String }"#.into())
        .parse_object()
        .unwrap();
    p
}

/* -------------- tests ---------------------------------------------------- */

#[test]
fn confidence_none_when_structure_unknown() {
    let parser = parser_pa();
    let val = sv(json!({ "foo": 1, "bar": 2 }));

    assert!(parser.guess_type(&val).is_none());
}

#[test]
fn confidence_low_on_ambiguous_subset() {
    // key `name` appears on Person **and** Company  → ambiguous  → Low
    let parser = parser_pac();
    let val = sv(json!({ "name": "Acme"}));
    let res = parser.guess_type(&val).unwrap();
    assert_eq!(res.confidence, GuessConfidence::Low);
}

#[test]
fn confidence_medium_on_unique_subset_with_fuzzy_key() {
    // key `cty` (edit-distance 1 from `city`) appears only on Address → Medium
    let parser = parser_pa();
    let val = sv(json!({ "cty": "NY" }));
    let res = parser.guess_type(&val).unwrap();
    assert_eq!(res.confidence, GuessConfidence::Medium);
    assert_eq!(typename(&res.ty), "Address");
}

#[test]
fn confidence_high_when_all_required_fields_present() {
    // All Person fields present (plus extra) → High
    let parser = parser_pa();
    let val = sv(json!({ "name": "Jane", "age": 28, "nickname": "JJ" }));
    let res = parser.guess_type(&val).unwrap();
    assert_eq!(res.confidence, GuessConfidence::High);
    assert_eq!(typename(&res.ty), "Person");
}

#[test]
fn confidence_exact_on_exact_type_tag() {
    let parser = parser_pa();
    let val = sv(json!({ "_type": "Person" }));
    let res = parser.guess_type(&val).unwrap();
    assert_eq!(res.confidence, GuessConfidence::Exact);
    assert_eq!(typename(&res.ty), "Person");
}

#[test]
fn confidence_exact_on_unique_fuzzy_type_tag() {
    // “Persn” is Levenshtein-1 from Person and unique in registry → Exact
    let parser = parser_pa();
    let val = sv(json!({ "_type": "Persn" }));
    let res = parser.guess_type(&val).unwrap();
    assert_eq!(res.confidence, GuessConfidence::Exact);
    assert_eq!(typename(&res.ty), "Person");
}

// ------ NEW TESTS FOR “expected-type aware” guessing -----------------------

/// helper: pull the *declared* return type of the first template in the registry
fn first_template_output(parser: &WAILParser) -> WAILType {
    let reg = parser.template_registry.borrow();
    reg.values().next().unwrap().output.field_type.clone()
}

#[test]
fn confidence_high_for_union_element_when_expected_known() {
    // schema: Person | Number | String
    let schema = r#"
        object Person { name:String }
        union Mixed = Person | Number | String;
        template T() -> Mixed { prompt:"" }
    "#;

    let p = WAILParser::new(std::env::current_dir().unwrap());
    p.parse_wail_file(schema.into(), WAILFileType::Library, true)
        .unwrap();

    let expected = first_template_output(&p); // Mixed
    let val = sv(json!({"name":"Alice"}));

    let res = p.guess_against_expected(&val, &expected).unwrap();
    assert_eq!(res.confidence, GuessConfidence::High);
    assert_eq!(typename(&res.ty), "Person");
}

#[test]
fn confidence_high_for_array_of_union_when_expected_known() {
    // schema: [ Person | Number | String ]
    let schema = r#"
        object Person { name:String }
        union Mixed = Person | Number | String;
        template T() -> Mixed[] { prompt:"" }
    "#;

    let p = WAILParser::new(std::env::current_dir().unwrap());
    p.parse_wail_file(schema.into(), WAILFileType::Library, true)
        .unwrap();

    let expected = first_template_output(&p); // Mixed[]
    let val = sv(json!([ {"name":"Bob"}, 7, "hi" ]));

    let res = p.guess_against_expected(&val, &expected).unwrap();
    assert_eq!(res.confidence, GuessConfidence::High);

    // top-level should be an Array whose element-type is that Mixed union
    match res.ty {
        WAILType::Composite(WAILCompositeType::Array(arr)) => {
            let elem = arr.type_data.element_type.unwrap();
            assert!(matches!(
                *elem,
                WAILType::Composite(WAILCompositeType::Union(_))
            ));
        }
        _ => panic!("expected Array<Union>"),
    }
}
// ------ NEW TESTS THAT EXERCISE “FIX → GUESS” END-TO-END -------------------
#[test]
fn repair_then_guess_on_object_missing_type_tag() {
    use crate::{
        json_types::JsonValue,
        wail_parser::{WAILFileType, WAILParser},
    };
    use serde_json::json;

    /* ── 1. compile the schema ─────────────────────────────────────── */

    let src = r#"
        object Person { name:String age:Number }

        template Make() -> Person {
            prompt:"""
            {{return_type}}
            """
        }
    "#;

    let parser = WAILParser::new(std::env::current_dir().unwrap());
    parser
        .parse_wail_file(src.into(), WAILFileType::Library, true)
        .expect("schema parse failed");

    /* ── 2. grab the declared return-type of `Make` ────────────────── */

    let tpl = parser
        .template_registry
        .borrow()
        .get("Make")
        .expect("template not in registry");
    let expected_ty = &tpl.output.field_type;

    /* ── 3. raw LLM output (missing the “_type” tag) ───────────────── */

    let raw_json = sv(json!({ "name": "John", "age": 30 }));

    /* ── 4. fix & guess in one go ──────────────────────────────────── */

    let (fixed, guess) = parser
        .fix_and_guess(raw_json, expected_ty)
        .expect("fix_and_guess failed");

    /* ── 5. assertions ─────────────────────────────────────────────── */

    assert_eq!(guess.confidence, crate::wail_parser::GuessConfidence::Exact);
    assert_eq!(guess.ty.type_name(), "Person");

    // confirm the auto-inserted tag
    match &fixed {
        JsonValue::Object(map) => {
            assert_eq!(
                map.get("_type")
                    .and_then(|v| v.as_string())
                    .expect("no _type tag"),
                "Person"
            );
        }
        _ => panic!("expected object"),
    }
}

#[test]
fn repair_then_guess_on_array_of_union() {
    let schema = r#"
        object Person  { name:String }
        object Address { street:String }
        union Mixed = Person | Address;
        template Make() -> Mixed[] { 
            prompt: """
            """ 
        }

        main {
            let res = Make();
            prompt { {{res}} }
        }
    "#;

    let p = WAILParser::new(std::env::current_dir().unwrap());
    p.parse_wail_file(schema.into(), WAILFileType::Application, true)
        .unwrap();
    let expected = first_template_output(&p); // Mixed[]

    // Payload: two objects, both missing _type.
    let mut val = sv(json!([
        { "name": "Alice" },
        { "street": "Main" }
    ]));

    let (fixed, guess) = p.fix_and_guess(val.clone(), &expected).unwrap();
    // assert_eq!(guess.confidence, GuessConfidence::High);

    // each element should now be tagged
    assert_eq!(fixed[0]["_type"].as_string().unwrap(), "Person");
    assert_eq!(fixed[1]["_type"].as_string().unwrap(), "Address");
}
