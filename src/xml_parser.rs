use crate::xml_types::{XmlError, XmlValue};
use std::collections::HashMap;
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

/// Convert a vector of XML events to an XmlValue tree
pub fn events_to_xml_value(events: Vec<Result<Event, XmlError>>) -> Result<XmlValue, XmlError> {
    let mut stack: Vec<(String, HashMap<String, String>, Vec<XmlValue>)> = Vec::new();
    let mut current_text = String::new();

    for event in events {
        match event? {
            Event::ElementStart(tag) => {
                // Push any accumulated text as a child
                if !current_text.trim().is_empty() {
                    if let Some((_, _, ref mut children)) = stack.last_mut() {
                        children.push(XmlValue::Text(current_text.trim().to_string()));
                    }
                }
                current_text.clear();

                // Convert attributes
                let mut attrs = HashMap::new();
                for ((name, _), value) in tag.attributes {
                    attrs.insert(name, value);
                }

                // Push new element onto stack
                stack.push((tag.name, attrs, Vec::new()));
            }
            Event::ElementEnd(tag) => {
                // Push any accumulated text as a child
                if !current_text.trim().is_empty() {
                    if let Some((_, _, ref mut children)) = stack.last_mut() {
                        children.push(XmlValue::Text(current_text.trim().to_string()));
                    }
                }
                current_text.clear();

                // Pop the completed element
                if let Some((name, attrs, children)) = stack.pop() {
                    if name != tag.name {
                        return Err(XmlError::ParserError(format!(
                            "Mismatched tags: expected {}, got {}",
                            name, tag.name
                        )));
                    }

                    let element = XmlValue::Element(name, attrs, children);

                    if stack.is_empty() {
                        // This is the root element
                        return Ok(element);
                    } else {
                        // Add to parent's children
                        if let Some((_, _, ref mut parent_children)) = stack.last_mut() {
                            parent_children.push(element);
                        }
                    }
                }
            }
            Event::Characters(text) => {
                current_text.push_str(&text);
            }
            _ => {} // Ignore other events
        }
    }

    // If we have remaining text at the root level
    if !current_text.trim().is_empty() {
        return Ok(XmlValue::Text(current_text.trim().to_string()));
    }

    // If stack is not empty, we have unclosed tags
    if !stack.is_empty() {
        return Err(XmlError::ParserError("Unclosed XML tags".to_string()));
    }

    Err(XmlError::ParserError("No root element found".to_string()))
}
