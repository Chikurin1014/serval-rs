use dioxus::prelude::*;
use dioxus_icons::lucide;

use super::field::{ChoicesField, LabelField, TextField};
use super::map_list::labels_list;
use super::{LOG_LABEL, form_of};
use crate::{
    data::{DataType, Decode, DecodeSettings, MapKind, MapPreset, MapRunner, RAW_BYTES_LABEL},
    helper::{Delimiter, single_line},
};

/// A preset reading each line of the raw bytes as a log by `$pattern`, under
/// `$to_label`.
macro_rules! decode_preset {
    ($name:literal, $pattern:expr, $to_label:expr, $level:expr, $message:expr) => {
        MapPreset {
            name: $name,
            detail: $pattern,
            create: || -> Box<dyn MapRunner> {
                Box::new(Decode::with(
                    RAW_BYTES_LABEL,
                    $pattern,
                    $to_label,
                    $level,
                    $message,
                ))
            },
        }
    };
}

/// A line as a log: its level, if it has one in brackets before it, alone or
/// with others (as `[level] message`, or `env_logger`'s), then its message.
pub const LOG_LINE: &str = r"^(?:\[(?:.*? )??({level})(?: .*?)?\] )?(.*)$";

pub const DECODE: MapKind = MapKind {
    name: "Decode",
    from: &[DataType::Bytes],
    to: DataType::Log,
    create: || Box::new(Decode::new("", "")),
    form: |settings| {
        form_of(
            settings,
            |settings: DecodeSettings| rsx! { DecodeForm { settings } },
        )
    },
    presets: &[decode_preset!(
        "[... level ...] message",
        LOG_LINE,
        LOG_LABEL,
        "$1",
        "$2"
    )],
};

/// Its lines as a Match map to a log takes them: by a pattern, into a level
/// (nothing for none) and a message.
#[component]
pub fn DecodeForm(settings: DecodeSettings) -> Element {
    let DecodeSettings {
        from_label,
        mut delimiters,
        pattern,
        to_label,
        level,
        message,
        pattern_error,
        replacement_error,
    } = settings;
    let chosen = delimiters
        .read()
        .iter()
        .filter_map(|delimiter| Delimiter::ALL.iter().position(|all| all == delimiter))
        .collect::<Vec<_>>();

    rsx! {
        div {
            class: "map-row",
            div {
                class: "field-stack",
                LabelField {
                    value: from_label,
                    list: labels_list(DataType::Bytes),
                    placeholder: "Input label",
                }
                ChoicesField {
                    name: "Delimiter",
                    options: Delimiter::ALL.map(|delimiter| single_line(delimiter.text())).to_vec(),
                    chosen,
                    on_change: move |chosen: Vec<usize>| {
                        delimiters.set(chosen.into_iter().map(|index| Delimiter::ALL[index]).collect());
                    },
                }
                TextField {
                    name: "From",
                    icon: rsx! { lucide::Regex {} },
                    value: pattern,
                    placeholder: "Text to be matched",
                    error: pattern_error,
                }
                if let Some(error) = pattern_error() {
                    span { class: "field-error", "{error}" }
                }
            }
            lucide::MoveRight { size: 20 }
            div {
                class: "field-stack",
                LabelField {
                    value: to_label,
                    list: labels_list(DataType::Log),
                    placeholder: "Output label",
                }
                TextField {
                    name: "Level",
                    value: level,
                    placeholder: "None",
                    error: replacement_error,
                }
                TextField { name: "Message", value: message, error: replacement_error }
                if let Some(error) = replacement_error() {
                    span { class: "field-error", "{error}" }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use dioxus::prelude::*;

    use crate::{
        data::{ByteData, Decode, DecodeSettings, compile_pattern, testing::Maps},
        helper::Delimiter,
    };

    use super::{DECODE, LOG_LINE};

    #[test]
    fn its_preset_is_as_the_initial_decode() {
        assert_eq!(DECODE.presets.len(), 1);
        assert_eq!(DECODE.presets[0].detail, LOG_LINE);
    }

    #[test]
    fn log_line_takes_a_level_if_there_is_one() {
        let log = |input: &str| {
            let captures = compile_pattern(LOG_LINE)
                .unwrap()
                .captures(input)
                .unwrap()?;
            Some((
                captures
                    .get(1)
                    .map_or("", |level| level.as_str())
                    .to_string(),
                captures[2].to_string(),
            ))
        };
        let log_of = |level: &str, message: &str| Some((level.to_string(), message.to_string()));
        assert_eq!(log("[warning] disk full"), log_of("warning", "disk full"));
        assert_eq!(
            log("[2026-10-13T01:23:45Z WARN  app::sensor] low battery"),
            log_of("WARN", "low battery")
        );
        assert_eq!(log("[INFO app] ready"), log_of("INFO", "ready"));
        // The first level-like word in the brackets
        assert_eq!(log("[INFO module info] x"), log_of("INFO", "x"));
        // No level: the line whole
        assert_eq!(log("temp:20.5"), log_of("", "temp:20.5"));
        assert_eq!(log("[1, 2] ready"), log_of("", "[1, 2] ready"));
        assert_eq!(log(""), log_of("", ""));
    }

    fn bytes(bytes: &[u8]) -> impl FnOnce(i64) -> ByteData {
        let bytes = bytes.to_vec();
        move |timestamp| ByteData::new(timestamp, bytes)
    }

    #[test]
    fn lines_are_logs_without_escape_sequences() {
        let mut maps = Maps::new();
        maps.add(
            || Decode::with("raw", LOG_LINE, "log", "$1", "$2"),
            |_: DecodeSettings| {},
        );
        maps.push("raw", bytes(b"\x1b[1;31m-1.5e2\x1b[0m\x1b[K\n[warn"));
        maps.push("raw", bytes(b"ing] disk full\r\n"));
        maps.push(
            "raw",
            bytes(b"[2026-10-13T01:23:45Z INFO  app::sensor] ready\n"),
        );
        assert_eq!(
            maps.values("log"),
            ["-1.5e2", "[WARN] disk full", "[INFO] ready"]
        );
    }

    #[test]
    fn lines_end_at_the_chosen_delimiters_alone() {
        let mut maps = Maps::new();
        maps.add(
            || Decode::new("raw", "line"),
            |mut settings: DecodeSettings| settings.delimiters.set(vec![Delimiter::Cr]),
        );
        maps.push("raw", bytes(b"x1\ry2\r"));
        assert_eq!(maps.values("line"), ["x1", "y2"]);
    }
}
