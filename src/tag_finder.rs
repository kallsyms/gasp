//! Incremental tag-scanner:  <Tag> … (raw bytes) … </Tag>

use crate::json_types::JsonError;

#[derive(Debug)]
pub enum TagEvent {
    Open(String),  // <Tag>
    Bytes(String), // payload
    Close(String), // </Tag>
}

#[derive(Default, Debug)]
pub struct TagFinder {
    buf: String,  // carries over up to a whole unfinished tag
    inside: bool, // true ⇢ we’re between <Tag> … </Tag>
}

impl TagFinder {
    pub fn new() -> Self {
        Self::default()
    }
    /// Feed the next text chunk, emitting TagEvents.
    /// `emit` will be called with:
    ///   • TagEvent::Open  { name }
    ///   • TagEvent::Bytes(payload)
    ///   • TagEvent::Close { name }
    pub fn push(
        &mut self,
        chunk: &str,
        mut emit: impl FnMut(TagEvent) -> Result<(), JsonError>,
    ) -> Result<(), JsonError> {
        self.buf.push_str(chunk);

        loop {
            /*──────── look for the next '<' ───────────────────────────*/
            let lt = match self.buf.find('<') {
                Some(i) => i,
                None => break,
            };

            /*──────── everything *before* it is payload ──────────────*/
            if self.inside && lt > 0 {
                let payload = self.buf[..lt].to_owned();
                if !payload.is_empty() {
                    emit(TagEvent::Bytes(payload))?;
                }
            }

            /*──────── look for the matching '>' ───────────────────────*/
            let gt = match self.buf[lt..].find('>') {
                Some(off) => lt + off,
                None => {
                    // tag split across chunks → keep tail for next push()
                    self.buf.drain(..lt); // drop handled bytes
                    return Ok(());
                }
            };

            /*──────── analyse the tag ────────────────────────────────*/
            let tag_body = &self.buf[lt + 1..gt]; // without '<' / '>'
            let is_close = tag_body.starts_with('/');
            let name_part = if is_close { &tag_body[1..] } else { tag_body };
            // strip attributes if present, keep only the tag name
            let name = name_part.split_whitespace().next().unwrap_or("").to_owned();

            // Debug output for tests
            // println!("Tag found: '{}', is_close: {}, inside: {}", name, is_close, self.inside);

            if !self.inside && !is_close {
                /* <Tag> : enter streaming mode */
                self.inside = true;
                emit(TagEvent::Open(name))?;
            } else if self.inside && is_close {
                /* </Tag> : exit streaming mode */
                emit(TagEvent::Close(name))?;
                self.inside = false;
            }
            // else: nested <…> or stray </…> – ignore.

            /*──────── consume the tag itself ─────────────────────────*/
            self.buf.drain(..gt + 1);
        }

        /*──────── no '<' left in buffer – handle tail ───────────────*/
        if self.inside && !self.buf.is_empty() {
            emit(TagEvent::Bytes(std::mem::take(&mut self.buf)))?;
        } else {
            // keep only a tiny tail (≤200 chars) to recognise a split tag
            let keep = self.buf.len().min(200);
            let tail = self.buf.split_off(self.buf.len() - keep);
            self.buf = tail;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_tag_handling() {
        // Test case: Tag name split across chunks
        // For example: "<Report" and "Sub>" as separate chunks

        let mut finder = TagFinder::new();
        let mut events = Vec::new();

        // First chunk contains part of opening tag
        finder
            .push("<Report", |event| {
                events.push(event);
                Ok(())
            })
            .unwrap();

        // Assert no events emitted yet since the tag is incomplete
        assert_eq!(events.len(), 0);

        // Second chunk completes the opening tag
        finder
            .push("Sub>{", |event| {
                events.push(event);
                Ok(())
            })
            .unwrap();

        // Print the actual events for debugging
        println!("Events after second chunk: {:?}", events);

        // The TagFinder should have emitted at least one event (Open)
        assert!(!events.is_empty(), "No events emitted after opening tag");

        // Check that the ReportSub tag was recognized in some form
        let mut found_tag = false;
        for event in &events {
            if let TagEvent::Open(name) = event {
                if name == "ReportSub" {
                    found_tag = true;
                    break;
                }
            }
        }
        assert!(found_tag, "'ReportSub' found");

        // Clear events to test the final chunk
        events.clear();

        // Third chunk has content and closing tag
        finder
            .push(" more content</ReportSub>", |event| {
                events.push(event);
                Ok(())
            })
            .unwrap();

        println!("Events after third chunk: {:?}", events);

        // We should have at least the Close event
        let mut has_close = false;
        let mut has_bytes = false;

        for event in &events {
            match event {
                TagEvent::Close(name) if name == "ReportSub" => has_close = true,
                TagEvent::Bytes(_) => has_bytes = true,
                _ => {}
            }
        }

        assert!(has_close, "No close event for ReportSub tag");
        assert!(has_bytes, "No bytes event for content");
    }

    #[test]
    fn test_extreme_tag_splitting() {
        // Test case where tag opening bracket, name, and closing bracket are all in separate chunks

        let mut finder = TagFinder::new();
        let mut events = Vec::new();

        // First chunk just has opening bracket
        finder
            .push("<", |event| {
                events.push(event);
                Ok(())
            })
            .unwrap();

        println!("Events after first chunk: {:?}", events);

        // Second chunk has tag name
        finder
            .push("ReportSub", |event| {
                events.push(event);
                Ok(())
            })
            .unwrap();

        println!("Events after second chunk: {:?}", events);

        // Third chunk completes the opening tag
        finder
            .push(">content", |event| {
                events.push(event);
                Ok(())
            })
            .unwrap();

        println!("Events after third chunk: {:?}", events);

        // Check for any Open event
        let mut has_open_report = false;
        for event in &events {
            if let TagEvent::Open(name) = event {
                if name == "ReportSub" {
                    has_open_report = true;
                    break;
                }
            }
        }

        assert!(has_open_report, "No open event for ReportSub tag found");

        // Now test closing tag in chunks
        events.clear();

        finder
            .push("</Report", |event| {
                events.push(event);
                Ok(())
            })
            .unwrap();

        finder
            .push("Sub>", |event| {
                events.push(event);
                Ok(())
            })
            .unwrap();

        println!("Events after closing tag: {:?}", events);

        // Check for a Close event
        let mut has_close = false;
        for event in &events {
            if let TagEvent::Close(name) = event {
                if name == "ReportSub" {
                    has_close = true;
                    break;
                }
            }
        }

        assert!(has_close, "No close event for ReportSub tag found");
    }
}
