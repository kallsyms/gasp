use crate::parser_types::WAILField;
use std::collections::HashMap;
use std::hash::Hash;

use crate::json_types::{JsonValue, Number};

static STRING_TYPE: &str = "String";
static NUMBER_TYPE: &str = "Number";
static OBJECT_TYPE: &str = "Object";
static TOOL_TYPE: &str = "Tool";
static ARRAY_TYPE: &str = "Array";

#[derive(Debug, Clone)]
pub enum WAILValue {
    String(String),
    Number(i64),
    Float(f64),
    TypeRef(String), // For when we reference a type like "String" or "Number"
}

#[derive(Debug, Clone)]
pub enum WAILSimpleType<'a> {
    String(WAILString<'a>),
    Number(WAILNumber<'a>),
}

#[derive(Debug, Clone)]
pub enum WAILCompositeType<'a> {
    Tool(WAILTool<'a>),
    Object(WAILObject<'a>),
    Array(WAILArray<'a>),
}

#[derive(Debug, Clone)]
pub struct WAILArray<'a> {
    pub values: Vec<WAILType<'a>>,
    pub type_data: WAILTypeData<'a>,
}

#[derive(Debug, Clone)]
pub enum WAILType<'a> {
    Simple(WAILSimpleType<'a>),
    Composite(WAILCompositeType<'a>),
    Value(WAILValue), // For literal values
}

impl<'a> WAILType<'a> {
    pub fn to_schema(&self) -> String {
        match self {
            WAILType::Simple(simple) => match simple {
                WAILSimpleType::String(_) => "string".to_string(),
                WAILSimpleType::Number(_) => "number".to_string(),
            },
            WAILType::Composite(composite) => match composite {
                WAILCompositeType::Tool(_) => "object".to_string(),
                WAILCompositeType::Object(obj) => {
                    let mut schema = String::from("\n{\n");
                    if let Some(fields) = &obj.type_data.field_definitions {
                        for field in fields {
                            schema.push_str(&format!(
                                "  {}: {}\n",
                                field.name,
                                field.field_type.to_schema()
                            ));
                        }
                    }
                    schema.push('}');
                    schema
                }
                WAILCompositeType::Array(arr) => {
                    if let Some(first) = arr.values.first() {
                        format!("array<{}>", first.to_schema())
                    } else {
                        "array".to_string()
                    }
                }
            },
            WAILType::Value(value) => match value {
                WAILValue::String(s) => format!("\"{}\"", s),
                WAILValue::Number(n) => n.to_string(),
                WAILValue::Float(f) => f.to_string(),
                WAILValue::TypeRef(t) => t.clone(),
            },
        }
    }

    pub fn is_object_ref(&self) -> bool {
        match self {
            WAILType::Composite(WAILCompositeType::Object(_)) => true,
            _ => false,
        }
    }

    pub fn type_name(&self) -> &'a str {
        return self.type_data().type_name;
    }

    pub fn field_definitions(&self) -> Option<Vec<WAILField<'a>>> {
        return self.type_data().field_definitions.clone();
    }

    pub fn element_type(&self) -> Option<&'a str> {
        return self.type_data().element_type;
    }

    pub fn type_data(&self) -> &WAILTypeData<'a> {
        match self {
            WAILType::Simple(simple) => match simple {
                WAILSimpleType::String(s) => &s.type_data,
                WAILSimpleType::Number(n) => match n {
                    WAILNumber::Integer(i) => &i.type_data,
                    WAILNumber::Float(f) => &f.type_data,
                },
            },
            WAILType::Composite(composite) => match composite {
                WAILCompositeType::Tool(t) => &t.type_data,
                WAILCompositeType::Object(o) => &o.type_data,
                WAILCompositeType::Array(a) => &a.type_data,
            },
            WAILType::Value(_) => unreachable!(),
        }
    }

    pub fn validate_json(&self, json: &JsonValue) -> Result<(), String> {
        match (self, json) {
            // Simple types
            (WAILType::Simple(WAILSimpleType::String(_)), JsonValue::String(_)) => Ok(()),
            (WAILType::Simple(WAILSimpleType::Number(_)), JsonValue::Number(_)) => Ok(()),

            // Object type
            (WAILType::Composite(WAILCompositeType::Object(obj)), JsonValue::Object(map)) => {
                // Get field definitions
                let fields = obj
                    .type_data
                    .field_definitions
                    .as_ref()
                    .ok_or("Object type missing field definitions")?;

                // Check all required fields are present with correct types
                for field in fields {
                    match map.get(&field.name) {
                        Some(value) => field.field_type.validate_json(value)?,
                        None => return Err(format!("Missing required field: {}", field.name)),
                    }
                }
                Ok(())
            }

            // Array type
            (WAILType::Composite(WAILCompositeType::Array(arr)), JsonValue::Array(values)) => {
                // If we have an element type, validate each element
                if let Some(first) = arr.values.first() {
                    for value in values {
                        first.validate_json(value)?;
                    }
                }
                Ok(())
            }

            // Type mismatch
            _ => Err(format!(
                "Type mismatch: expected {}, got {}",
                self.type_name(),
                match json {
                    JsonValue::String(_) => "String",
                    JsonValue::Number(_) => "Number",
                    JsonValue::Object(_) => "Object",
                    JsonValue::Array(_) => "Array",
                    JsonValue::Boolean(_) => "Boolean",
                    JsonValue::Null => "Null",
                }
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WAILTool<'a> {
    pub name: WAILString<'a>,
    pub parameters: HashMap<WAILString<'a>, WAILType<'a>>,
    pub type_data: WAILTypeData<'a>,
}

#[derive(Debug, Clone)]
pub struct WAILTypeData<'a> {
    pub json_type: JsonValue,
    pub type_name: &'a str,
    pub field_definitions: Option<Vec<WAILField<'a>>>,
    pub element_type: Option<&'a str>, // Just "String", "Number", etc.
}

#[derive(Debug, Clone)]
pub struct WAILInteger<'a> {
    pub value: u64,
    pub type_data: WAILTypeData<'a>,
}

#[derive(Debug, Clone)]
pub struct WAILFloat<'a> {
    pub value: f64,
    pub type_data: WAILTypeData<'a>,
}

#[derive(Debug, Clone)]
pub enum WAILNumber<'a> {
    Integer(WAILInteger<'a>),
    Float(WAILFloat<'a>),
}

#[derive(Debug, Clone)]
pub struct WAILObject<'a> {
    pub value: HashMap<WAILString<'a>, WAILType<'a>>,
    pub type_data: WAILTypeData<'a>,
}

#[derive(Debug, Clone)]
pub struct WAILString<'a> {
    pub value: String,
    pub type_data: WAILTypeData<'a>,
}

impl<'a> Hash for WAILString<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state)
    }
}

impl<'a> PartialEq for WAILString<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<'a> Eq for WAILString<'a> {}

use std::convert::{TryFrom, TryInto};

// First for WAILString since it's used by other types
impl<'a> TryFrom<JsonValue> for WAILString<'a> {
    type Error = String;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        match value.clone() {
            JsonValue::String(s) => Ok(WAILString {
                value: s,
                type_data: WAILTypeData {
                    json_type: value,
                    type_name: STRING_TYPE,
                    field_definitions: None,
                    element_type: None,
                },
            }),
            _ => Err("Expected String JsonValue".to_string()),
        }
    }
}

// For WAILNumber
impl<'a> TryFrom<JsonValue> for WAILNumber<'a> {
    type Error = String;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        match value.clone() {
            JsonValue::Number(n) => match n {
                Number::Integer(i) => Ok(WAILNumber::Integer(WAILInteger {
                    value: i as u64,
                    type_data: WAILTypeData {
                        json_type: value,
                        type_name: NUMBER_TYPE,
                        field_definitions: None,
                        element_type: None,
                    },
                })),
                Number::Float(f) => Ok(WAILNumber::Float(WAILFloat {
                    value: f,
                    type_data: WAILTypeData {
                        json_type: value,
                        type_name: NUMBER_TYPE,
                        field_definitions: None,
                        element_type: None,
                    },
                })),
            },
            _ => Err("Expected Number JsonValue".to_string()),
        }
    }
}

// For WAILSimpleType
impl<'a> TryFrom<JsonValue> for WAILSimpleType<'a> {
    type Error = String;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        match value {
            JsonValue::String(_) => Ok(WAILSimpleType::String(value.try_into()?)),
            JsonValue::Number(_) => Ok(WAILSimpleType::Number(value.try_into()?)),
            _ => Err("Expected Simple Type JsonValue".to_string()),
        }
    }
}

// For WAILObject
impl<'a> TryFrom<JsonValue> for WAILObject<'a> {
    type Error = String;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        match value.clone() {
            JsonValue::Object(map) => {
                let mut wail_map = HashMap::new();
                for (k, v) in map.clone() {
                    let wail_key = WAILString::try_from(JsonValue::String(k))?;
                    let wail_value = WAILType::try_from(v)?;
                    wail_map.insert(wail_key, wail_value);
                }
                Ok(WAILObject {
                    value: wail_map,
                    type_data: WAILTypeData {
                        json_type: value,
                        type_name: OBJECT_TYPE,
                        field_definitions: Some(
                            map.clone()
                                .iter()
                                .map(|(k, v)| WAILField {
                                    name: k.to_string(),
                                    field_type: WAILType::try_from(v.clone()).unwrap(),
                                    annotations: Vec::new(),
                                })
                                .collect(),
                        ),
                        element_type: None,
                    },
                })
            }
            _ => Err("Expected Object JsonValue".to_string()),
        }
    }
}

// For WAILTool
impl<'a> TryFrom<JsonValue> for WAILTool<'a> {
    type Error = String;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        match value.clone() {
            JsonValue::Object(mut map) => {
                let name = map.remove("name").ok_or("Missing name field")?.try_into()?;

                let parameters = map.remove("parameters").ok_or("Missing parameters field")?;

                match parameters {
                    JsonValue::Object(param_map) => {
                        let mut wail_params = HashMap::new();
                        for (k, v) in param_map {
                            let wail_key = WAILString::try_from(JsonValue::String(k))?;
                            let wail_value = WAILType::try_from(v)?;
                            wail_params.insert(wail_key, wail_value);
                        }
                        Ok(WAILTool {
                            name,
                            parameters: wail_params,
                            type_data: WAILTypeData {
                                json_type: value.clone(),
                                type_name: TOOL_TYPE,
                                field_definitions: None,
                                element_type: None,
                            },
                        })
                    }
                    _ => Err("Parameters must be an object".to_string()),
                }
            }
            _ => Err("Expected Object JsonValue for Tool".to_string()),
        }
    }
}

// For WAILType
impl<'a> TryFrom<JsonValue> for WAILType<'a> {
    type Error = String;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        match value {
            JsonValue::String(_) | JsonValue::Number(_) => Ok(WAILType::Simple(value.try_into()?)),
            JsonValue::Object(ref map) => {
                if map.contains_key("name") && map.contains_key("parameters") {
                    Ok(WAILType::Composite(WAILCompositeType::Tool(
                        value.try_into()?,
                    )))
                } else {
                    Ok(WAILType::Composite(WAILCompositeType::Object(
                        value.try_into()?,
                    )))
                }
            }
            _ => Err("Unsupported JsonValue type".to_string()),
        }
    }
}

// And for converting back to JsonValue
impl<'a> From<WAILType<'a>> for JsonValue {
    fn from(wail_type: WAILType<'a>) -> JsonValue {
        match wail_type {
            WAILType::Simple(simple) => match simple {
                WAILSimpleType::String(s) => JsonValue::String(s.value),
                WAILSimpleType::Number(n) => match n {
                    WAILNumber::Integer(i) => JsonValue::Number(Number::Integer(i.value as i64)),
                    WAILNumber::Float(f) => JsonValue::Number(Number::Float(f.value)),
                },
            },
            WAILType::Composite(composite) => match composite {
                WAILCompositeType::Tool(f) => {
                    let mut map = HashMap::new();
                    map.insert("name".to_string(), JsonValue::String(f.name.value));
                    let params_map: HashMap<String, JsonValue> = f
                        .parameters
                        .into_iter()
                        .map(|(k, v)| (k.value, v.into()))
                        .collect();
                    map.insert("parameters".to_string(), JsonValue::Object(params_map));
                    JsonValue::Object(map)
                }
                WAILCompositeType::Object(o) => {
                    let map: HashMap<String, JsonValue> = o
                        .value
                        .into_iter()
                        .map(|(k, v)| (k.value, v.into()))
                        .collect();
                    JsonValue::Object(map)
                }
                WAILCompositeType::Array(array) => {
                    JsonValue::Array(array.values.into_iter().map(|v| v.into()).collect())
                }
            },
            WAILType::Value(value) => match value {
                WAILValue::String(s) => JsonValue::String(s),
                WAILValue::Number(n) => JsonValue::Number(Number::Integer(n)),
                WAILValue::Float(f) => JsonValue::Number(Number::Float(f)),
                WAILValue::TypeRef(t) => JsonValue::String(t),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wail_string() {
        let json = JsonValue::String("hello".to_string());
        let wail: WAILString = json.clone().try_into().unwrap();
        assert_eq!(wail.value, "hello");
        let back: JsonValue = WAILType::Simple(WAILSimpleType::String(wail)).into();
        assert!(matches!(back, JsonValue::String(s) if s == "hello"));
    }

    #[test]
    fn test_wail_number_integer() {
        let json = JsonValue::Number(Number::Integer(42));
        let wail: WAILNumber = json.clone().try_into().unwrap();
        assert!(matches!(wail, WAILNumber::Integer(ref n) if n.value == 42));
        let back: JsonValue = WAILType::Simple(WAILSimpleType::Number(wail)).into();
        assert!(matches!(back, JsonValue::Number(Number::Integer(42))));
    }

    #[test]
    fn test_wail_number_float() {
        let json = JsonValue::Number(Number::Float(3.14));
        let wail: WAILNumber = json.clone().try_into().unwrap();
        assert!(matches!(wail, WAILNumber::Float(ref f) if f.value == 3.14));
        let back: JsonValue = WAILType::Simple(WAILSimpleType::Number(wail)).into();
        assert!(matches!(back, JsonValue::Number(Number::Float(f)) if f == 3.14));
    }

    #[test]
    fn test_wail_object() {
        let mut map = HashMap::new();
        map.insert("key".to_string(), JsonValue::String("value".to_string()));
        let json = JsonValue::Object(map);

        let wail: WAILObject = json.clone().try_into().unwrap();
        assert_eq!(wail.value.len(), 1);

        let back: JsonValue = WAILType::Composite(WAILCompositeType::Object(wail)).into();
        assert!(matches!(back, JsonValue::Object(m) if m.len() == 1));
    }

    #[test]
    fn test_wail_function() {
        let mut params = HashMap::new();
        params.insert(
            "param1".to_string(),
            JsonValue::String("value1".to_string()),
        );

        let mut func_map = HashMap::new();
        func_map.insert("name".to_string(), JsonValue::String("myFunc".to_string()));
        func_map.insert("parameters".to_string(), JsonValue::Object(params));

        let json = JsonValue::Object(func_map);

        let wail: WAILTool = json.clone().try_into().unwrap();
        assert_eq!(wail.name.value, "myFunc");
        assert_eq!(wail.parameters.len(), 1);

        let back: JsonValue = WAILType::Composite(WAILCompositeType::Tool(wail)).into();
        assert!(matches!(back, JsonValue::Object(m) if m.len() == 2));
    }

    #[test]
    fn test_invalid_conversions() {
        // Test string expected, got number
        let result: Result<WAILString, _> = JsonValue::Number(Number::Integer(42)).try_into();
        assert!(result.is_err());

        // Test number expected, got string
        let result: Result<WAILNumber, _> =
            JsonValue::String("not a number".to_string()).try_into();
        assert!(result.is_err());

        // Test function missing required fields
        let empty_map = HashMap::new();
        let result: Result<WAILTool, _> = JsonValue::Object(empty_map).try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_array_type_json_conversion() {
        let string_array = WAILType::Composite(WAILCompositeType::Array(WAILArray {
            type_data: WAILTypeData {
                json_type: JsonValue::Array(vec![]),
                type_name: ARRAY_TYPE,
                field_definitions: None,
                element_type: Some(STRING_TYPE),
            },
            values: vec![
                WAILType::Simple(WAILSimpleType::String(WAILString {
                    value: "hello".to_string(),
                    type_data: WAILTypeData {
                        json_type: JsonValue::String("hello".to_string()),
                        type_name: STRING_TYPE,
                        field_definitions: None,
                        element_type: None,
                    },
                })),
                WAILType::Simple(WAILSimpleType::String(WAILString {
                    value: "world".to_string(),
                    type_data: WAILTypeData {
                        json_type: JsonValue::String("world".to_string()),
                        type_name: STRING_TYPE,
                        field_definitions: None,
                        element_type: None,
                    },
                })),
            ],
        }));

        let json = JsonValue::from(string_array);
        assert!(matches!(json, JsonValue::Array(ref values) if values.len() == 2));

        if let JsonValue::Array(values) = json {
            assert!(matches!(&values[0], JsonValue::String(s) if s == "hello"));
            assert!(matches!(&values[1], JsonValue::String(s) if s == "world"));
        }
    }

    #[test]
    fn test_json_validation() {
        // Create a Person type
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

        let person_type = WAILType::Composite(WAILCompositeType::Object(WAILObject {
            value: HashMap::new(),
            type_data: WAILTypeData {
                json_type: JsonValue::Object(HashMap::new()),
                type_name: "Person",
                field_definitions: Some(person_fields),
                element_type: None,
            },
        }));

        // Valid person
        let mut valid_person = HashMap::new();
        valid_person.insert("name".to_string(), JsonValue::String("John".to_string()));
        valid_person.insert("age".to_string(), JsonValue::Number(Number::Integer(30)));
        assert!(person_type
            .validate_json(&JsonValue::Object(valid_person))
            .is_ok());

        // Invalid person - missing age
        let mut invalid_person = HashMap::new();
        invalid_person.insert("name".to_string(), JsonValue::String("John".to_string()));
        assert!(person_type
            .validate_json(&JsonValue::Object(invalid_person))
            .is_err());

        // Invalid person - wrong type for age
        let mut invalid_person = HashMap::new();
        invalid_person.insert("name".to_string(), JsonValue::String("John".to_string()));
        invalid_person.insert("age".to_string(), JsonValue::String("30".to_string()));
        assert!(person_type
            .validate_json(&JsonValue::Object(invalid_person))
            .is_err());
    }
}
