#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
use std::collections::HashMap;
use std::sync::Arc;

use crate::json_types::{JsonError, JsonValue, Number};

// For your character-based stack parser
#[derive(Debug, Clone)]
enum ParserState {
    InObject,
    InArray,
    InString,
    InNumber,
    AfterKey,   // After key, expecting colon
    AfterColon, // After colon, expecting value
    AfterValue, // After value, expecting comma or end
}

#[derive(Debug)]
pub struct Parser {
    parser_state: ParserState,
    container: Arc<Vec<u8>>,
    pos: usize,
    stack: Vec<ParserState>,
}

impl Parser {
    pub fn new(input: Vec<u8>) -> Self {
        Parser {
            parser_state: ParserState::InObject,
            container: Arc::new(input),
            pos: 0,
            stack: Vec::new(),
        }
    }

    pub fn parse(&mut self) -> Result<JsonValue, JsonError> {
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                unsafe {
                    return self.parse_value();
                }
            }
        }
        self.parse_fallback()
    }
}

#[cfg(target_arch = "x86_64")]
impl Parser {
    #[target_feature(enable = "avx2")]
    unsafe fn parse_value(&mut self) -> Result<JsonValue, JsonError> {
        self.skip_whitespace();

        match self.peek_byte()? {
            b'{' => self.parse_object(),
            b'[' => self.parse_array(),
            b'"' => {
                self.pos += 1; // Skip opening quote
                Ok(JsonValue::String(self.parse_string_content()?))
            }
            b't' => self.parse_true(),
            b'f' => self.parse_false(),
            b'n' => self.parse_null(),
            b'0'..=b'9' | b'-' => self.parse_number(),
            c => Err(JsonError::UnexpectedChar(c as char)),
        }
    }

    #[target_feature(enable = "avx2")]
    unsafe fn parse_true(&mut self) -> Result<JsonValue, JsonError> {
        if self.container.len() - self.pos >= 4
            && &self.container[self.pos..self.pos + 4] == b"true"
        {
            self.pos += 4;
            Ok(JsonValue::Boolean(true))
        } else {
            Err(JsonError::UnexpectedChar(self.peek_byte()? as char))
        }
    }

    #[target_feature(enable = "avx2")]
    unsafe fn parse_false(&mut self) -> Result<JsonValue, JsonError> {
        if self.container.len() - self.pos >= 5
            && &self.container[self.pos..self.pos + 5] == b"false"
        {
            self.pos += 5;
            Ok(JsonValue::Boolean(false))
        } else {
            Err(JsonError::UnexpectedChar(self.peek_byte()? as char))
        }
    }

    #[target_feature(enable = "avx2")]
    unsafe fn parse_null(&mut self) -> Result<JsonValue, JsonError> {
        if self.container.len() - self.pos >= 4
            && &self.container[self.pos..self.pos + 4] == b"null"
        {
            self.pos += 4;
            Ok(JsonValue::Null)
        } else {
            Err(JsonError::UnexpectedChar(self.peek_byte()? as char))
        }
    }

    #[target_feature(enable = "avx2")]
    unsafe fn skip_whitespace(&mut self) {
        while self.pos < self.container.len() {
            let remaining = self.container.len() - self.pos;
            if remaining < 32 {
                // Handle remaining bytes normally
                while self.pos < self.container.len()
                    && self.container[self.pos].is_ascii_whitespace()
                {
                    self.pos += 1;
                }
                return;
            }

            let input = _mm256_loadu_si256(self.container[self.pos..].as_ptr() as *const __m256i);

            // Match against space, tab, newline, carriage return
            let whitespace = _mm256_cmpeq_epi8(input, _mm256_set1_epi8(b' ' as i8));
            let tabs = _mm256_cmpeq_epi8(input, _mm256_set1_epi8(b'\t' as i8));
            let newlines = _mm256_cmpeq_epi8(input, _mm256_set1_epi8(b'\n' as i8));
            let crs = _mm256_cmpeq_epi8(input, _mm256_set1_epi8(b'\r' as i8));

            let mask = _mm256_movemask_epi8(_mm256_or_si256(
                _mm256_or_si256(whitespace, tabs),
                _mm256_or_si256(newlines, crs),
            )) as u32;

            // Find first non-whitespace
            let zeros = mask.trailing_zeros();
            self.pos += zeros as usize;

            if zeros < 32 {
                return;
            }
        }
    }

    #[target_feature(enable = "avx2")]
    unsafe fn parse_string_content(&mut self) -> Result<String, JsonError> {
        let mut result = String::new();
        let mut start = self.pos;

        while self.pos < self.container.len() {
            let input = _mm256_loadu_si256(self.container[self.pos..].as_ptr() as *const __m256i);

            let quotes = _mm256_cmpeq_epi8(input, _mm256_set1_epi8(b'"' as i8));
            let escapes = _mm256_cmpeq_epi8(input, _mm256_set1_epi8(b'\\' as i8));

            let mask = _mm256_movemask_epi8(_mm256_or_si256(quotes, escapes)) as u32;

            if mask != 0 {
                let pos = mask.trailing_zeros() as usize;
                match self.container[self.pos + pos] {
                    b'"' => {
                        // Append final chunk
                        result.push_str(
                            std::str::from_utf8(&self.container[start..self.pos + pos])
                                .map_err(|_| JsonError::InvalidString)?,
                        );
                        self.pos += pos + 1;
                        return Ok(result);
                    }
                    b'\\' => {
                        // Append chunk before escape
                        result.push_str(
                            std::str::from_utf8(&self.container[start..self.pos + pos])
                                .map_err(|_| JsonError::InvalidString)?,
                        );
                        self.pos += pos + 1; // Skip backslash

                        // Handle escape sequence
                        match self.peek_byte()? {
                            b'"' | b'\\' | b'/' => {
                                result.push(self.peek_byte()? as char);
                                self.pos += 1;
                            }
                            b'b' => {
                                result.push('\u{0008}');
                                self.pos += 1;
                            }
                            b'f' => {
                                result.push('\u{000C}');
                                self.pos += 1;
                            }
                            b'n' => {
                                result.push('\n');
                                self.pos += 1;
                            }
                            b'r' => {
                                result.push('\r');
                                self.pos += 1;
                            }
                            b't' => {
                                result.push('\t');
                                self.pos += 1;
                            }
                            b'u' => {
                                self.pos += 1; // Skip 'u'
                                let hex =
                                    std::str::from_utf8(&self.container[self.pos..self.pos + 4])
                                        .map_err(|_| JsonError::InvalidEscape)?;
                                let code = u16::from_str_radix(hex, 16)
                                    .map_err(|_| JsonError::InvalidEscape)?;
                                result.push(
                                    char::from_u32(code as u32).ok_or(JsonError::InvalidEscape)?,
                                );
                                self.pos += 4;
                            }
                            _ => return Err(JsonError::InvalidEscape),
                        }
                        start = self.pos;
                    }
                    _ => unreachable!(),
                }
            }
            self.pos += 32;
        }
        Err(JsonError::UnexpectedEof)
    }

    #[target_feature(enable = "avx2")]
    unsafe fn parse_object(&mut self) -> Result<JsonValue, JsonError> {
        self.pos += 1; // Skip opening brace
        self.stack.push(ParserState::InObject);
        let mut map = HashMap::new();

        self.skip_whitespace();

        // Handle empty object
        if self.peek_byte()? == b'}' {
            self.pos += 1;
            self.stack.pop();
            return Ok(JsonValue::Object(map));
        }

        loop {
            // Parse key
            if self.peek_byte()? != b'"' {
                return Err(JsonError::UnexpectedChar(self.peek_byte()? as char));
            }
            self.pos += 1; // Skip opening quote
            let key = self.parse_string_content()?;

            self.skip_whitespace();

            // Expect colon
            if self.peek_byte()? != b':' {
                return Err(JsonError::ExpectedColon);
            }
            self.pos += 1;

            self.skip_whitespace();

            // Parse value
            let value = self.parse_value()?;
            map.insert(key, value);

            self.skip_whitespace();

            match self.peek_byte()? {
                b',' => {
                    self.pos += 1;
                    self.skip_whitespace();
                }
                b'}' => {
                    self.pos += 1;
                    self.stack.pop();
                    return Ok(JsonValue::Object(map));
                }
                _ => return Err(JsonError::ExpectedComma),
            }
        }
    }

    #[target_feature(enable = "avx2")]
    unsafe fn parse_array(&mut self) -> Result<JsonValue, JsonError> {
        self.pos += 1; // Skip opening bracket
        self.stack.push(ParserState::InArray);
        let mut array = Vec::new();

        self.skip_whitespace();

        // Handle empty array
        if self.peek_byte()? == b']' {
            self.pos += 1;
            self.stack.pop();
            return Ok(JsonValue::Array(array));
        }

        loop {
            let value = self.parse_value()?;
            array.push(value);

            self.skip_whitespace();

            match self.peek_byte()? {
                b',' => {
                    self.pos += 1;
                    self.skip_whitespace();
                }
                b']' => {
                    self.pos += 1;
                    self.stack.pop();
                    return Ok(JsonValue::Array(array));
                }
                _ => return Err(JsonError::ExpectedComma),
            }
        }
    }

    #[target_feature(enable = "avx2")]
    unsafe fn parse_number(&mut self) -> Result<JsonValue, JsonError> {
        let start = self.pos;

        // Handle negative sign
        if self.peek_byte()? == b'-' {
            self.pos += 1;
        }

        // Use SIMD to find end of number
        let mut has_decimal = false;
        let mut has_exponent = false;

        while self.pos < self.container.len() {
            let input = _mm256_loadu_si256(self.container[self.pos..].as_ptr() as *const __m256i);

            // Match digits, decimal point, and exponent markers
            let mut digits = _mm256_cmpeq_epi8(input, _mm256_set1_epi8(b'0' as i8));
            for i in 1..10 {
                let digit = _mm256_cmpeq_epi8(input, _mm256_set1_epi8((b'0' + i) as i8));
                digits = _mm256_or_si256(digits, digit);
            }

            let decimal = _mm256_cmpeq_epi8(input, _mm256_set1_epi8(b'.' as i8));
            let exponent = _mm256_cmpeq_epi8(input, _mm256_set1_epi8(b'e' as i8));
            let exp_upper = _mm256_cmpeq_epi8(input, _mm256_set1_epi8(b'E' as i8));

            let mask = _mm256_movemask_epi8(_mm256_or_si256(
                digits,
                _mm256_or_si256(decimal, _mm256_or_si256(exponent, exp_upper)),
            )) as u32;

            if mask == 0 {
                break;
            }

            let pos = mask.trailing_zeros() as usize;
            match self.container[self.pos + pos] {
                b'.' if has_decimal => {
                    return Err(JsonError::InvalidNumber("Multiple decimal points".into()))
                }
                b'.' => has_decimal = true,
                b'e' | b'E' if has_exponent => {
                    return Err(JsonError::InvalidNumber("Multiple exponents".into()))
                }
                b'e' | b'E' => {
                    has_exponent = true;
                    // Handle optional +/- after exponent
                    if self.pos + pos + 1 < self.container.len() {
                        match self.container[self.pos + pos + 1] {
                            b'+' | b'-' => self.pos += 1,
                            _ => {}
                        }
                    }
                }
                _ => {}
            }

            self.pos += pos;
        }

        let num_str = std::str::from_utf8(&self.container[start..self.pos])
            .map_err(|_| JsonError::InvalidNumber("Invalid UTF-8".into()))?;

        if has_decimal || has_exponent {
            let num = num_str
                .parse::<f64>()
                .map_err(|_| JsonError::InvalidNumber("Invalid float".into()))?;
            Ok(JsonValue::Number(Number::Float(num)))
        } else {
            let num = num_str
                .parse::<i64>()
                .map_err(|_| JsonError::InvalidNumber("Invalid integer".into()))?;
            Ok(JsonValue::Number(Number::Integer(num)))
        }
    }
}

impl Parser {
    fn peek_byte(&self) -> Result<u8, JsonError> {
        if self.pos >= self.container.len() {
            Err(JsonError::UnexpectedEof)
        } else {
            Ok(self.container[self.pos])
        }
    }

    // And probably good to have:
    fn peek_byte_offset(&self, offset: usize) -> u8 {
        if self.pos + offset < self.container.len() {
            self.container[self.pos + offset]
        } else {
            0
        }
    }
}

impl Parser {
    fn parse_fallback(&mut self) -> Result<JsonValue, JsonError> {
        self.skip_whitespace_fallback();
        self.parse_value_fallback()
    }

    fn skip_whitespace_fallback(&mut self) {
        while self.pos < self.container.len() && self.container[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn parse_value_fallback(&mut self) -> Result<JsonValue, JsonError> {
        match self.peek_byte()? {
            b'{' => self.parse_object_fallback(),
            b'[' => self.parse_array_fallback(),
            b'"' => self.parse_string_fallback(),
            b't' => self.parse_true_fallback(),
            b'f' => self.parse_false_fallback(),
            b'n' => self.parse_null_fallback(),
            b'0'..=b'9' | b'-' => self.parse_number_fallback(),
            c => Err(JsonError::UnexpectedChar(c as char)),
        }
    }

    fn parse_string_fallback(&mut self) -> Result<JsonValue, JsonError> {
        self.pos += 1; // Skip opening quote
        let content = self.parse_string_content_fallback()?;
        Ok(JsonValue::String(content))
    }

    fn parse_string_content_fallback(&mut self) -> Result<String, JsonError> {
        let mut result = String::new();

        while self.pos < self.container.len() {
            match self.container[self.pos] {
                b'"' => {
                    self.pos += 1;
                    return Ok(result);
                }
                b'\\' => {
                    self.pos += 1;
                    match self.peek_byte()? {
                        b'"' | b'\\' | b'/' => {
                            result.push(self.peek_byte()? as char);
                            self.pos += 1;
                        }
                        b'b' => {
                            result.push('\u{0008}');
                            self.pos += 1;
                        }
                        b'f' => {
                            result.push('\u{000C}');
                            self.pos += 1;
                        }
                        b'n' => {
                            result.push('\n');
                            self.pos += 1;
                        }
                        b'r' => {
                            result.push('\r');
                            self.pos += 1;
                        }
                        b't' => {
                            result.push('\t');
                            self.pos += 1;
                        }
                        b'u' => {
                            self.pos += 1;
                            let hex = std::str::from_utf8(&self.container[self.pos..self.pos + 4])
                                .map_err(|_| JsonError::InvalidEscape)?;
                            let code = u16::from_str_radix(hex, 16)
                                .map_err(|_| JsonError::InvalidEscape)?;
                            result
                                .push(char::from_u32(code as u32).ok_or(JsonError::InvalidEscape)?);
                            self.pos += 4;
                        }
                        _ => return Err(JsonError::InvalidEscape),
                    }
                }
                b => {
                    result.push(b as char);
                    self.pos += 1;
                }
            }
        }
        Err(JsonError::UnexpectedEof)
    }

    fn parse_object_fallback(&mut self) -> Result<JsonValue, JsonError> {
        self.pos += 1; // Skip opening brace
        self.stack.push(ParserState::InObject);
        let mut map = HashMap::new();

        self.skip_whitespace_fallback();

        // Handle empty object
        if self.peek_byte()? == b'}' {
            self.pos += 1;
            self.stack.pop();
            return Ok(JsonValue::Object(map));
        }

        loop {
            // Parse key
            if self.peek_byte()? != b'"' {
                return Err(JsonError::UnexpectedChar(self.peek_byte()? as char));
            }
            self.pos += 1;
            let key = self.parse_string_content_fallback()?;

            if map.contains_key(&key) {
                return Err(JsonError::DuplicateKey(key));
            }

            self.skip_whitespace_fallback();

            // Expect colon
            if self.peek_byte()? != b':' {
                return Err(JsonError::ExpectedColon);
            }
            self.pos += 1;

            self.skip_whitespace_fallback();

            // Parse value
            let value = self.parse_value_fallback()?;
            map.insert(key, value);

            self.skip_whitespace_fallback();

            match self.peek_byte()? {
                b',' => {
                    self.pos += 1;
                    self.skip_whitespace_fallback();
                }
                b'}' => {
                    self.pos += 1;
                    self.stack.pop();
                    return Ok(JsonValue::Object(map));
                }
                _ => return Err(JsonError::ExpectedComma),
            }
        }
    }

    fn parse_array_fallback(&mut self) -> Result<JsonValue, JsonError> {
        self.pos += 1; // Skip opening bracket
        self.stack.push(ParserState::InArray);
        let mut array = Vec::new();

        self.skip_whitespace_fallback();

        // Handle empty array
        if self.peek_byte()? == b']' {
            self.pos += 1;
            self.stack.pop();
            return Ok(JsonValue::Array(array));
        }

        loop {
            let value = self.parse_value_fallback()?;
            array.push(value);

            self.skip_whitespace_fallback();

            match self.peek_byte()? {
                b',' => {
                    self.pos += 1;
                    self.skip_whitespace_fallback();
                }
                b']' => {
                    self.pos += 1;
                    self.stack.pop();
                    return Ok(JsonValue::Array(array));
                }
                _ => return Err(JsonError::ExpectedComma),
            }
        }
    }

    fn parse_number_fallback(&mut self) -> Result<JsonValue, JsonError> {
        let start = self.pos;
        let mut has_decimal = false;
        let mut has_exponent = false;

        // Handle negative sign
        if self.peek_byte()? == b'-' {
            self.pos += 1;
        }

        while self.pos < self.container.len() {
            match self.container[self.pos] {
                b'0'..=b'9' => self.pos += 1,
                b'.' if !has_decimal => {
                    has_decimal = true;
                    self.pos += 1;
                }
                b'e' | b'E' if !has_exponent => {
                    has_exponent = true;
                    self.pos += 1;
                    // Handle optional +/-
                    match self.peek_byte()? {
                        b'+' | b'-' => self.pos += 1,
                        _ => {}
                    }
                }
                b'.' if has_decimal => {
                    return Err(JsonError::InvalidNumber("Multiple decimal points".into()))
                }
                b'e' | b'E' if has_exponent => {
                    return Err(JsonError::InvalidNumber("Multiple exponents".into()))
                }
                _ => break,
            }
        }

        let num_str = std::str::from_utf8(&self.container[start..self.pos])
            .map_err(|_| JsonError::InvalidNumber("Invalid UTF-8".into()))?;

        if has_decimal || has_exponent {
            let num = num_str
                .parse::<f64>()
                .map_err(|_| JsonError::InvalidNumber("Invalid float".into()))?;
            Ok(JsonValue::Number(Number::Float(num)))
        } else {
            let num = num_str
                .parse::<i64>()
                .map_err(|_| JsonError::InvalidNumber("Invalid integer".into()))?;
            Ok(JsonValue::Number(Number::Integer(num)))
        }
    }

    fn parse_true_fallback(&mut self) -> Result<JsonValue, JsonError> {
        if self.container.len() - self.pos >= 4
            && &self.container[self.pos..self.pos + 4] == b"true"
        {
            self.pos += 4;
            Ok(JsonValue::Boolean(true))
        } else {
            Err(JsonError::UnexpectedChar(self.peek_byte()? as char))
        }
    }

    fn parse_false_fallback(&mut self) -> Result<JsonValue, JsonError> {
        if self.container.len() - self.pos >= 5
            && &self.container[self.pos..self.pos + 5] == b"false"
        {
            self.pos += 5;
            Ok(JsonValue::Boolean(false))
        } else {
            Err(JsonError::UnexpectedChar(self.peek_byte()? as char))
        }
    }

    fn parse_null_fallback(&mut self) -> Result<JsonValue, JsonError> {
        if self.container.len() - self.pos >= 4
            && &self.container[self.pos..self.pos + 4] == b"null"
        {
            self.pos += 4;
            Ok(JsonValue::Null)
        } else {
            Err(JsonError::UnexpectedChar(self.peek_byte()? as char))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            ("{", JsonError::UnexpectedEof),
            ("[1,]", JsonError::UnexpectedChar(']')),
            (
                r#"{"key": true, "key": false}"#,
                JsonError::DuplicateKey("key".to_string()),
            ),
            ("invalid", JsonError::UnexpectedChar('i')),
        ];

        for (input, expected_err) in cases {
            let mut parser = Parser::new(input.as_bytes().to_vec());
            match parser.parse() {
                Err(e) => assert_eq!(e, expected_err),
                Ok(_) => panic!("Expected error for input: {}", input),
            }
        }
    }
}

#[cfg(test)]
#[cfg(target_arch = "x86_64")]
mod simd_tests {
    use super::*;

    #[test]
    fn test_simd_string() {
        let input = r#""hello world""#.as_bytes().to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::String(s)) => assert_eq!(s, "hello world"),
            _ => panic!("Expected string value"),
        }
    }

    #[test]
    fn test_simd_string_escapes() {
        let input = r#""hello\nworld\t\"quote\"""#.as_bytes().to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::String(s)) => assert_eq!(s, "hello\nworld\t\"quote\""),
            _ => panic!("Expected string value"),
        }
    }

    #[test]
    fn test_simd_whitespace() {
        let input = "    \n\t  42  \r\n  ".as_bytes().to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Number(Number::Integer(n))) => assert_eq!(n, 42),
            _ => panic!("Expected integer value"),
        }
    }

    #[test]
    fn test_simd_large_array() {
        // Test with array large enough to trigger multiple SIMD operations
        let input = "[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20]"
            .as_bytes()
            .to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Array(arr)) => {
                assert_eq!(arr.len(), 20);
                match &arr[19] {
                    JsonValue::Number(Number::Integer(n)) => assert_eq!(*n, 20),
                    _ => panic!("Expected integer"),
                }
            }
            _ => panic!("Expected array"),
        }
    }

    #[test]
    fn test_simd_large_object() {
        // Test object large enough to need multiple SIMD operations
        let input = r#"{
           "key1": "value1",
           "key2": "value2",
           "key3": "value3",
           "key4": "value4",
           "key5": "value5",
           "key6": "value6",
           "key7": "value7",
           "key8": "value8",
           "key9": "value9",
           "key10": "value10"
       }"#
        .as_bytes()
        .to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Object(map)) => {
                assert_eq!(map.len(), 10);
                assert_eq!(map.get("key10").unwrap().as_string().unwrap(), "value10");
            }
            _ => panic!("Expected object"),
        }
    }

    #[test]
    fn test_simd_long_string() {
        // String longer than SIMD register width
        let long_string = "a".repeat(256);
        let input = format!("\"{}\"", long_string).as_bytes().to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::String(s)) => assert_eq!(s.len(), 256),
            _ => panic!("Expected string value"),
        }
    }

    #[test]
    fn test_simd_nested_arrays() {
        // Test deeply nested structure
        let input = r#"[1,[2,[3,[4,[5]]]]]"#.as_bytes().to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Array(arr)) => {
                assert_eq!(arr.len(), 2);
                let mut current = &arr[1];
                for i in 2..=5 {
                    match current {
                        JsonValue::Array(nested) => {
                            assert_eq!(nested.len(), 2);
                            if i < 5 {
                                current = &nested[1];
                            }
                        }
                        _ => panic!("Expected nested array"),
                    }
                }
            }
            _ => panic!("Expected array"),
        }
    }

    #[test]
    fn test_simd_mixed_whitespace() {
        // Test mixture of different whitespace characters
        let input = " \n\t\r { \n\t\r \"key\" \n\t\r : \n\t\r [1, \n\t\r 2] \n\t\r } \n\t\r "
            .as_bytes()
            .to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Object(map)) => match map.get("key").unwrap() {
                JsonValue::Array(arr) => assert_eq!(arr.len(), 2),
                _ => panic!("Expected array"),
            },
            _ => panic!("Expected object"),
        }
    }

    #[test]
    fn test_simd_large_numbers() {
        // Test parsing of various number formats
        let input = r#"[
           9223372036854775807,  
           -9223372036854775808,
           3.14159265359,
           -2.718281828,
           1e308,
           1E-308
       ]"#
        .as_bytes()
        .to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Array(arr)) => {
                assert_eq!(arr.len(), 6);
                match &arr[0] {
                    JsonValue::Number(Number::Integer(n)) => assert_eq!(*n, i64::MAX),
                    _ => panic!("Expected integer"),
                }
            }
            _ => panic!("Expected array"),
        }
    }

    #[test]
    fn test_simd_buffer_boundary() {
        // Create string that's just slightly longer than SIMD register
        // AVX2 is 256 bits = 32 bytes
        let boundary_string = format!(
            r#"{{ "key": "{}", "value": {} }}"#,
            "a".repeat(30), // Push the key right up to buffer boundary
            "b".repeat(30)  // And the value across it
        );
        let input = boundary_string.as_bytes().to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Object(map)) => {
                assert_eq!(map.len(), 2);
                assert_eq!(map.get("key").unwrap().as_string().unwrap().len(), 30);
            }
            _ => panic!("Expected object"),
        }
    }

    #[test]
    fn test_simd_special_chars() {
        let input = r#"{
           "unicode": "\u0001\u0002\u0003",
           "escapes": "\"\\/\b\f\n\r\t",
           "mixed": "hello\u0020world\u0021",
           "emoji": "🦀"
       }"#
        .as_bytes()
        .to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Object(map)) => {
                assert_eq!(
                    map.get("escapes").unwrap().as_string().unwrap(),
                    "\"\\/\u{0008}\u{000C}\n\r\t"
                );
                assert_eq!(
                    map.get("mixed").unwrap().as_string().unwrap(),
                    "hello world!"
                );
                assert_eq!(map.get("emoji").unwrap().as_string().unwrap(), "🦀");
            }
            _ => panic!("Expected object"),
        }
    }

    #[test]
    fn test_simd_max_nesting() {
        // Generate deeply nested array
        let mut nested = String::from("[");
        for _ in 0..100 {
            // 100 levels deep
            nested.push_str("1,[");
        }
        nested.push_str("1");
        nested.push_str("]".repeat(100).as_str());

        let input = nested.as_bytes().to_vec();
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Array(arr)) => {
                let mut current = &arr[1];
                for _ in 0..99 {
                    match current {
                        JsonValue::Array(nested) => current = &nested[1],
                        _ => panic!("Expected nested array"),
                    }
                }
            }
            _ => panic!("Expected array"),
        }
    }

    #[test]
    fn test_simd_mixed_chunks() {
        // Test mixing of different types that would cross SIMD chunk boundaries
        let long_key = "k".repeat(31); // Just under SIMD width
        let input = format!(
            r#"{{
           "{}": [1, 2, 3],
           "{}": {{"nested": true}},
           "{}": "string value",
           "{}": null,
           "{}": 42.5
       }}"#,
            long_key, long_key, long_key, long_key, long_key
        )
        .as_bytes()
        .to_vec();

        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(JsonValue::Object(map)) => assert_eq!(map.len(), 5),
            _ => panic!("Expected object"),
        }
    }

    #[test]
    fn test_simd_error_locations() {
        let test_cases = vec![
            // Error at SIMD boundary
            (format!(r#"{{"key": "{}"}}"#, "a".repeat(31)), None),
            // Unterminated string crossing boundary
            (
                format!(r#"{{"key": "{}"#, "a".repeat(32)),
                Some(JsonError::UnexpectedEof),
            ),
            // Invalid escape sequence at boundary
            (
                format!(r#"{{"key": "{}\x00"}}"#, "a".repeat(30)),
                Some(JsonError::InvalidEscape),
            ),
            // Malformed number at boundary
            (
                format!(r#"{{"key": {}.}}"#, "1".repeat(31)),
                Some(JsonError::InvalidNumber("Invalid float".into())),
            ),
        ];

        for (input, expected_err) in test_cases {
            let mut parser = Parser::new(input.as_bytes().to_vec());
            match (parser.parse(), expected_err) {
                (Ok(_), None) => continue,
                (Err(e), Some(expected)) => assert_eq!(e, expected),
                _ => panic!("Unexpected parser result"),
            }
        }
    }

    #[test]
    fn test_simd_number_boundaries() {
        let test_cases = vec![
            // Split scientific notation
            format!(r#"{{"key": {}e-10}}"#, "1".repeat(31)),
            // Split decimal point
            format!(r#"{{"key": {}.123}}"#, "1".repeat(31)),
            // Split negative sign
            format!(r#"{{"k": "{}", "v": -{}}}"#, "a".repeat(31), "1".repeat(5)),
            // Large integers at boundary
            format!(r#"{{"k": "{}", "v": {}}}"#, "a".repeat(31), i64::MAX),
        ];

        for input in test_cases {
            let mut parser = Parser::new(input.as_bytes().to_vec());
            assert!(parser.parse().is_ok());
        }
    }

    #[test]
    fn test_simd_exact_boundaries() {
        // Test exactly 32 bytes (AVX2 register size)
        let exact_32 = r#"{"key":"aaaaaaaaaaaaaaaaaaa"}"#; // 32 bytes
        let mut parser = Parser::new(exact_32.as_bytes().to_vec());
        assert!(parser.parse().is_ok());

        // Test exactly 64 bytes
        let exact_64 = r#"{"key":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#;
        let mut parser = Parser::new(exact_64.as_bytes().to_vec());
        assert!(parser.parse().is_ok());
    }

    #[test]
    fn test_simd_whitespace_boundaries() {
        let input = format!(
            "{{ {}     \n\t\r     \"key\": true }}",
            " ".repeat(30) // Push right up to SIMD boundary
        );
        let mut parser = Parser::new(input.as_bytes().to_vec());
        assert!(parser.parse().is_ok());
    }
}
