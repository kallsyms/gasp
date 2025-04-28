//! `json_tok` – tolerant, state-carrying tokenizer that survives `push()`.
//
//  It never produces &str slices that outlive `&self.src`; instead every
//  token carries a `start..end` byte range back into the parser's buffer,
//  so the buffer can grow and re-allocate safely.

use crate::json_types::JsonError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Colon,
    Comma,
    True,
    False,
    Null,
    Num,
    Str,
    Ident,
    Eof,
}

#[derive(Debug, Clone, Copy)]
pub struct Tok {
    pub kind: Kind,
    pub start: usize, // byte offsets in the *current* buffer
    pub end: usize,
}

#[derive(Debug, Clone, Copy)]
enum LState {
    Start,
    InString { quote: u8 },
    InNumber { seen_dot: bool, seen_exp: bool },
    InIdent,
    InLineComment,
    InBlockComment { star: bool }, // saw ‘*’ last byte?
    Done,
}

pub struct Tokenizer {
    state: LState,
    pos: usize, // absolute byte position
}

impl Tokenizer {
    pub fn new() -> Self {
        Self {
            state: LState::Start,
            pos: 0,
        }
    }

    /// Re-initialise after `push()` with the same logical state.
    pub fn reset(&mut self, new_start: usize) {
        self.pos = new_start;
    }

    /// Return next token *or* `Tok {kind:Eof}` if no progress possible.
    /// `src` is the *entire* current buffer.
    pub fn next_tok(&mut self, src: &str) -> Result<Tok, JsonError> {
        use LState::*;
        let bytes = src.as_bytes();
        loop {
            match self.state {
                Start => {
                    if self.pos >= bytes.len() {
                        self.state = Done;
                        return Ok(eof(self.pos));
                    }
                    match bytes[self.pos] {
                        b'{' => return single(self, Kind::LBrace),
                        b'}' => return single(self, Kind::RBrace),
                        b'[' => return single(self, Kind::LBracket),
                        b']' => return single(self, Kind::RBracket),
                        b':' => return single(self, Kind::Colon),
                        b',' => return single(self, Kind::Comma),

                        b'"' | b'\'' => {
                            self.state = InString {
                                quote: bytes[self.pos],
                            };
                        }
                        b'0'..=b'9' | b'-' | b'.' => {
                            self.state = InNumber {
                                seen_dot: false,
                                seen_exp: false,
                            };
                        }
                        b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                            self.state = InIdent;
                        }
                        b'/' if peek(bytes, self.pos + 1) == Some(b'/') => {
                            self.state = InLineComment;
                            self.pos += 2;
                        }
                        b'/' if peek(bytes, self.pos + 1) == Some(b'*') => {
                            self.state = InBlockComment { star: false };
                            self.pos += 2;
                        }
                        b if b.is_ascii_whitespace() => {
                            self.pos += 1;
                        }
                        b => return Err(JsonError::UnexpectedChar(b as char)),
                    }
                }

                InString { quote } => {
                    let start = self.pos + 1; // skip opening quote
                    while self.pos + 1 < bytes.len() {
                        self.pos += 1;
                        match bytes[self.pos] {
                            b if b == quote => {
                                let end = self.pos;
                                self.pos += 1;
                                self.state = Start;
                                return Ok(Tok {
                                    kind: Kind::Str,
                                    start,
                                    end,
                                });
                            }
                            b'\\' => {
                                // skip escaped byte
                                self.pos += 1;
                                if self.pos >= bytes.len() {
                                    break;
                                }
                            }
                            _ => (),
                        }
                    }
                    return Err(JsonError::UnexpectedEof);
                }

                InNumber {
                    ref mut seen_dot,
                    ref mut seen_exp,
                } => {
                    let start = self.pos;
                    while self.pos < bytes.len() {
                        match bytes[self.pos] {
                            b'0'..=b'9' => self.pos += 1,
                            b'.' if !*seen_dot => {
                                *seen_dot = true;
                                self.pos += 1;
                            }
                            b'e' | b'E' if !*seen_exp => {
                                *seen_exp = true;
                                self.pos += 1;
                                if matches!(peek(bytes, self.pos), Some(b'+' | b'-')) {
                                    self.pos += 1;
                                }
                            }
                            _ => break,
                        }
                    }
                    if self.pos == start {
                        return Err(JsonError::UnexpectedChar(bytes[self.pos] as char));
                    }
                    let end = self.pos;
                    self.state = Start;
                    return Ok(Tok {
                        kind: Kind::Num,
                        start,
                        end,
                    });
                }

                InIdent => {
                    let start = self.pos;
                    while matches!(
                        peek(bytes, self.pos),
                        Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_')
                    ) {
                        self.pos += 1;
                    }
                    let s = &src[start..self.pos];
                    let kind = match s {
                        "true" => Kind::True,
                        "false" => Kind::False,
                        "null" => Kind::Null,
                        _ => Kind::Ident,
                    };
                    self.state = Start;
                    return Ok(Tok {
                        kind,
                        start,
                        end: self.pos,
                    });
                }

                InLineComment => {
                    while let Some(b) = peek(bytes, self.pos) {
                        self.pos += 1;
                        if b == b'\n' {
                            break;
                        }
                    }
                    self.state = Start;
                }

                InBlockComment { ref mut star } => {
                    while let Some(b) = peek(bytes, self.pos) {
                        self.pos += 1;
                        *star = b == b'*';
                        if *star && peek(bytes, self.pos) == Some(b'/') {
                            self.pos += 1;
                            self.state = Start;
                            break;
                        }
                    }
                    if self.pos >= bytes.len() {
                        return Err(JsonError::UnexpectedEof);
                    }
                }

                Done => return Ok(eof(self.pos)),
            }
        }

        #[inline]
        fn peek(bytes: &[u8], i: usize) -> Option<u8> {
            bytes.get(i).copied()
        }
        #[inline]
        fn single(tok: &mut Tokenizer, k: Kind) -> Result<Tok, JsonError> {
            let start = tok.pos;
            tok.pos += 1;
            Ok(Tok {
                kind: k,
                start,
                end: start + 1,
            })
        }
        #[inline]
        fn eof(pos: usize) -> Tok {
            Tok {
                kind: Kind::Eof,
                start: pos,
                end: pos,
            }
        }
    }
}
