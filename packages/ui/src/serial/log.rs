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

/// What a log file in `format` is named with.
pub fn extension(format: LogFormat) -> &'static str {
    match format {
        LogFormat::Raw => "bin",
        LogFormat::Text { .. } => "log",
        LogFormat::Hex { .. } => "tsv",
    }
}

/// How much of an existing log's end is read to tell its format.
pub const TAIL_BYTES: usize = 64 * 1024;

/// A file picked for a log.
pub struct LogFile {
    pub sink: Rc<dyn LogSink>,
    pub name: String,
    /// The end of what an existing file holds, at most [`TAIL_BYTES`].
    pub tail: Vec<u8>,
}

impl LogFormat {
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

/// The format an existing log was written in, from its name and the `tail`
/// of it: its last lines, or its extension while it has none.
pub fn detect_format(name: &str, tail: &[u8]) -> LogFormat {
    let by_extension = match name.rsplit_once('.').map(|(_, extension)| extension) {
        Some("bin") => return LogFormat::Raw,
        Some("tsv") => LogFormat::Hex {
            timestamps: false,
            sent: false,
        },
        _ => TEXT,
    };
    // The tail may start inside a character, and a line
    let start = tail
        .iter()
        .take(3)
        .take_while(|&&byte| byte & 0xC0 == 0x80)
        .count();
    let Ok(text) = std::str::from_utf8(&tail[start..]) else {
        return LogFormat::Raw;
    };
    // None but tabs and line ends are left in text, nor anything but LFs
    if text
        .chars()
        .any(|c| c.is_control() && c != '\t' && c != '\n')
    {
        return LogFormat::Raw;
    }
    let lines = text
        .split_terminator('\n')
        .skip(usize::from(tail.len() >= TAIL_BYTES))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return by_extension;
    }
    // Hex lines, all with the same fields; bare ones only in a hex file
    let first = hex_fields(lines[0]);
    if let Some((timestamps, sent)) = first {
        let alike = lines.iter().all(|line| hex_fields(line) == first);
        if alike && (timestamps || sent || by_extension != TEXT) {
            return LogFormat::Hex { timestamps, sent };
        }
    }
    let timestamps = lines.iter().all(|line| after_time(line).is_some());
    let marked = |line: &&str| {
        let rest = if timestamps {
            after_time(line).unwrap_or(line)
        } else {
            line
        };
        rest.starts_with("< ") || rest.starts_with("> ")
    };
    LogFormat::Text {
        timestamps,
        sent: lines.iter().all(marked),
    }
}

/// Whether a `Hex` line (`[time⇥][RX|TX⇥]hex`) has its time and direction.
fn hex_fields(line: &str) -> Option<(bool, bool)> {
    let mut fields = line.split('\t').collect::<Vec<_>>();
    let hex = fields.pop()?;
    let bytes = hex
        .split(' ')
        .all(|byte| byte.len() == 2 && byte.chars().all(|c| c.is_ascii_hexdigit()));
    let sent = fields
        .last()
        .is_some_and(|&field| matches!(field, "RX" | "TX"));
    if sent {
        fields.pop();
    }
    let timestamps = match fields[..] {
        [] => false,
        [time] if is_unix_time(time) => true,
        _ => return None,
    };
    bytes.then_some((timestamps, sent))
}

const TEXT: LogFormat = LogFormat::Text {
    timestamps: false,
    sent: false,
};

/// The rest of a line after its `[time] `.
fn after_time(line: &str) -> Option<&str> {
    let (time, rest) = line.strip_prefix('[')?.split_once("] ")?;
    is_unix_time(time).then_some(rest)
}

/// As [`unix_time`] writes it.
fn is_unix_time(text: &str) -> bool {
    text.split_once('.').is_some_and(|(seconds, millis)| {
        !seconds.is_empty()
            && seconds.chars().all(|c| c.is_ascii_digit())
            && millis.len() == 3
            && millis.chars().all(|c| c.is_ascii_digit())
    })
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
    /// A line for each chunk: its bytes in hex, after its time (with
    /// `timestamps`) and `RX` or `TX` (with `sent`), tab separated.
    Hex { timestamps: bool, sent: bool },
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
            LogFormat::Hex { timestamps, sent } => {
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
                let hex = bytes
                    .iter()
                    .map(|byte| format!("{byte:02X}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                line.push_str(&hex);
                line.push('\n');
                line.into_bytes()
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
    fn detect_format_reads_what_each_format_writes() {
        let cases = [
            ("a.log", "temp:20.5\nled: on\n", TEXT_PLAIN),
            (
                "a.log",
                "[1791409892.542] temp:20.5\n[1791409892.600] ok\n",
                LogFormat::Text {
                    timestamps: true,
                    sent: false,
                },
            ),
            (
                "a.log",
                "[1791409892.542] > ls\n[1791409892.600] < a.txt\n",
                LogFormat::Text {
                    timestamps: true,
                    sent: true,
                },
            ),
            ("a.tsv", "6F 6B\n0D\n", hex(false, false)),
            ("a.tsv", "1791409892.542\t6F 6B\n", hex(true, false)),
            (
                "a.tsv",
                "1791409892.542\tRX\t6F 6B\n1791409892.600\tTX\t0D\n",
                hex(true, true),
            ),
            // Bare hex in a text file is text
            ("a.log", "6F 6B\n", TEXT_PLAIN),
            // Not all alike: text
            ("a.tsv", "1791409892.542\t6F\n6F\n", TEXT_PLAIN),
            // Text keeps no CR nor escape sequences: written raw
            ("a.log", "ok\r\n\x1b[0m", LogFormat::Raw),
        ];
        for (name, tail, format) in cases {
            assert_eq!(detect_format(name, tail.as_bytes()), format, "{tail:?}");
        }
        assert_eq!(detect_format("a.log", b"\xff\xfe"), LogFormat::Raw);
    }

    #[test]
    fn detect_format_goes_by_the_extension_without_lines() {
        assert_eq!(detect_format("a.bin", b"ok\n"), LogFormat::Raw);
        assert_eq!(detect_format("a.tsv", b""), hex(false, false));
        assert_eq!(detect_format("a.log", b""), TEXT_PLAIN);
        assert_eq!(detect_format("notes", b""), TEXT_PLAIN);
    }

    #[test]
    fn detect_format_skips_a_cut_first_line_and_character() {
        let mut tail = "温".as_bytes()[1..].to_vec();
        tail.extend_from_slice("度 cut\n".as_bytes());
        tail.extend(std::iter::repeat_n(b'x', TAIL_BYTES));
        tail.extend_from_slice(b"\n[1791409892.542] ok\n");
        let tail = &tail[tail.len() - TAIL_BYTES..];
        assert_eq!(
            detect_format("a.log", tail),
            LogFormat::Text {
                timestamps: true,
                sent: false,
            }
        );
        assert_eq!(detect_format("a.log", &"温".as_bytes()[1..]), TEXT_PLAIN);
    }

    const TEXT_PLAIN: LogFormat = LogFormat::Text {
        timestamps: false,
        sent: false,
    };

    fn hex(timestamps: bool, sent: bool) -> LogFormat {
        LogFormat::Hex { timestamps, sent }
    }

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
        let mut log = serializer(hex(true, true));
        assert_eq!(text(log.received(1, b"ok\r\n")), "0.001\tRX\t6F 6B 0D 0A\n");
        assert_eq!(text(log.sent(2, &[0x02, 0xff])), "0.002\tTX\t02 FF\n");
        assert_eq!(log.received(3, b""), b"");

        let mut log = serializer(hex(true, false));
        assert_eq!(text(log.received(1, b"ok")), "0.001\t6F 6B\n");
        assert_eq!(log.sent(2, b"x"), b"");

        let mut log = serializer(hex(false, false));
        assert_eq!(text(log.received(1, b"ok")), "6F 6B\n");
    }
}
