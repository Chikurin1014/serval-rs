//! The bytes received lately, for the console: apart from the data, so clearing
//! or capping that leaves them be.

use std::collections::VecDeque;

use crate::data::ByteData;

/// How many of the newest bytes are kept: about Tera Term's 10,000 lines.
pub const HISTORY_BYTES: usize = 1 << 20;

/// The newest received bytes, at most [`HISTORY_BYTES`]. Readers keep the
/// position they read to, counted from the first byte ever received.
#[derive(Default)]
pub struct ReceivedHistory {
    bytes: VecDeque<u8>,
    /// The position after the newest byte.
    end: u64,
}

/// What a reader has not read yet.
#[derive(Debug, PartialEq)]
pub struct Unread {
    /// Whether some of it was dropped before it was read, or nothing was read
    /// yet: then `bytes` are all those kept, to show from the start.
    pub restarted: bool,
    pub bytes: Vec<u8>,
    /// The position to read from next time.
    pub end: u64,
}

impl ReceivedHistory {
    pub(crate) fn extend(&mut self, chunks: &[ByteData]) {
        self.extend_within(chunks, HISTORY_BYTES);
    }

    fn extend_within(&mut self, chunks: &[ByteData], max_bytes: usize) {
        for chunk in chunks {
            self.bytes.extend(chunk.value());
            self.end += chunk.value().len() as u64;
        }
        let over = self.bytes.len().saturating_sub(max_bytes);
        self.bytes.drain(..over);
    }

    /// What came after `read_to`; `None` for a reader that has read nothing.
    pub fn since(&self, read_to: Option<u64>) -> Unread {
        let start = self.end - self.bytes.len() as u64;
        let (restarted, from) = match read_to {
            Some(read_to) if read_to >= start => (false, read_to - start),
            _ => (true, 0),
        };
        Unread {
            restarted,
            bytes: self.bytes.range(from as usize..).copied().collect(),
            end: self.end,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(bytes: &[u8]) -> ByteData {
        ByteData::new(0, bytes.to_vec())
    }

    #[test]
    fn a_reader_gets_what_came_after_it_last_read() {
        let mut history = ReceivedHistory::default();
        history.extend(&[chunk(b"led: o"), chunk(b"n\n")]);

        let first = history.since(None);
        assert_eq!(
            first,
            Unread {
                restarted: true,
                bytes: b"led: on\n".to_vec(),
                end: 8
            }
        );

        history.extend(&[chunk(b"ok\n")]);
        assert_eq!(
            history.since(Some(first.end)),
            Unread {
                restarted: false,
                bytes: b"ok\n".to_vec(),
                end: 11
            }
        );
        assert_eq!(history.since(Some(11)).bytes, b"");
    }

    #[test]
    fn it_keeps_only_the_newest_bytes() {
        let mut history = ReceivedHistory::default();
        history.extend_within(&[chunk(b"abc"), chunk(b"defg")], 4);
        assert_eq!(history.since(None).bytes, b"defg");

        // Read up to "c", but "d" and on is all that is left: from the start
        history.extend_within(&[chunk(b"h")], 4);
        assert_eq!(
            history.since(Some(3)),
            Unread {
                restarted: true,
                bytes: b"efgh".to_vec(),
                end: 8
            }
        );
        assert_eq!(history.since(Some(6)).bytes, b"gh");
    }
}
