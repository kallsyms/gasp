use crate::xml_types::XmlError;
use std::fmt;
use xml::{Event, Parser};

pub struct StreamParser {
    parser: Parser,
    done: bool,
}

impl fmt::Debug for StreamParser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StreamParser")
            .field("done", &self.done)
            .finish()
    }
}

impl Default for StreamParser {
    fn default() -> Self {
        Self {
            parser: Parser::new(),
            done: false,
        }
    }
}

impl StreamParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_done(&self) -> bool {
        self.done
    }

    pub fn step(&mut self, chunk: &str) -> Result<Vec<Event>, XmlError> {
        self.parser.feed_str(chunk);
        self.parser
            .by_ref()
            .map(|e| e.map_err(|e| XmlError::ParserError(e.to_string())))
            .collect()
    }
}
