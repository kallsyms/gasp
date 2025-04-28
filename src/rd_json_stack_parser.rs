use crate::json_tok::{Kind, Tok, Tokenizer};
use crate::json_types::{JsonError, JsonValue, Number};
use crate::stream_json_parser::{Path, StreamEvent};
use std::collections::HashMap;

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

    fn with_mode(input: Vec<u8>, streaming: bool) -> Self {
        let mut me = Self {
            buf: String::from_utf8(input).unwrap(),
            cur: 0,
            lex: Tokenizer::new(),
            look: Tok {
                kind: Kind::Eof,
                start: 0,
                end: 0,
            },
            streaming,
            depth: 0,
        };
        me.refill_look().unwrap();
        me
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
        if self.depth == 0 {
            let val = self.value()?;
            let mut arr = vec![val];
            loop {
                match self.look.kind {
                    Kind::Comma => {
                        self.bump();
                    } // swallow separator
                    Kind::Eof => break,                     // done
                    Kind::RBracket | Kind::RBrace => break, // just in case
                    _ => (),                                // next value immediately
                }
                if self.look.kind == Kind::Eof {
                    break;
                }
                arr.push(self.value()?);
            }
            return Ok(Step::YieldComplete(JsonValue::Array(arr)));
        }
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

    /*==========================================================*/
    /*  recursive-descent value parsing                         */
    /*==========================================================*/
    fn value(&mut self) -> Result<JsonValue, JsonError> {
        match self.look.kind {
            Kind::LBrace => self.object(),
            Kind::LBracket => self.array(),
            Kind::Str => {
                let t = self.bump();
                Ok(JsonValue::String(self.buf[t.start..t.end].to_owned()))
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
                let t = self.bump();
                Ok(JsonValue::String(self.buf[t.start..t.end].to_owned()))
            }
            k => Err(JsonError::UnexpectedToken(format!("token {:?}", k))),
        }
    }

    fn object(&mut self) -> Result<JsonValue, JsonError> {
        self.depth += 1;
        self.bump(); // consume '{'
        let mut map = HashMap::new();
        while self.look.kind != Kind::RBrace {
            let key = match self.look.kind {
                Kind::Str | Kind::Ident => {
                    let t = self.bump();
                    self.buf[t.start..t.end].to_owned()
                }
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
        Ok(JsonValue::Object(map))
    }

    fn array(&mut self) -> Result<JsonValue, JsonError> {
        self.depth += 1;
        self.bump(); // consume '['
        let mut vec = Vec::new();
        while self.look.kind != Kind::RBracket {
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
        Ok(JsonValue::Array(vec))
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
