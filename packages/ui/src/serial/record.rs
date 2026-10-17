//! A record file's bytes, made from the traffic as it comes: nothing is kept
//! but the end of an unfinished line.

use std::rc::Rc;

mod detect;

pub use detect::detect_format;

use super::{LocalFuture, SerialResult};
use crate::{
    data::ByteData,
    helper::{AnsiStripper, Delimiter, decode_utf8, hex, split_lines},
};

/// Where a record's bytes go as they are made: a file, by the platform.
pub trait RecordSink {
    /// In order after the earlier writes; a failure stops the writes after
    /// it, and shows in [`Self::failure`] and as it closes.
    fn write(&self, bytes: Vec<u8>);

    /// After the writes: nothing more is written. Fails if any write did.
    fn close(&self) -> LocalFuture<SerialResult<()>>;

    /// The first failed write's error, as soon as it fails; `None` once
    /// closed without one, or for a sink that tells only as it closes.
    fn failure(&self) -> LocalFuture<Option<String>> {
        Box::pin(async { None })
    }
}

/// What a record file in `format` is named with.
pub fn extension(format: RecordFormat) -> &'static str {
    match format {
        RecordFormat::Raw => "bin",
        RecordFormat::Text { .. } => "log",
        RecordFormat::Hex { .. } => "tsv",
    }
}

/// How much of an existing record's end is read to tell its format.
pub const TAIL_BYTES: usize = 64 * 1024;

/// A file picked for a record.
pub struct RecordFile {
    pub sink: Rc<dyn RecordSink>,
    pub name: String,
    /// The end of what an existing file holds, at most [`TAIL_BYTES`].
    pub tail: Vec<u8>,
}

impl RecordFormat {
    pub fn name(self) -> &'static str {
        match self {
            Self::Raw => "Raw bytes",
            Self::Text { timestamps, sent } => match (timestamps, sent) {
                (false, false) => "Text",
                (false, true) => "Text with sent",
                (true, false) => "Text with time",
                (true, true) => "Text with time and sent",
            },
            Self::Hex { timestamps, sent } => match (timestamps, sent) {
                (false, false) => "Hex",
                (false, true) => "Hex with sent",
                (true, false) => "Hex with time",
                (true, true) => "Hex with time and sent",
            },
        }
    }
}

/// Around an event's line in text.
const EVENT_MARK: &str = "---";

/// A record being written: what the port receives and sends, to its sink.
pub(crate) struct Recorder {
    serializer: RecordSerializer,
    sink: Rc<dyn RecordSink>,
}

impl Recorder {
    pub(crate) fn new(format: RecordFormat, sink: Rc<dyn RecordSink>) -> Self {
        Self {
            serializer: RecordSerializer::new(format),
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

    /// See [`RecordSerializer::event`].
    pub(crate) fn event(&mut self, timestamp: i64, event: &str) {
        let bytes = self.serializer.event(timestamp, event);
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

/// What a record keeps of the traffic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordFormat {
    /// The received bytes as they are.
    Raw,
    /// Received lines as text, without ANSI escape sequences or control
    /// characters but tabs. With `sent`, the sent lines too, marked `>` (and
    /// the received ones `<`).
    Text { timestamps: bool, sent: bool },
    /// A line for each chunk: its bytes in hex, after its time (with
    /// `timestamps`) and `RX` or `TX` (with `sent`), tab separated.
    Hex { timestamps: bool, sent: bool },
}

/// Turns chunks into the bytes to add to a record in `format`.
pub struct RecordSerializer {
    format: RecordFormat,
    received: Lines,
    sent: Lines,
}

#[derive(Clone, Copy, PartialEq)]
enum Direction {
    Received,
    Sent,
}

impl RecordSerializer {
    pub fn new(format: RecordFormat) -> Self {
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

    /// The unfinished lines, for the end of the record.
    pub fn finish(&mut self) -> Vec<u8> {
        let RecordFormat::Text { sent, .. } = self.format else {
            return Vec::new();
        };
        let mut record = String::new();
        if let Some((start, line)) = self.received.finish() {
            record.push_str(&self.text_line(Direction::Received, start, &line));
        }
        if sent && let Some((start, line)) = self.sent.finish() {
            record.push_str(&self.text_line(Direction::Sent, start, &line));
        }
        record.into_bytes()
    }

    /// A line telling what happened to the port, e.g. `--- Port closed: COM3 ---`,
    /// after the unfinished lines; only in text.
    pub fn event(&mut self, timestamp: i64, event: &str) -> Vec<u8> {
        let RecordFormat::Text { timestamps, .. } = self.format else {
            return Vec::new();
        };
        let mut record = self.finish();
        let mut line = String::new();
        if timestamps {
            line.push_str(&format!("[{}] ", unix_time(timestamp)));
        }
        line.push_str(&format!("{EVENT_MARK} {} {EVENT_MARK}\n", plain(event)));
        record.extend(line.into_bytes());
        record
    }

    fn add(&mut self, direction: Direction, timestamp: i64, bytes: &[u8]) -> Vec<u8> {
        match self.format {
            RecordFormat::Raw => match direction {
                Direction::Received => bytes.to_vec(),
                Direction::Sent => Vec::new(),
            },
            RecordFormat::Text { sent, .. } => {
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
            RecordFormat::Hex { timestamps, sent } => {
                if (direction == Direction::Sent && !sent) || bytes.is_empty() {
                    return Vec::new();
                }
                let mut line = String::new();
                if timestamps {
                    line.push_str(&unix_time(timestamp));
                    line.push('\t');
                }
                if sent {
                    line.push_str(match direction {
                        Direction::Received => "RX\t",
                        Direction::Sent => "TX\t",
                    });
                }
                line.push_str(&hex(bytes));
                line.push('\n');
                line.into_bytes()
            }
        }
    }

    fn text_line(&self, direction: Direction, start: i64, line: &str) -> String {
        let RecordFormat::Text { timestamps, sent } = self.format else {
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

    impl RecordSink for Recorded {
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
    fn recorder_writes_the_gathered_chunks_at_once() {
        let sink = Rc::new(Recorded::default());
        let mut recorder = Recorder::new(
            RecordFormat::Text {
                timestamps: false,
                sent: true,
            },
            sink.clone(),
        );
        recorder.received(&[
            ByteData::new(1, b"temp:20".to_vec()),
            ByteData::new(2, b".5\nvolt:3.3\n".to_vec()),
        ]);
        assert_eq!(sink.writes.borrow().len(), 1);
        assert_eq!(sink.text(), "< temp:20.5\n< volt:3.3\n");

        // Nothing to write: no write
        recorder.received(&[ByteData::new(3, b"led".to_vec())]);
        recorder.sent(4, b"l");
        assert_eq!(sink.writes.borrow().len(), 1);
    }

    #[test]
    fn recorder_closes_after_the_unfinished_lines() {
        let sink = Rc::new(Recorded::default());
        let mut recorder = Recorder::new(
            RecordFormat::Text {
                timestamps: false,
                sent: false,
            },
            sink.clone(),
        );
        recorder.received(&[ByteData::new(1, b"login: ".to_vec())]);
        assert!(!*sink.closed.borrow());
        drop(recorder.close());
        assert_eq!(sink.text(), "login: \n");
        assert!(*sink.closed.borrow());
    }

    fn serializer(format: RecordFormat) -> RecordSerializer {
        RecordSerializer::new(format)
    }

    fn text(record: Vec<u8>) -> String {
        String::from_utf8(record).unwrap()
    }

    const TEXT: RecordFormat = RecordFormat::Text {
        timestamps: true,
        sent: false,
    };

    fn hex(timestamps: bool, sent: bool) -> RecordFormat {
        RecordFormat::Hex { timestamps, sent }
    }

    #[test]
    fn times_are_unix_time_in_seconds_to_the_millisecond() {
        assert_eq!(unix_time(1_791_409_892_542), "1791409892.542");
        assert_eq!(unix_time(1_791_409_892_005), "1791409892.005");
        assert_eq!(unix_time(0), "0.000");
    }

    #[test]
    fn raw_keeps_the_received_bytes_as_they_are() {
        let mut record = serializer(RecordFormat::Raw);
        assert_eq!(
            record.received(1, b"\x1b[1mok\r\n\xff"),
            b"\x1b[1mok\r\n\xff"
        );
        assert_eq!(record.sent(2, b"led on\r"), b"");
        assert_eq!(record.finish(), b"");
    }

    #[test]
    fn text_lines_take_the_time_their_first_byte_came() {
        let mut record = serializer(TEXT);
        assert_eq!(text(record.received(1, b"temp:2")), "");
        assert_eq!(
            text(record.received(2, b"0.5\nvolt:3.3\nle")),
            "[0.001] temp:20.5\n[0.002] volt:3.3\n"
        );
        assert_eq!(text(record.received(3, b"d: on\n")), "[0.002] led: on\n");
    }

    #[test]
    fn text_tells_events_after_the_unfinished_lines() {
        let mut record = serializer(RecordFormat::Text {
            timestamps: true,
            sent: true,
        });
        assert_eq!(text(record.received(1, b"ok\nhal")), "[0.001] < ok\n");
        assert_eq!(
            text(record.event(2, "Port closed: COM3")),
            "[0.001] < hal\n[0.002] --- Port closed: COM3 ---\n"
        );
        assert_eq!(text(record.received(3, b"f\n")), "[0.003] < f\n");

        let mut record = serializer(RecordFormat::Text {
            timestamps: false,
            sent: false,
        });
        assert_eq!(
            text(record.event(1, "Connection lost: COM3\n")),
            "--- Connection lost: COM3 ---\n"
        );
        for format in [RecordFormat::Raw, hex(true, true)] {
            assert_eq!(serializer(format).event(1, "Port closed"), b"");
        }
    }

    #[test]
    fn text_ends_lines_at_cr_lf_or_crlf() {
        let mut record = serializer(TEXT);
        assert_eq!(
            text(record.received(1, b"a\r\nb\rc\nd")),
            "[0.001] a\n[0.001] b\n[0.001] c\n"
        );
        // A last CR ends its line at once, and the LF after it is not another
        assert_eq!(text(record.received(2, b"\r")), "[0.001] d\n");
        assert_eq!(text(record.received(3, b"\ne\n")), "[0.003] e\n");
    }

    #[test]
    fn text_drops_escape_sequences_and_control_characters_but_tabs() {
        let mut record = serializer(TEXT);
        assert_eq!(
            text(record.received(1, b"\x1b[32mOK\x1b[0m\x07\tdone\x1b[")),
            ""
        );
        assert_eq!(text(record.received(2, b"K\n")), "[0.001] OK\tdone\n");
    }

    #[test]
    fn text_joins_a_character_cut_between_chunks() {
        let mut record = serializer(TEXT);
        let bytes = "温度\n".as_bytes();
        assert_eq!(text(record.received(1, &bytes[..4])), "");
        assert_eq!(text(record.received(2, &bytes[4..])), "[0.001] 温度\n");
    }

    #[test]
    fn text_finish_gives_the_unfinished_lines() {
        let mut record = serializer(RecordFormat::Text {
            timestamps: false,
            sent: true,
        });
        assert_eq!(text(record.received(1, b"login: ")), "");
        assert_eq!(text(record.sent(2, b"roo")), "");
        assert_eq!(text(record.finish()), "< login: \n> roo\n");
        assert_eq!(record.finish(), b"");
    }

    #[test]
    fn text_marks_sent_lines_typed_key_by_key() {
        let mut record = serializer(RecordFormat::Text {
            timestamps: true,
            sent: true,
        });
        for (time, key) in [(1, "l"), (2, "s"), (3, "\r")] {
            let line = text(record.sent(time, key.as_bytes()));
            assert_eq!(line, if key == "\r" { "[0.001] > ls\n" } else { "" });
        }
        assert_eq!(text(record.received(4, b"a.txt\r\n")), "[0.004] < a.txt\n");
    }

    #[test]
    fn text_leaves_out_what_is_sent_unless_asked() {
        let mut record = serializer(TEXT);
        assert_eq!(record.sent(1, b"ls\r"), b"");
        assert_eq!(record.finish(), b"");
    }

    #[test]
    fn hex_gives_a_line_for_each_chunk() {
        let mut record = serializer(hex(true, true));
        assert_eq!(
            text(record.received(1, b"ok\r\n")),
            "0.001\tRX\t6F 6B 0D 0A\n"
        );
        assert_eq!(text(record.sent(2, &[0x02, 0xff])), "0.002\tTX\t02 FF\n");
        assert_eq!(record.received(3, b""), b"");

        let mut record = serializer(hex(true, false));
        assert_eq!(text(record.received(1, b"ok")), "0.001\t6F 6B\n");
        assert_eq!(record.sent(2, b"x"), b"");

        let mut record = serializer(hex(false, false));
        assert_eq!(text(record.received(1, b"ok")), "6F 6B\n");
    }
}
