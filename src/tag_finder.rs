//! Incremental tag-scanner:  <Tag> … (raw bytes) … </Tag>

use crate::json_types::JsonError;

#[derive(Debug)]
pub enum TagEvent {
    Open,          // “<Something>”
    Bytes(String), // payload between the tags
    Close,         // “</Something>”
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

    /// Feed the next text chunk.
    pub fn push(
        &mut self,
        chunk: &str,
        mut emit: impl FnMut(TagEvent) -> Result<(), JsonError>,
    ) -> Result<(), JsonError> {
        self.buf.push_str(chunk);

        loop {
            /*──────── look for the next '<' ────────────────────────*/
            let lt = match self.buf.find('<') {
                Some(i) => i,
                None => break,
            };

            /*──────── everything *before* it is payload ───────────*/
            if self.inside && lt > 0 {
                let payload = self.buf[..lt].to_owned();
                if !payload.is_empty() {
                    emit(TagEvent::Bytes(payload))?;
                }
            }

            /*──────── check we also have a matching '>' ───────────*/
            let gt = match self.buf[lt..].find('>') {
                Some(off) => lt + off,
                None => {
                    // tag split across chunks
                    // keep the tail for the next push() call
                    self.buf.drain(..lt); // drop what we already handled
                    return Ok(());
                }
            };

            /*──────── analyse the tag ─────────────────────────────*/
            let tag_body = &self.buf[lt + 1..gt]; // without the '<' or '>'
            let is_close = tag_body.starts_with('/');

            if !self.inside && !is_close {
                /* <Tag> : enter streaming mode */
                self.inside = true;
                emit(TagEvent::Open)?;
            } else if self.inside && is_close {
                /* </Tag> : exit streaming mode */
                emit(TagEvent::Close)?;
                self.inside = false;
            }
            // else: either nested <…> or stray </…> – ignore.

            /*──────── consume the tag itself ─────────────────────*/
            self.buf.drain(..gt + 1);
        }

        /*──────── no '<' left in the buffer ──────────────────────*/
        if self.inside && !self.buf.is_empty() {
            emit(TagEvent::Bytes(std::mem::take(&mut self.buf)))?;
        } else {
            // keep only a very small tail (up to 4 chars) –
            // enough to recognise a tag start in the next push().
            let keep = self.buf.len().min(4);
            let tail = self.buf.split_off(self.buf.len() - keep);
            self.buf = tail;
        }
        Ok(())
    }
}
