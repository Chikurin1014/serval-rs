//! Telling the format of an existing record, to go on writing it as it was.

use super::{EVENT_MARK, RecordFormat, TAIL_BYTES};

/// The format an existing record was written in, from its name and the `tail`
/// of it: its last lines, or its extension while it has none.
pub fn detect_format(name: &str, tail: &[u8]) -> RecordFormat {
    let by_extension = match name.rsplit_once('.').map(|(_, extension)| extension) {
        Some("bin") => return RecordFormat::Raw,
        Some("tsv") => RecordFormat::Hex {
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
        return RecordFormat::Raw;
    };
    // None but tabs and line ends are left in text, nor anything but LFs
    if text
        .chars()
        .any(|c| c.is_control() && c != '\t' && c != '\n')
    {
        return RecordFormat::Raw;
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
            return RecordFormat::Hex { timestamps, sent };
        }
    }
    let timestamps = lines.iter().all(|line| after_time(line).is_some());
    // What happened to the port, in either: not lines received or sent
    let traffic = lines
        .iter()
        .map(|line| {
            if timestamps {
                after_time(line).unwrap_or(line)
            } else {
                line
            }
        })
        .filter(|rest| !is_event(rest))
        .collect::<Vec<_>>();
    let sent = !traffic.is_empty()
        && traffic
            .iter()
            .all(|rest| rest.starts_with("< ") || rest.starts_with("> "));
    RecordFormat::Text { timestamps, sent }
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

const TEXT: RecordFormat = RecordFormat::Text {
    timestamps: false,
    sent: false,
};

/// Whether text, after its time, is an event line: `--- event ---`.
fn is_event(text: &str) -> bool {
    text.starts_with(EVENT_MARK) && text.ends_with(EVENT_MARK) && text.len() > 2 * EVENT_MARK.len()
}

/// The rest of a line after its `[time] `.
fn after_time(line: &str) -> Option<&str> {
    let (time, rest) = line.strip_prefix('[')?.split_once("] ")?;
    is_unix_time(time).then_some(rest)
}

/// As [`super::unix_time`] writes it.
fn is_unix_time(text: &str) -> bool {
    text.split_once('.').is_some_and(|(seconds, millis)| {
        !seconds.is_empty()
            && seconds.chars().all(|c| c.is_ascii_digit())
            && millis.len() == 3
            && millis.chars().all(|c| c.is_ascii_digit())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_format_reads_what_each_format_writes() {
        let cases = [
            ("a.log", "temp:20.5\nled: on\n", TEXT_PLAIN),
            (
                "a.log",
                "[1791409892.542] temp:20.5\n[1791409892.600] ok\n",
                RecordFormat::Text {
                    timestamps: true,
                    sent: false,
                },
            ),
            (
                "a.log",
                "[1791409892.542] > ls\n[1791409892.600] < a.txt\n",
                RecordFormat::Text {
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
            ("a.log", "ok\r\n\x1b[0m", RecordFormat::Raw),
        ];
        for (name, tail, format) in cases {
            assert_eq!(detect_format(name, tail.as_bytes()), format, "{tail:?}");
        }
        assert_eq!(detect_format("a.log", b"\xff\xfe"), RecordFormat::Raw);
    }

    #[test]
    fn detect_format_passes_over_event_lines() {
        let tail = "[1791409892.000] --- Port opened: COM3 at 115200 bps ---\n\
                    [1791409892.542] > ls\n\
                    [1791409892.600] --- Port closed: COM3 ---\n";
        assert_eq!(
            detect_format("a.log", tail.as_bytes()),
            RecordFormat::Text {
                timestamps: true,
                sent: true,
            }
        );
        // Events alone tell nothing of what was sent
        assert_eq!(
            detect_format("a.log", b"--- Port closed: COM3 ---\n"),
            TEXT_PLAIN
        );
    }

    #[test]
    fn detect_format_goes_by_the_extension_without_lines() {
        assert_eq!(detect_format("a.bin", b"ok\n"), RecordFormat::Raw);
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
            RecordFormat::Text {
                timestamps: true,
                sent: false,
            }
        );
        assert_eq!(detect_format("a.log", &"温".as_bytes()[1..]), TEXT_PLAIN);
    }

    const TEXT_PLAIN: RecordFormat = RecordFormat::Text {
        timestamps: false,
        sent: false,
    };

    fn hex(timestamps: bool, sent: bool) -> RecordFormat {
        RecordFormat::Hex { timestamps, sent }
    }
}
