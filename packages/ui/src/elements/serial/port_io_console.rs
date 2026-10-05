use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::{
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        input::Input,
    },
    data::{ByteData, DataContext, NewEntries, RAW_DATA_LABEL, SourceCursor},
    serial::SerialContext,
};

const PORT_IO_CONSOLE_CSS: Asset = asset!("/assets/styling/port-io-console.css");

/// The text handling, then the output, which uses it (see both files)
const CONSOLE_JS: &str = concat!(
    include_str!("console_output.js"),
    include_str!("port_io_console.js"),
);

#[component]
pub fn PortIoConsole() -> Element {
    let serial = use_context::<SerialContext>();
    let data_context = use_context::<DataContext>();

    let mut text_to_send = use_signal(String::new);
    // Owns the output text, so each chunk costs the same however long it is
    let output = use_hook(|| document::eval(CONSOLE_JS));

    // Sends only the chunks received since the last run
    let mut cursor = SourceCursor::default();
    use_effect(move || {
        let Some(NewEntries {
            entries, restarted, ..
        }) = cursor.new_entries::<ByteData>(&data_context, RAW_DATA_LABEL)
        else {
            cursor.reset();
            let _ = output.send((true, String::new()));
            return;
        };
        if restarted || !entries.is_empty() {
            let _ = output.send((restarted, render_raw_data_text(&entries)));
        }
    });

    use_drop(move || {
        let _ = output.send(());
    });

    let mut send_text = move || {
        if !serial.is_open() || text_to_send().trim().is_empty() {
            return;
        }
        serial.send(text_to_send().into_bytes());
        *text_to_send.write() = String::new();
    };

    rsx! {
        document::Link { rel: "stylesheet", href: PORT_IO_CONSOLE_CSS }

        div {
            class: "console",
            pre {
                "data-port-io-console": true,
                class: "console-output",
                // Filled by `port_io_console.js`
            }
            div {
                class: "console-send",
                Input {
                    type: "text",
                    placeholder: "Type text to send to the active port",
                    value: "{text_to_send()}",
                    oninput: move |event: FormEvent| {
                        *text_to_send.write() = event.value();
                    },
                    onkeydown: move |event: KeyboardEvent| {
                        if event.key() == Key::Enter {
                            send_text();
                        }
                    }
                }
                Button {
                    variant: ButtonVariant::Primary,
                    size: ButtonSize::Sm,
                    background: "var(--secondary-success-color)",
                    disabled: !serial.is_open() || text_to_send().trim().is_empty(),
                    onclick: move |_| {
                        send_text();
                    },
                    lucide::Send {}
                }
            }
        }
    }
}

fn render_raw_data_text(raw_data: &[ByteData]) -> String {
    raw_data
        .iter()
        .map(|data| String::from_utf8_lossy(data.value()).into_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::ByteData;

    #[test]
    fn render_raw_data_text_does_not_repeat_old_chunks() {
        let raw_data = vec![
            ByteData::new(1, b"led: on\n".to_vec()),
            ByteData::new(2, b"led: off\n".to_vec()),
        ];

        let rendered = render_raw_data_text(&raw_data);
        assert_eq!(rendered, "led: on\nled: off\n");
        assert!(!rendered.contains("led: onled: on"));
    }
}
