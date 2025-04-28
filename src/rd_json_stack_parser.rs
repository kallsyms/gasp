use crate::json_tok::{Kind, Tok, Tokenizer};
use crate::json_types::{JsonError, JsonValue, Number};
use crate::stream_json_parser::{Path, StreamEvent};
use std::collections::HashMap;

fn unescape(src: &str) -> Result<String, JsonError> {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some('/') => out.push('/'),
            Some('b') => out.push('\u{0008}'),
            Some('f') => out.push('\u{000C}'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                if hex.len() != 4 {
                    return Err(JsonError::InvalidEscape);
                }
                let cp = u16::from_str_radix(&hex, 16).map_err(|_| JsonError::InvalidEscape)?;
                out.push(char::from_u32(cp as u32).ok_or(JsonError::InvalidEscape)?);
            }
            _ => return Err(JsonError::InvalidEscape),
        }
    }
    Ok(out)
}

/*==============================================================*/
/*  parser façade                                               */
/*==============================================================*/

pub struct Parser {
    buf: String,
    cur: usize, // cursor byte offset we last consumed
    lex: Tokenizer,
    look: Tok,
    streaming: bool,

    // stack for depth-1 event detection
    depth: usize,
}

impl Parser {
    pub fn new(input: Vec<u8>) -> Self {
        Self::with_mode(input, false)
    }
    pub fn new_stream(input: Vec<u8>) -> Self {
        Self::with_mode(input, true)
    }

    fn ensure_look(&mut self) -> Result<(), JsonError> {
        if self.look.kind == Kind::Eof {
            self.look = self.lex.next_tok(&self.buf)?;
        }
        Ok(())
    }

    fn with_mode(input: Vec<u8>, streaming: bool) -> Self {
        Self {
            buf: String::from_utf8(input).unwrap(),
            cur: 0,
            lex: Tokenizer::new(),
            look: Tok {
                kind: Kind::Eof,
                start: 0,
                end: 0,
            }, // placeholder
            streaming,
            depth: 0,
        }
    }

    pub fn push(&mut self, chunk: &[u8]) {
        self.buf.push_str(std::str::from_utf8(chunk).unwrap());
        self.lex.reset(self.cur); // point lexer at new slice
                                  // leave `look` untouched – it still refers to the old peek token
    }

    pub fn advance(&mut self) -> Result<StreamEvent, JsonError> {
        match self.parse_step()? {
            Step::NeedMore => Ok(StreamEvent::NeedMore),
            Step::YieldComplete(v) => Ok(StreamEvent::Complete(v)),
            Step::YieldPath(p, v) => Ok(StreamEvent::Partial(p, v)),
        }
    }

    pub fn parse(mut self) -> Result<JsonValue, JsonError> {
        loop {
            match self.parse_step()? {
                Step::YieldComplete(v) => return Ok(v),
                Step::NeedMore => return Err(JsonError::UnexpectedEof),
                _ => continue,
            }
        }
    }

    /*==========================================================*/
    /*  single driver step                                      */
    /*==========================================================*/
    fn parse_step(&mut self) -> Result<Step, JsonError> {
        self.ensure_look()?;
        // ─────────── root level ────────────
        if self.depth == 0 {
            // nothing to read yet
            if self.look.kind == Kind::Eof {
                return Ok(Step::NeedMore);
            }

            // first root value
            let first = self.value()?;

            // if we’re at EOF now, that was the *only* root value
            if self.look.kind == Kind::Eof {
                return Ok(Step::YieldComplete(first));
            }

            // otherwise collect additional top-level values → implicit array
            let mut items = vec![first];
            loop {
                match self.look.kind {
                    Kind::Comma => {
                        self.bump();
                    } // skip separator
                    Kind::Eof => break,                     // finished
                    Kind::RBracket | Kind::RBrace => break, // safety
                    _ => (),                                // next value immediately
                }
                if self.look.kind == Kind::Eof {
                    break;
                }
                items.push(self.value()?);
            }
            return Ok(Step::YieldComplete(JsonValue::Array(items)));
        }

        // ─────────── inside array/object ────────────
        // attach_value() will emit Partial/Complete when a child finishes;
        // for the driver that just means “wait”.
        Ok(Step::NeedMore)
    }
    /*==========================================================*/
    /*  token helpers                                           */
    /*==========================================================*/
    fn bump(&mut self) -> Tok {
        let t = self.look;
        self.cur = t.end;
        self.refill_look().unwrap();
        t
    }
    fn expect(&mut self, k: Kind) -> Result<(), JsonError> {
        if self.look.kind == k {
            self.bump();
            Ok(())
        } else {
            Err(JsonError::ExpectedComma)
        }
    }
    fn refill_look(&mut self) -> Result<(), JsonError> {
        self.look = self.lex.next_tok(&self.buf)?;
        Ok(())
    }

    fn return_root(&self, v: JsonValue) -> Step {
        if self.streaming && self.depth == 1 {
            match &v {
                JsonValue::Object(map) => {
                    if let Some((k, _)) = map.iter().next() {
                        return Step::YieldPath(Path::RootField(k.clone()), v);
                    }
                }
                JsonValue::Array(arr) if !arr.is_empty() => {
                    return Step::YieldPath(Path::ArrayElem(0), v);
                }
                _ => {}
            }
        }
        Step::NeedMore
    }

    /*==========================================================*/
    /*  recursive-descent value parsing                         */
    /*==========================================================*/
    fn value(&mut self) -> Result<JsonValue, JsonError> {
        match self.look.kind {
            Kind::LBrace => self.object(),
            Kind::LBracket => self.array(),
            Kind::Str => {
                let t = self.bump();
                let raw = &self.buf[t.start..t.end];
                return Ok(JsonValue::String(unescape(raw)?));
            }
            Kind::Num => {
                let t = self.bump();
                let slice = &self.buf[t.start..t.end];
                let val = if slice.contains('.') || slice.contains('e') || slice.contains('E') {
                    let f = slice
                        .parse::<f64>()
                        .map_err(|_| JsonError::InvalidNumber("float".into()))?;
                    JsonValue::Number(Number::Float(f))
                } else {
                    let i = slice
                        .parse::<i64>()
                        .map_err(|_| JsonError::InvalidNumber("int".into()))?;
                    JsonValue::Number(Number::Integer(i))
                };
                Ok(val)
            }

            Kind::True => {
                self.bump();
                Ok(JsonValue::Boolean(true))
            }
            Kind::False => {
                self.bump();
                Ok(JsonValue::Boolean(false))
            }
            Kind::Null => {
                self.bump();
                Ok(JsonValue::Null)
            }
            Kind::Ident => {
                let mut s = String::new();
                loop {
                    let t = self.bump();
                    s.push_str(&self.buf[t.start..t.end]);

                    // capture whitespace between identifiers
                    let ws = &self.buf[self.cur..self.look.start];
                    if self.look.kind == Kind::Ident {
                        s.push_str(ws);
                        continue; // grab next ident
                    }
                    break;
                }
                return Ok(JsonValue::String(s));
            }

            k => Err(JsonError::UnexpectedToken(format!("token {:?}", k))),
        }
    }

    fn object(&mut self) -> Result<JsonValue, JsonError> {
        self.depth += 1;
        self.bump(); // consume '{'
        if self.look.kind == Kind::Eof {
            return Err(JsonError::InvalidKey);
        }
        let mut map = HashMap::new();
        while self.look.kind != Kind::RBrace {
            let key = match self.look.kind {
                Kind::Str | Kind::Ident => {
                    let t = self.bump();
                    let s = &self.buf[t.start..t.end];
                    if matches!(self.look.kind, Kind::Colon)
                        && matches!(t.kind, Kind::Ident)
                        && (s == "true" || s == "false" || s == "null")
                    {
                        return Err(JsonError::ReservedKeyword(s.into()));
                    }
                    s.to_owned()
                }
                Kind::True => return Err(JsonError::ReservedKeyword("true".into())),
                Kind::False => return Err(JsonError::ReservedKeyword("false".into())),
                Kind::Null => return Err(JsonError::ReservedKeyword("null".into())),
                Kind::Comma => return Err(JsonError::UnexpectedChar(',')),
                _ => return Err(JsonError::InvalidKey),
            };
            self.expect(Kind::Colon)?;
            let val = self.value()?;
            if map.insert(key.clone(), val.clone()).is_some() {
                return Err(JsonError::DuplicateKey(key));
            }
            match self.look.kind {
                Kind::Comma => {
                    self.bump();
                }
                Kind::RBrace => (),
                Kind::Eof if self.streaming => {
                    return Err(JsonError::StreamingSnapshot(JsonValue::Object(map)))
                }
                _ => return Err(JsonError::ExpectedComma),
            }
        }
        self.bump(); // consume '}'
        self.depth -= 1;

        let obj = JsonValue::Object(map);
        if let Step::YieldPath(_, _) = self.return_root(obj.clone()) {
            return Ok(obj); // caller will handle the Step already queued
        };
        Ok(obj)
    }

    fn array(&mut self) -> Result<JsonValue, JsonError> {
        self.depth += 1;
        self.bump(); // consume '['
        let mut vec = Vec::new();
        while self.look.kind != Kind::RBracket {
            if self.look.kind == Kind::Comma {
                return Err(JsonError::UnexpectedChar(','));
            }
            vec.push(self.value()?);
            match self.look.kind {
                Kind::Comma => {
                    self.bump();
                }
                Kind::RBracket => (),
                Kind::Eof if self.streaming => {
                    return Err(JsonError::StreamingSnapshot(JsonValue::Array(vec)))
                }
                _ => return Err(JsonError::ExpectedComma),
            }
        }
        self.bump(); // consume ']'
        self.depth -= 1;

        let array = JsonValue::Array(vec);
        if let Step::YieldPath(_, _) = self.return_root(array.clone()) {
            return Ok(array); // caller will handle the Step already queued
        };
        Ok(array)
    }
}

/*==============================================================*/
/*  event enum returned by Parser::parse_step                   */
/*==============================================================*/

#[derive(Debug)]
enum Step {
    NeedMore,
    YieldPath(Path, JsonValue),
    YieldComplete(JsonValue),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_implicit_arrays() {
        // Test comma-separated
        let input = r#"{"message": 123},{"code": 404}"#.as_bytes().to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Array(arr)) => {
                assert_eq!(arr.len(), 2);
                match &arr[0] {
                    JsonValue::Object(obj) => assert_eq!(
                        obj.get("message").unwrap(),
                        &JsonValue::Number(Number::Integer(123))
                    ),
                    _ => panic!("Expected first element to be object"),
                }
                match &arr[1] {
                    JsonValue::Object(obj) => assert_eq!(
                        obj.get("code").unwrap(),
                        &JsonValue::Number(Number::Integer(404))
                    ),
                    _ => panic!("Expected second element to be object"),
                }
            }
            _ => panic!("Expected array"),
        }

        // Test space-separated
        let input = r#"{"message": 123} {"code": 404}"#.as_bytes().to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Array(arr)) => {
                assert_eq!(arr.len(), 2);
                match &arr[0] {
                    JsonValue::Object(obj) => assert_eq!(
                        obj.get("message").unwrap(),
                        &JsonValue::Number(Number::Integer(123))
                    ),
                    _ => panic!("Expected first element to be object"),
                }
                match &arr[1] {
                    JsonValue::Object(obj) => assert_eq!(
                        obj.get("code").unwrap(),
                        &JsonValue::Number(Number::Integer(404))
                    ),
                    _ => panic!("Expected second element to be object"),
                }
            }
            _ => panic!("Expected array"),
        }

        // Test newline-separated
        let input = r#"{"message": 123}
    {"code": 404}"#
            .as_bytes()
            .to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Array(arr)) => {
                assert_eq!(arr.len(), 2);
                match &arr[0] {
                    JsonValue::Object(obj) => assert_eq!(
                        obj.get("message").unwrap(),
                        &JsonValue::Number(Number::Integer(123))
                    ),
                    _ => panic!("Expected first element to be object"),
                }
                match &arr[1] {
                    JsonValue::Object(obj) => assert_eq!(
                        obj.get("code").unwrap(),
                        &JsonValue::Number(Number::Integer(404))
                    ),
                    _ => panic!("Expected second element to be object"),
                }
            }
            _ => panic!("Expected array"),
        }

        // Test no separation
        let input = r#"{"message": 123}{"code": 404}"#.as_bytes().to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Array(arr)) => {
                assert_eq!(arr.len(), 2);
                match &arr[0] {
                    JsonValue::Object(obj) => assert_eq!(
                        obj.get("message").unwrap(),
                        &JsonValue::Number(Number::Integer(123))
                    ),
                    _ => panic!("Expected first element to be object"),
                }
                match &arr[1] {
                    JsonValue::Object(obj) => assert_eq!(
                        obj.get("code").unwrap(),
                        &JsonValue::Number(Number::Integer(404))
                    ),
                    _ => panic!("Expected second element to be object"),
                }
            }
            _ => panic!("Expected array"),
        }
    }

    #[test]
    fn test_simple_string() {
        let input = r#""hello world""#.as_bytes().to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::String(s)) => assert_eq!(s, "hello world"),
            _ => panic!("Expected string value"),
        }
    }

    #[test]
    fn test_string_escapes() {
        let input = r#""hello\nworld\t\"quote\"""#.as_bytes().to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::String(s)) => assert_eq!(s, "hello\nworld\t\"quote\""),
            _ => panic!("Expected string value"),
        }
    }

    #[test]
    fn test_simple_number() {
        let input = "42".as_bytes().to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Number(Number::Integer(n))) => assert_eq!(n, 42),
            _ => panic!("Expected integer value"),
        }
    }

    #[test]
    fn test_float_number() {
        let input = "42.5".as_bytes().to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Number(Number::Float(n))) => assert_eq!(n, 42.5),
            _ => panic!("Expected float value"),
        }
    }

    #[test]
    fn test_simple_object() {
        let input = r#"{"key": "value"}"#.as_bytes().to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Object(map)) => {
                assert_eq!(map.len(), 1);
                match map.get("key") {
                    Some(JsonValue::String(s)) => assert_eq!(s, "value"),
                    _ => panic!("Expected string value"),
                }
            }
            _ => panic!("Expected object"),
        }
    }

    #[test]
    fn test_simple_array() {
        let input = r#"[1, 2, 3]"#.as_bytes().to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Array(arr)) => {
                assert_eq!(arr.len(), 3);
                match &arr[0] {
                    JsonValue::Number(Number::Integer(n)) => assert_eq!(*n, 1),
                    _ => panic!("Expected integer"),
                }
            }
            _ => panic!("Expected array"),
        }
    }

    #[test]
    fn test_nested_structure() {
        let input = r#"
       {
           "name": "test",
           "numbers": [1, 2, 3],
           "object": {
               "nested": true,
               "null_value": null
           }
       }"#
        .as_bytes()
        .to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Object(map)) => {
                assert_eq!(map.len(), 3);
                match map.get("name") {
                    Some(JsonValue::String(s)) => assert_eq!(s, "test"),
                    _ => panic!("Expected string for name"),
                }
                match map.get("numbers") {
                    Some(JsonValue::Array(arr)) => assert_eq!(arr.len(), 3),
                    _ => panic!("Expected array for numbers"),
                }
                match map.get("object") {
                    Some(JsonValue::Object(obj)) => {
                        assert_eq!(obj.len(), 2);
                        assert!(matches!(obj.get("nested"), Some(JsonValue::Boolean(true))));
                        assert!(matches!(obj.get("null_value"), Some(JsonValue::Null)));
                    }
                    _ => panic!("Expected nested object"),
                }
            }
            _ => panic!("Expected object"),
        }
    }

    #[test]
    fn test_error_cases() {
        let cases = vec![
            ("{", JsonError::InvalidKey),
            (
                r#"{"key": true, "key": false}"#,
                JsonError::DuplicateKey("key".to_string()),
            ),
            ("@invalid", JsonError::UnexpectedChar('@')),
            ("{,}", JsonError::UnexpectedChar(',')),
            ("[,]", JsonError::UnexpectedChar(',')),
            ("{true:1}", JsonError::ReservedKeyword("true".to_string())),
        ];

        for (input, expected_err) in cases {
            let mut parser = Parser::new(input.as_bytes().to_vec());
            match parser.parse() {
                Err(e) => assert_eq!(e, expected_err),
                Ok(_) => panic!("Expected error for input: {}", input),
            }
        }
    }

    #[test]
    fn test_unquoted_keys() {
        let input = r#"{
            name: "John",
            age: 30,
            city: "New York"
        }"#
        .as_bytes()
        .to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Object(map)) => {
                assert_eq!(map.len(), 3);
                assert_eq!(map.get("name").unwrap().as_string().unwrap(), "John");
                match map.get("age").unwrap() {
                    JsonValue::Number(Number::Integer(n)) => assert_eq!(*n, 30),
                    _ => panic!("Expected integer for age"),
                }
            }
            _ => panic!("Expected object"),
        }
    }

    #[test]
    fn test_single_quotes() {
        let input = r#"{
            'name': 'John',
            'nested': {'key': 'value'}
        }"#
        .as_bytes()
        .to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Object(map)) => {
                assert_eq!(map.len(), 2);
                assert_eq!(map.get("name").unwrap().as_string().unwrap(), "John");
                match map.get("nested").unwrap() {
                    JsonValue::Object(nested) => {
                        assert_eq!(nested.get("key").unwrap().as_string().unwrap(), "value");
                    }
                    _ => panic!("Expected nested object"),
                }
            }
            _ => panic!("Expected object"),
        }
    }

    #[test]
    fn test_trailing_commas() {
        let input = r#"{
            "array": [1, 2, 3,],
            "object": {
                "key": "value",
            },
        }"#
        .as_bytes()
        .to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Object(map)) => {
                assert_eq!(map.len(), 2);
                match map.get("array").unwrap() {
                    JsonValue::Array(arr) => assert_eq!(arr.len(), 3),
                    _ => panic!("Expected array"),
                }
                match map.get("object").unwrap() {
                    JsonValue::Object(obj) => assert_eq!(obj.len(), 1),
                    _ => panic!("Expected nested object"),
                }
            }
            _ => panic!("Expected object"),
        }
    }

    #[test]
    fn test_unquoted_strings() {
        let input = r#"{
            "name": John,
            "status": active,
            "type": user
        }"#
        .as_bytes()
        .to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Object(map)) => {
                assert_eq!(map.len(), 3);
                assert_eq!(map.get("name").unwrap().as_string().unwrap(), "John");
                assert_eq!(map.get("status").unwrap().as_string().unwrap(), "active");
                assert_eq!(map.get("type").unwrap().as_string().unwrap(), "user");
            }
            _ => panic!("Expected object"),
        }
    }

    #[test]
    fn test_malformed_array() {
        let input = r#"{"message": 123},
            {"code": "404", "details": "error"}"#
            .as_bytes()
            .to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Array(arr)) => {
                assert_eq!(arr.len(), 2);
                match &arr[0] {
                    JsonValue::Object(obj) => {
                        assert!(obj.contains_key("message"));
                    }
                    _ => panic!("Expected first element to be object"),
                }
                match &arr[1] {
                    JsonValue::Object(obj) => {
                        assert!(obj.contains_key("code"));
                        assert!(obj.contains_key("details"));
                    }
                    _ => panic!("Expected second element to be object"),
                }
            }
            _ => panic!("Expected array"),
        }
    }

    #[test]
    fn test_mixed_recovery() {
        let input = r#"{
            name: 'John',
            age: 30,
            hobbies: [coding, gaming, reading,],
            address: {
                city: New York,
                country: USA,
            },
        }"#
        .as_bytes()
        .to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Object(map)) => {
                assert_eq!(map.len(), 4);
                assert_eq!(map.get("name").unwrap().as_string().unwrap(), "John");
                match map.get("age").unwrap() {
                    JsonValue::Number(Number::Integer(n)) => assert_eq!(*n, 30),
                    _ => panic!("Expected integer for age"),
                }
                match map.get("hobbies").unwrap() {
                    JsonValue::Array(arr) => {
                        assert_eq!(arr.len(), 3);
                        assert_eq!(arr[0].as_string().unwrap(), "coding");
                        assert_eq!(arr[1].as_string().unwrap(), "gaming");
                        assert_eq!(arr[2].as_string().unwrap(), "reading");
                    }
                    _ => panic!("Expected array for hobbies"),
                }
                match map.get("address").unwrap() {
                    JsonValue::Object(addr) => {
                        assert_eq!(addr.len(), 2);
                        assert_eq!(addr.get("city").unwrap().as_string().unwrap(), "New York");
                        assert_eq!(addr.get("country").unwrap().as_string().unwrap(), "USA");
                    }
                    _ => panic!("Expected object for address"),
                }
            }
            _ => panic!("Expected object"),
        }
    }
}
