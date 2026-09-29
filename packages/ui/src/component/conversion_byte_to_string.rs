use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{LdMoveRight, LdTag},
    Icon,
};

use crate::{
    data::{DataContext, StringData},
    time::TimeContext,
};

#[component]
pub fn ConversionByteToString(
    #[props(default)] initial_source_label: String,
    #[props(default)] initial_target_label: String,
    #[props(default = "\\n".to_string())] initial_delimiter: String,
    enabled: ReadSignal<bool>,
) -> Element {
    let mut data_context = use_context::<DataContext>();
    let time_context = use_context::<TimeContext>();
    let mut source_label = use_signal(|| initial_source_label);
    let mut target_label = use_signal(|| initial_target_label);
    let mut delimiter = use_signal(|| initial_delimiter);
    let mut buffer = use_signal(String::new);
    let mut processed_count = use_signal(|| 0usize);

    use_effect(move || {
        if !enabled() {
            return;
        }

        let source_label = source_label();
        let target_label = target_label();
        if source_label.trim().is_empty()
            || target_label.trim().is_empty()
            || source_label.trim() == target_label.trim()
        {
            return;
        }

        let Some(source_queue) = data_context.get_bytes(&source_label) else {
            processed_count.set(0);
            buffer.set(String::new());
            return;
        };

        let processed = processed_count();
        let start = if processed > source_queue.len() {
            processed_count.set(0);
            buffer.set(String::new());
            0
        } else {
            processed
        };
        let new_entries = source_queue.iter().skip(start).cloned().collect::<Vec<_>>();

        if new_entries.is_empty() {
            return;
        }

        let delim = decode_delimiter(&delimiter());
        let (pieces, next_buffer) = split_pending_bytes(&new_entries, &buffer(), &delim);
        buffer.set(next_buffer);
        processed_count.set(source_queue.len());

        if pieces.is_empty() {
            return;
        }

        let timestamp = time_context.current();
        for value in pieces {
            data_context.push_string(&target_label, StringData::new(timestamp, value));
        }
    });

    rsx! {
        div {
            class: "space-y-2",
            div {
                class: "flex items-center gap-2",
                div {
                    class: "join join-vertical",
                    label {
                        class: "input input-sm input-bordered",
                        Icon { icon: LdTag {} }
                        input {
                            class: "grow",
                            list: "conversion-bytes-labels", // Defined in `ConversionList` component
                            placeholder: "Source label",
                            autocomplete: "on",
                            value: "{source_label()}",
                            oninput: move |event| {
                                source_label.set(event.value());
                            },
                        }
                    }
                    label {
                        class: "input input-sm input-bordered",
                        span { class: "label", "Delimiter" }
                        input {
                            class: "grow",
                            value: "{delimiter()}",
                            oninput: move |event| {
                                delimiter.set(event.value());
                            },
                        }
                    }
                }
                Icon { icon: LdMoveRight {} }
                label {
                    class: "input input-sm input-bordered",
                    Icon { icon: LdTag {} }
                    input {
                        class: "grow",
                        placeholder: "Target label",
                        value: "{target_label()}",
                        oninput: move |event| {
                            target_label.set(event.value());
                        },
                    }
                }
            }
        }
    }
}

fn decode_delimiter(value: &str) -> String {
    value
        .replace("\\n", "\n")
        .replace("\\r", "\r")
        .replace("\\t", "\t")
        .replace("\\\\", "\\")
}

fn split_pending_bytes(
    entries: &[crate::data::ByteData],
    current: &str,
    delimiter: &str,
) -> (Vec<String>, String) {
    let mut buffer = current.to_string();
    let mut values = Vec::new();

    for entry in entries {
        buffer.push_str(&String::from_utf8_lossy(entry.value()).into_owned());

        let mut parts = buffer
            .split(delimiter)
            .map(str::to_string)
            .collect::<Vec<_>>();

        if buffer.ends_with(delimiter) {
            buffer.clear();
            parts.pop();
            values.extend(parts);
        } else {
            let trailing = parts.pop().unwrap_or_default();
            buffer = trailing;
            values.extend(parts);
        }
    }

    (values, buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_delimiter_supports_escape_sequences() {
        assert_eq!(decode_delimiter("\\n"), "\n");
        assert_eq!(decode_delimiter("\\r\\n"), "\r\n");
    }

    #[test]
    fn split_pending_bytes_keeps_all_same_timestamp_chunks() {
        let entries = vec![
            crate::data::ByteData::new(123, b"led:".to_vec()),
            crate::data::ByteData::new(123, b" on\nled:".to_vec()),
            crate::data::ByteData::new(123, b" off\n".to_vec()),
        ];

        let (values, remaining) = split_pending_bytes(&entries, "", "\n");
        assert_eq!(values, vec!["led: on", "led: off"]);
        assert_eq!(remaining, "");
    }
}
