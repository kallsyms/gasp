// src/rd_json_stream.rs
use crate::json_types::{JsonError, JsonValue};
// Lightweight streaming wrapper for the existing SIMD / fallback JSON parser.
// It leaves `rd_json_stack_parser::Parser` completely untouched.

use crate::rd_json_stack_parser::Parser as Core;
// stream_json_parser.rs

#[derive(Debug, Clone, PartialEq)]
pub enum Path {
    RootField(String),
    ArrayElem(usize),
}

#[derive(Debug)]
pub enum StreamEvent {
    Partial(Path, JsonValue),
    Complete(JsonValue),
    NeedMore,
}

pub struct StreamParser {
    buf: Vec<u8>, // grows with every push()
    last_snapshot: Option<JsonValue>,
    root_closed: bool,
}

impl StreamParser {
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            last_snapshot: None,
            root_closed: false,
        }
    }

    pub fn push(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    pub fn advance(&mut self) -> Result<StreamEvent, JsonError> {
        if self.root_closed {
            return Ok(StreamEvent::NeedMore);
        }

        // try to (re-)parse from scratch in **streaming** mode
        let parse_result = Core::new_stream(self.buf.clone()).parse();

        match parse_result {
            // ----------------- finished JSON -----------------
            Ok(tree) => {
                self.root_closed = true;
                self.last_snapshot = Some(tree.clone());
                return Ok(StreamEvent::Complete(tree));
            }

            // ----------------- mid-stream snapshot -----------------
            Err(JsonError::StreamingSnapshot(tree)) => {
                // figure out what, if anything, is new since last snapshot
                let evt = if let Some(prev) = &self.last_snapshot {
                    match diff_root(prev, &tree) {
                        Some((p, v)) => StreamEvent::Partial(p, v),
                        None => StreamEvent::NeedMore,
                    }
                } else {
                    // first-ever snapshot
                    first_fragment_event(&tree).unwrap_or(StreamEvent::NeedMore)
                };

                self.last_snapshot = Some(tree);
                return Ok(evt);
            }

            // ----------------- need more bytes -----------------
            Err(JsonError::UnexpectedEof) => Ok(StreamEvent::NeedMore),

            // ----------------- real parsing error -----------------
            Err(e) => Err(e),
        }
    }
}

/*----------------------------------------------------------
 * Helper utilities (unchanged in spirit)
 *----------------------------------------------------------*/

fn is_closed_json(_root: &JsonValue, buf: &[u8]) -> bool {
    // nothing but whitespace after the last non-WS char
    let mut i = buf.len();
    while i > 0 && buf[i - 1].is_ascii_whitespace() {
        i -= 1;
    }
    i == buf.len() // already at end ⇒ closed
}

fn first_fragment_event(tree: &JsonValue) -> Option<StreamEvent> {
    match tree {
        JsonValue::Object(map) if map.len() == 1 => {
            let (k, v) = map.iter().next().unwrap();
            Some(StreamEvent::Partial(Path::RootField(k.clone()), v.clone()))
        }
        JsonValue::Array(arr) if !arr.is_empty() => {
            Some(StreamEvent::Partial(Path::ArrayElem(0), arr[0].clone()))
        }
        _ => None,
    }
}

fn diff_root(prev: &JsonValue, curr: &JsonValue) -> Option<(Path, JsonValue)> {
    match (prev, curr) {
        (JsonValue::Object(p), JsonValue::Object(c)) if c.len() > p.len() => c
            .iter()
            .find(|(k, _)| !p.contains_key(*k))
            .map(|(k, v)| (Path::RootField(k.clone()), v.clone())),
        (JsonValue::Object(p), JsonValue::Object(c)) => {
            for (k, v_curr) in c {
                if let Some(v_prev) = p.get(k) {
                    match (v_prev, v_curr) {
                        (JsonValue::String(s0), JsonValue::String(s1)) if s1.len() > s0.len() => {
                            return Some((Path::RootField(k.clone()), v_curr.clone()))
                        }
                        (JsonValue::Array(a0), JsonValue::Array(a1)) if a1.len() > a0.len() => {
                            return Some((Path::RootField(k.clone()), v_curr.clone()))
                        }
                        (JsonValue::Object(o0), JsonValue::Object(o1)) if o1.len() > o0.len() => {
                            return Some((Path::RootField(k.clone()), v_curr.clone()))
                        }
                        _ => {}
                    }
                }
            }
            None
        }
        (JsonValue::Array(p), JsonValue::Array(c)) if c.len() > p.len() => {
            Some((Path::ArrayElem(p.len()), c[p.len()].clone()))
        }
        _ => None,
    }
}

/*-------------------------------------------------------------
 * Tests
 *-----------------------------------------------------------*/
#[cfg(test)]
mod tests {
    use super::*;
    use crate::json_types::JsonValue;

    #[test]
    fn object_fields_once_each() {
        let mut sp = StreamParser::new();
        sp.push(br#"{"a":1,"b":"hi"#); // 'b' incomplete
        let e1 = sp.advance().unwrap();

        println!("e1 = {:?}", e1);
        match e1 {
            StreamEvent::Partial(Path::RootField(k), _) => assert_eq!(k, "a"),
            _ => panic!("expected Partial for 'a'"),
        }

        sp.push(br#"!"#); // still in the middle of 'b'
        assert!(matches!(sp.advance().unwrap(), StreamEvent::NeedMore));

        sp.push(br#""}"#); // close string and object
        let e2 = sp.advance().unwrap();

        println!("e2 = {:?}", e2);
        match e2 {
            StreamEvent::Partial(Path::RootField(k), JsonValue::String(s)) => {
                assert_eq!(k, "b");
                assert_eq!(s, "hi!");
            }
            _ => panic!("expected Partial for 'b'"),
        }

        // Final call -> Complete
        match sp.advance().unwrap() {
            StreamEvent::Complete(JsonValue::Object(map)) => {
                assert_eq!(map.len(), 2);
            }
            _ => panic!("expected Complete"),
        }
    }

    #[test]
    fn array_partials_and_complete() {
        let mut sp = StreamParser::new();
        sp.push(br#"["x","#); // first elem done, second starting
        match sp.advance().unwrap() {
            StreamEvent::Partial(Path::ArrayElem(0), JsonValue::String(s)) => assert_eq!(s, "x"),
            _ => panic!("expected element 0"),
        }

        sp.push(br#""y","#); // second incomplete
        let x = sp.advance().unwrap();
        println!("x = {:?}", x);

        assert!(matches!(x, StreamEvent::Partial(_, _)));

        sp.push(br#""z"]"#); // finish array
                             // element 1 Partial
                             // Complete
        assert!(matches!(sp.advance().unwrap(), StreamEvent::Complete(_)));
    }
}
