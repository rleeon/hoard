//! Server-Sent Events framing, for the Hoard server's `GET /v1/events`.
//!
//! Only as much of the format as that stream uses: `event:` names the frame,
//! `data:` lines accumulate its body, a blank line ends it, and lines starting
//! with `:` are the keep-alive comments the server sends every 25 s.

#[derive(Debug, PartialEq, Eq)]
pub struct Event {
    pub kind: String,
    pub data: String,
}

#[derive(Default)]
pub struct Parser {
    buf: Vec<u8>,
    kind: String,
    data: String,
}

impl Parser {
    /// Feed bytes as they arrive and get back the events they complete. A chunk
    /// can end anywhere, mid-line or inside a multi-byte character, so lines
    /// are cut from the bytes first and only decoded once whole.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<Event> {
        self.buf.extend_from_slice(chunk);
        let mut out = Vec::new();
        while let Some(pos) = self.buf.iter().position(|b| *b == b'\n') {
            let raw: Vec<u8> = self.buf.drain(..=pos).collect();
            let text = String::from_utf8_lossy(&raw[..raw.len() - 1]);
            let line = text.strip_suffix('\r').unwrap_or(&text);
            if line.is_empty() {
                if !self.kind.is_empty() || !self.data.is_empty() {
                    out.push(Event {
                        kind: std::mem::take(&mut self.kind),
                        data: std::mem::take(&mut self.data),
                    });
                }
            } else if let Some(rest) = line.strip_prefix("event:") {
                self.kind = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("data:") {
                if !self.data.is_empty() {
                    self.data.push('\n');
                }
                self.data.push_str(rest.strip_prefix(' ').unwrap_or(rest));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAVE: &str = "event: save\ndata: {\"save_id\":\"a\",\"version_num\":3}\n\n";

    #[test]
    fn a_frame_split_at_every_byte_still_arrives_once() {
        let mut p = Parser::default();
        let mut got = Vec::new();
        for b in SAVE.as_bytes() {
            got.extend(p.push(&[*b]));
        }
        assert_eq!(
            got,
            vec![Event {
                kind: "save".into(),
                data: r#"{"save_id":"a","version_num":3}"#.into(),
            }]
        );
    }

    #[test]
    fn keep_alives_and_crlf_are_not_events() {
        let mut p = Parser::default();
        let got = p.push(b":\n\n: ping\r\n\r\nevent: lagged\r\ndata: \r\n\r\n");
        assert_eq!(
            got,
            vec![Event {
                kind: "lagged".into(),
                data: String::new(),
            }]
        );
    }

    #[test]
    fn data_lines_join_with_newlines_and_multibyte_splits_survive() {
        let mut p = Parser::default();
        let bytes = "event: save\ndata: ñandú\ndata: 𝄞\n\n".as_bytes();
        // Byte 34 falls inside the four-byte clef.
        let (a, b) = bytes.split_at(34);
        assert!(p.push(a).is_empty());
        let got = p.push(b);
        assert_eq!(got[0].data, "ñandú\n𝄞");
    }
}
