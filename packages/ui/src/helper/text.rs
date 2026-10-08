/// `value` with its `\n`, `\r`, `\t` and `\\` escapes turned into characters.
pub(crate) fn unescape(value: &str) -> String {
    value
        .replace("\\n", "\n")
        .replace("\\r", "\r")
        .replace("\\t", "\t")
        .replace("\\\\", "\\")
}

/// `text` escaped to one line: the reverse of [`unescape`].
pub(crate) fn single_line(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

/// `bytes` with thousands separated, e.g. "12,345 B".
pub(crate) fn format_bytes(bytes: u64) -> String {
    let digits = bytes.to_string();
    let mut grouped = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    format!("{grouped} B")
}

/// `field` quoted as RFC 4180 needs.
pub(crate) fn csv_field(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

/// Decodes `pending` then `bytes` as UTF-8, leaving a cut-off character in `pending`.
pub(crate) fn decode_utf8(pending: &mut Vec<u8>, bytes: &[u8]) -> String {
    pending.extend_from_slice(bytes);
    let mut text = String::new();
    let mut rest = pending.as_slice();
    while !rest.is_empty() {
        match std::str::from_utf8(rest) {
            Ok(valid) => {
                text.push_str(valid);
                rest = &[];
            }
            Err(error) => {
                let (valid, after) = rest.split_at(error.valid_up_to());
                text.push_str(std::str::from_utf8(valid).expect("checked up to here"));
                match error.error_len() {
                    Some(invalid) => {
                        text.push(char::REPLACEMENT_CHARACTER);
                        rest = &after[invalid..];
                    }
                    None => {
                        rest = after;
                        break;
                    }
                }
            }
        }
    }
    *pending = rest.to_vec();
    text
}

#[cfg(test)]
mod tests {
    use super::{csv_field, decode_utf8, format_bytes, single_line, unescape};

    #[test]
    fn unescape_supports_escape_sequences() {
        assert_eq!(unescape("\\n"), "\n");
        assert_eq!(unescape("\\r\\n"), "\r\n");
        assert_eq!(unescape(""), "");
    }

    #[test]
    fn single_line_escapes_line_breaks() {
        assert_eq!(single_line("led: on\r\n"), "led: on\\r\\n");
        assert_eq!(single_line("a\tb\\"), "a\\tb\\\\");
    }

    #[test]
    fn format_bytes_separates_thousands() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(999), "999 B");
        assert_eq!(format_bytes(1_000), "1,000 B");
        assert_eq!(format_bytes(1_234_567), "1,234,567 B");
    }

    #[test]
    fn csv_field_quotes_what_needs_it() {
        assert_eq!(csv_field("20.5"), "20.5");
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_field("temp:1\n"), "\"temp:1\n\"");
    }

    #[test]
    fn decode_utf8_joins_a_character_cut_between_entries() {
        let bytes = "温度".as_bytes();
        let mut pending = Vec::new();
        assert_eq!(decode_utf8(&mut pending, &bytes[..4]), "温");
        assert_eq!(pending, bytes[3..4]);
        assert_eq!(decode_utf8(&mut pending, &bytes[4..]), "度");
        assert!(pending.is_empty());
    }

    #[test]
    fn decode_utf8_replaces_invalid_bytes() {
        let mut pending = Vec::new();
        assert_eq!(decode_utf8(&mut pending, b"a\xffb"), "a\u{FFFD}b");
        assert!(pending.is_empty());
    }
}
