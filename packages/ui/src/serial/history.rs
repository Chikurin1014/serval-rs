//! The bytes received lately, for the console: apart from the data, so clearing
//! or capping that leaves them be.

use std::collections::VecDeque;

use crate::data::ByteData;

/// How many of the newest bytes are kept: about Tera Term's 10,000 lines.
pub const HISTORY_BYTES: usize = 1 << 20;

/// The newest received bytes, at most [`HISTORY_BYTES`], with when each chunk
/// of them came. Readers keep the position they read to, counted from the
/// first byte ever received.
#[derive(Default)]
pub struct ReceivedHistory {
    bytes: VecDeque<u8>,
    /// Where each chunk starts, and when it came; the first may have started
    /// before the bytes kept.
    chunks: VecDeque<(u64, i64)>,
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
    /// Where in `bytes` each chunk starts, and when it came; the first at 0.
    pub times: Vec<(usize, i64)>,
    /// The position to read from next time.
    pub end: u64,
}

impl ReceivedHistory {
    pub(crate) fn extend(&mut self, chunks: &[ByteData]) {
        self.extend_within(chunks, HISTORY_BYTES);
    }

    fn extend_within(&mut self, chunks: &[ByteData], max_bytes: usize) {
        for chunk in chunks.iter().filter(|chunk| !chunk.value().is_empty()) {
            self.chunks.push_back((self.end, chunk.timestamp()));
            self.bytes.extend(chunk.value());
            self.end += chunk.value().len() as u64;
        }
        let over = self.bytes.len().saturating_sub(max_bytes);
        self.bytes.drain(..over);
        let start = self.start();
        while self.chunks.get(1).is_some_and(|&(at, _)| at <= start) {
            self.chunks.pop_front();
        }
    }

    fn start(&self) -> u64 {
        self.end - self.bytes.len() as u64
    }

    /// What came after `read_to`; `None` for a reader that has read nothing.
    pub fn since(&self, read_to: Option<u64>) -> Unread {
        let start = self.start();
        let from = match read_to {
            Some(read_to) if read_to >= start => read_to,
            _ => start,
        };
        // The chunks from the one `from` is in
        let first = self
            .chunks
            .partition_point(|&(at, _)| at <= from)
            .saturating_sub(1);
        let times = self
            .chunks
            .range(first..)
            .filter(|_| from < self.end)
            .map(|&(at, time)| (at.saturating_sub(from) as usize, time))
            .collect();
        Unread {
            restarted: from != read_to.unwrap_or(u64::MAX),
            bytes: self
                .bytes
                .range((from - start) as usize..)
                .copied()
                .collect(),
            times,
            end: self.end,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(time: i64, bytes: &[u8]) -> ByteData {
        ByteData::new(time, bytes.to_vec())
    }

    #[test]
    fn a_reader_gets_what_came_after_it_last_read() {
        let mut history = ReceivedHistory::default();
        history.extend(&[chunk(1, b"led: o"), chunk(2, b"n\n")]);

        let first = history.since(None);
        assert_eq!(
            first,
            Unread {
                restarted: true,
                bytes: b"led: on\n".to_vec(),
                times: vec![(0, 1), (6, 2)],
                end: 8
            }
        );

        history.extend(&[chunk(3, b"ok\n")]);
        assert_eq!(
            history.since(Some(first.end)),
            Unread {
                restarted: false,
                bytes: b"ok\n".to_vec(),
                times: vec![(0, 3)],
                end: 11
            }
        );
        let none = history.since(Some(11));
        assert_eq!((none.bytes, none.times), (vec![], vec![]));
    }

    #[test]
    fn a_reader_inside_a_chunk_gets_its_time() {
        let mut history = ReceivedHistory::default();
        history.extend(&[chunk(1, b"abc"), chunk(2, b"de")]);
        assert_eq!(history.since(Some(1)).times, vec![(0, 1), (2, 2)]);
    }

    #[test]
    fn it_keeps_only_the_newest_bytes() {
        let mut history = ReceivedHistory::default();
        history.extend_within(&[chunk(1, b"abc"), chunk(2, b"defg")], 4);
        assert_eq!(history.since(None).bytes, b"defg");

        // Read up to "c", but "d" and on is all that is left: from the start
        history.extend_within(&[chunk(3, b"h")], 4);
        assert_eq!(
            history.since(Some(3)),
            Unread {
                restarted: true,
                bytes: b"efgh".to_vec(),
                // "e" is of the chunk "defg"
                times: vec![(0, 2), (3, 3)],
                end: 8
            }
        );
        assert_eq!(history.since(Some(6)).bytes, b"gh");
        assert_eq!(history.chunks.len(), 2);
    }
}
