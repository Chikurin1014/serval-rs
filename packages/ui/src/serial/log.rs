//! A log file's bytes, made from the traffic as it comes: nothing is kept
//! but the end of an unfinished line.

use std::rc::Rc;

use super::{LocalFuture, SerialResult};
use crate::{
    data::ByteData,
    helper::{AnsiStripper, Delimiter, decode_utf8, split_lines},
};

/// Where a log's bytes go as they are made: a file, by the platform.
pub trait LogSink {
    /// In order after the earlier writes; a failure shows when it closes.
    fn write(&self, bytes: Vec<u8>);

    /// After the writes: nothing more is written. Fails if any write did.
    fn close(&self) -> LocalFuture<SerialResult<()>>;
}

/// What a log file in `format` is named with.
pub fn extension(format: LogFormat) -> &'static str {
    match format {
        LogFormat::Raw => "bin",
        LogFormat::Text { .. } => "log",
        LogFormat::Hex { .. } => "tsv",
    }
}

/// A log being written: what the port receives and sends, to its sink.
pub(crate) struct Logger {
    serializer: LogSerializer,
    sink: Rc<dyn LogSink>,
}

impl Logger {
    pub(crate) fn new(format: LogFormat, sink: Rc<dyn LogSink>) -> Self {
        Self {
            serializer: LogSerializer::new(format),
            sink,
        }
    }

    /// In one write, for the chunks gathered together.
    pub(crate) fn received(&mut self, chunks: &[ByteData]) {
        let bytes = chunks
            .iter()
            .flat_map(|chunk| self.serializer.received(chunk.timestamp(), chunk.value()))
            .collect();
        self.write(bytes);
    }

    pub(crate) fn sent(&mut self, timestamp: i64, bytes: &[u8]) {
        let bytes = self.serializer.sent(timestamp, bytes);
        self.write(bytes);
    }

    /// Writes the unfinished lines, then closes the sink.
    pub(crate) fn close(mut self) -> LocalFuture<SerialResult<()>> {
        let rest = self.serializer.finish();
        self.write(rest);
        self.sink.close()
    }

    fn write(&self, bytes: Vec<u8>) {
        if !bytes.is_empty() {
            self.sink.write(bytes);
        }
    }
}

/// What a log keeps of the traffic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogFormat {
    /// The received bytes as they are.
    Raw,
    /// Received lines as text, without ANSI escape sequences or control
    /// characters but tabs. With `sent`, the sent lines too, marked `>` (and
    /// the received ones `<`).
    Text { timestamps: bool, sent: bool },
    /// A line for each chunk: its time, `RX` or `TX`, and its bytes in hex.
    Hex { sent: bool },
}

/// Turns chunks into the bytes to add to a log in `format`.
pub struct LogSerializer {
    format: LogFormat,
    received: Lines,
    sent: Lines,
}

#[derive(Clone, Copy, PartialEq)]
enum Direction {
    Received,
    Sent,
}

impl LogSerializer {
    pub fn new(format: LogFormat) -> Self {
        Self {
            format,
            received: Lines::default(),
            sent: Lines::default(),
        }
    }

    pub fn received(&mut self, timestamp: i64, bytes: &[u8]) -> Vec<u8> {
        self.add(Direction::Received, timestamp, bytes)
    }

    /// Nothing unless the format keeps what is sent.
    pub fn sent(&mut self, timestamp: i64, bytes: &[u8]) -> Vec<u8> {
        self.add(Direction::Sent, timestamp, bytes)
    }

    /// The unfinished lines, for the end of the log.
    pub fn finish(&mut self) -> Vec<u8> {
        let LogFormat::Text { sent, .. } = self.format else {
            return Vec::new();
        };
        let mut log = String::new();
        if let Some((start, line)) = self.received.finish() {
            log.push_str(&self.text_line(Direction::Received, start, &line));
        }
        if sent {
            if let Some((start, line)) = self.sent.finish() {
                log.push_str(&self.text_line(Direction::Sent, start, &line));
            }
        }
        log.into_bytes()
    }

    fn add(&mut self, direction: Direction, timestamp: i64, bytes: &[u8]) -> Vec<u8> {
        match self.format {
            LogFormat::Raw => match direction {
                Direction::Received => bytes.to_vec(),
                Direction::Sent => Vec::new(),
            },
            LogFormat::Text { sent, .. } => {
                if direction == Direction::Sent && !sent {
                    return Vec::new();
                }
                let lines = match direction {
                    Direction::Received => self.received.push(timestamp, bytes),
                    Direction::Sent => self.sent.push(timestamp, bytes),
                };
                lines
                    .into_iter()
                    .map(|(start, line)| self.text_line(direction, start, &line))
                    .collect::<String>()
                    .into_bytes()
            }
            LogFormat::Hex { sent } => {
                if (direction == Direction::Sent && !sent) || bytes.is_empty() {
                    return Vec::new();
                }
                let hex = bytes
                    .iter()
                    .map(|byte| format!("{byte:02X}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                let direction = match direction {
                    Direction::Received => "RX",
                    Direction::Sent => "TX",
                };
                format!("{}\t{direction}\t{hex}\n", unix_time(timestamp)).into_bytes()
            }
        }
    }

    fn text_line(&self, direction: Direction, start: i64, line: &str) -> String {
        let LogFormat::Text { timestamps, sent } = self.format else {
            unreachable!("only text has lines");
        };
        let mut text = String::new();
        if timestamps {
            text.push_str(&format!("[{}] ", unix_time(start)));
        }
        if sent {
            text.push_str(match direction {
                Direction::Received => "< ",
                Direction::Sent => "> ",
            });
        }
        text.push_str(line);
        text.push('\n');
        text
    }
}

/// Text read into lines, ended by CR, LF or CRLF; a CR ends its line at once.
#[derive(Default)]
struct Lines {
    /// The start of a character cut off at the end of the previous chunk.
    pending: Vec<u8>,
    stripper: AnsiStripper,
    /// After the last line end.
    rest: String,
    /// When the first byte of `rest` came.
    start: Option<i64>,
    /// Whether the last line ended with a CR, which an LF may finish.
    after_cr: bool,
}

impl Lines {
    /// The lines `bytes` finish, each with when its first byte came.
    fn push(&mut self, timestamp: i64, bytes: &[u8]) -> Vec<(i64, String)> {
        let mut text = self.stripper.strip(&decode_utf8(&mut self.pending, bytes));
        if std::mem::take(&mut self.after_cr) && text.starts_with('\n') {
            text.remove(0);
        }
        if text.is_empty() {
            return Vec::new();
        }
        let mut start = *self.start.get_or_insert(timestamp);
        self.rest.push_str(&text);
        let (mut lines, mut rest) = split_lines(&self.rest, &Delimiter::ALL);
        // A last CR ends its line now, not when what follows shows it is no CRLF
        if let Some(line) = rest.strip_suffix('\r') {
            lines.push((line.to_string(), Delimiter::Cr));
            rest.clear();
            self.after_cr = true;
        }
        let lines = lines
            .into_iter()
            .map(|(line, _)| {
                let line = (start, plain(&line));
                // The next one starts in this chunk
                start = timestamp;
                line
            })
            .collect();
        self.start = (!rest.is_empty()).then_some(start);
        self.rest = rest;
        lines
    }

    fn finish(&mut self) -> Option<(i64, String)> {
        if !self.pending.is_empty() {
            self.pending.clear();
            self.rest.push(char::REPLACEMENT_CHARACTER);
        }
        let rest = std::mem::take(&mut self.rest);
        let start = self.start.take()?;
        let line = plain(&rest);
        (!line.is_empty()).then_some((start, line))
    }
}

/// Unix time in seconds, to the millisecond: e.g. `1791409892.542`.
fn unix_time(ms: i64) -> String {
    format!("{}.{:03}", ms.div_euclid(1000), ms.rem_euclid(1000))
}

/// `line` without control characters but tabs.
fn plain(line: &str) -> String {
    line.chars()
        .filter(|&c| c == '\t' || !c.is_control())
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::RefCell;

    use super::*;

    /// What was written, and whether it was closed.
    #[derive(Default)]
    pub(crate) struct Recorded {
        pub(crate) writes: RefCell<Vec<Vec<u8>>>,
        pub(crate) closed: RefCell<bool>,
    }

    impl LogSink for Recorded {
        fn write(&self, bytes: Vec<u8>) {
            self.writes.borrow_mut().push(bytes);
        }

        fn close(&self) -> LocalFuture<SerialResult<()>> {
            *self.closed.borrow_mut() = true;
            Box::pin(async { Ok(()) })
        }
    }

    impl Recorded {
        pub(crate) fn text(&self) -> String {
            String::from_utf8(self.writes.borrow().concat()).unwrap()
        }
    }

    #[test]
    fn logger_writes_the_gathered_chunks_at_once() {
        let sink = Rc::new(Recorded::default());
        let mut logger = Logger::new(
            LogFormat::Text {
                timestamps: false,
                sent: true,
            },
            sink.clone(),
        );
        logger.received(&[
            ByteData::new(1, b"temp:20".to_vec()),
            ByteData::new(2, b".5\nvolt:3.3\n".to_vec()),
        ]);
        assert_eq!(sink.writes.borrow().len(), 1);
        assert_eq!(sink.text(), "< temp:20.5\n< volt:3.3\n");

        // Nothing to write: no write
        logger.received(&[ByteData::new(3, b"led".to_vec())]);
        logger.sent(4, b"l");
        assert_eq!(sink.writes.borrow().len(), 1);
    }

    #[test]
    fn logger_closes_after_the_unfinished_lines() {
        let sink = Rc::new(Recorded::default());
        let mut logger = Logger::new(
            LogFormat::Text {
                timestamps: false,
                sent: false,
            },
            sink.clone(),
        );
        logger.received(&[ByteData::new(1, b"login: ".to_vec())]);
        assert!(!*sink.closed.borrow());
        drop(logger.close());
        assert_eq!(sink.text(), "login: \n");
        assert!(*sink.closed.borrow());
    }

    fn serializer(format: LogFormat) -> LogSerializer {
        LogSerializer::new(format)
    }

    fn text(log: Vec<u8>) -> String {
        String::from_utf8(log).unwrap()
    }

    const TEXT: LogFormat = LogFormat::Text {
        timestamps: true,
        sent: false,
    };

    #[test]
    fn times_are_unix_time_in_seconds_to_the_millisecond() {
        assert_eq!(unix_time(1_791_409_892_542), "1791409892.542");
        assert_eq!(unix_time(1_791_409_892_005), "1791409892.005");
        assert_eq!(unix_time(0), "0.000");
    }

    #[test]
    fn raw_keeps_the_received_bytes_as_they_are() {
        let mut log = serializer(LogFormat::Raw);
        assert_eq!(log.received(1, b"\x1b[1mok\r\n\xff"), b"\x1b[1mok\r\n\xff");
        assert_eq!(log.sent(2, b"led on\r"), b"");
        assert_eq!(log.finish(), b"");
    }

    #[test]
    fn text_lines_take_the_time_their_first_byte_came() {
        let mut log = serializer(TEXT);
        assert_eq!(text(log.received(1, b"temp:2")), "");
        assert_eq!(
            text(log.received(2, b"0.5\nvolt:3.3\nle")),
            "[0.001] temp:20.5\n[0.002] volt:3.3\n"
        );
        assert_eq!(text(log.received(3, b"d: on\n")), "[0.002] led: on\n");
    }

    #[test]
    fn text_ends_lines_at_cr_lf_or_crlf() {
        let mut log = serializer(TEXT);
        assert_eq!(
            text(log.received(1, b"a\r\nb\rc\nd")),
            "[0.001] a\n[0.001] b\n[0.001] c\n"
        );
        // A last CR ends its line at once, and the LF after it is not another
        assert_eq!(text(log.received(2, b"\r")), "[0.001] d\n");
        assert_eq!(text(log.received(3, b"\ne\n")), "[0.003] e\n");
    }

    #[test]
    fn text_drops_escape_sequences_and_control_characters_but_tabs() {
        let mut log = serializer(TEXT);
        assert_eq!(
            text(log.received(1, b"\x1b[32mOK\x1b[0m\x07\tdone\x1b[")),
            ""
        );
        assert_eq!(text(log.received(2, b"K\n")), "[0.001] OK\tdone\n");
    }

    #[test]
    fn text_joins_a_character_cut_between_chunks() {
        let mut log = serializer(TEXT);
        let bytes = "温度\n".as_bytes();
        assert_eq!(text(log.received(1, &bytes[..4])), "");
        assert_eq!(text(log.received(2, &bytes[4..])), "[0.001] 温度\n");
    }

    #[test]
    fn text_finish_gives_the_unfinished_lines() {
        let mut log = serializer(LogFormat::Text {
            timestamps: false,
            sent: true,
        });
        assert_eq!(text(log.received(1, b"login: ")), "");
        assert_eq!(text(log.sent(2, b"roo")), "");
        assert_eq!(text(log.finish()), "< login: \n> roo\n");
        assert_eq!(log.finish(), b"");
    }

    #[test]
    fn text_marks_sent_lines_typed_key_by_key() {
        let mut log = serializer(LogFormat::Text {
            timestamps: true,
            sent: true,
        });
        for (time, key) in [(1, "l"), (2, "s"), (3, "\r")] {
            let line = text(log.sent(time, key.as_bytes()));
            assert_eq!(line, if key == "\r" { "[0.001] > ls\n" } else { "" });
        }
        assert_eq!(text(log.received(4, b"a.txt\r\n")), "[0.004] < a.txt\n");
    }

    #[test]
    fn text_leaves_out_what_is_sent_unless_asked() {
        let mut log = serializer(TEXT);
        assert_eq!(log.sent(1, b"ls\r"), b"");
        assert_eq!(log.finish(), b"");
    }

    #[test]
    fn hex_gives_a_line_for_each_chunk() {
        let mut log = serializer(LogFormat::Hex { sent: true });
        assert_eq!(text(log.received(1, b"ok\r\n")), "0.001\tRX\t6F 6B 0D 0A\n");
        assert_eq!(text(log.sent(2, &[0x02, 0xff])), "0.002\tTX\t02 FF\n");
        assert_eq!(log.received(3, b""), b"");

        let mut log = serializer(LogFormat::Hex { sent: false });
        assert_eq!(log.sent(1, b"x"), b"");
    }
}
