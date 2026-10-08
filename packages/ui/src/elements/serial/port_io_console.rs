use std::{cell::RefCell, rc::Rc};

use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::{
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger},
        input::Input,
    },
    data::{ByteData, DataContext, NewEntries, RAW_BYTES_LABEL, SourceCursor},
    serial::SerialContext,
};

use super::send_format::SendFormat;

const PORT_IO_CONSOLE_CSS: Asset = asset!("/assets/styling/port-io-console.css");

const CONSOLE_JS: &str = concat!(
    include_str!("console_output.js"),
    include_str!("port_io_console.js"),
);

#[component]
pub fn PortIoConsole() -> Element {
    let serial = use_context::<SerialContext>();
    let data_context = use_context::<DataContext>();

    let mut text_to_send = use_signal(String::new);
    let mut format = use_signal(|| SendFormat::Text);
    let bytes = use_memo(move || format().parse(&text_to_send()));
    let can_send = serial.is_open() && !text_to_send().trim().is_empty() && bytes.read().is_ok();
    // Owns the output text, so a chunk costs the same however long it is
    let output = use_hook(|| document::eval(CONSOLE_JS));

    // In a hook: the effect takes each render's closure, and a fresh cursor
    // would redraw the output from the start
    let cursor = use_hook(|| Rc::new(RefCell::new(SourceCursor::default())));
    use_effect(move || {
        let mut cursor = cursor.borrow_mut();
        let Some(NewEntries {
            entries, restarted, ..
        }) = cursor.new_entries::<ByteData>(&data_context, RAW_BYTES_LABEL)
        else {
            cursor.reset();
            let _ = output.send((true, String::new()));
            return;
        };
        if restarted || !entries.is_empty() {
            let _ = output.send((restarted, render_raw_bytes_text(&entries)));
        }
    });

    use_drop(move || {
        let _ = output.send(());
    });

    let mut send_text = move || {
        if !serial.is_open() || text_to_send().trim().is_empty() {
            return;
        }
        let Ok(bytes) = bytes() else {
            return;
        };
        serial.send(bytes);
        *text_to_send.write() = String::new();
    };

    rsx! {
        document::Link { rel: "stylesheet", href: PORT_IO_CONSOLE_CSS }

        div {
            class: "console",
            pre {
                "data-port-io-console": true,
                class: "console-output",
            }
            div {
                class: "console-send",
                DropdownMenu {
                    class: "console-format",
                    DropdownMenuTrigger {
                        class: "console-format-trigger",
                        aria_label: "Send format",
                        lucide::ChevronUp {}
                        "{format().name()}"
                    }
                    DropdownMenuContent {
                        class: "console-format-menu",
                        for (index, option) in SendFormat::ALL.into_iter().enumerate() {
                            DropdownMenuItem {
                                value: option,
                                index,
                                on_select: move |option| format.set(option),
                                "{option.name()}"
                            }
                        }
                    }
                }
                label {
                    class: "field console-send-field",
                    "data-invalid": bytes.read().is_err(),
                    if let Some(prefix) = format().prefix() {
                        span { class: "field-label console-send-prefix", "{prefix}" }
                    }
                    Input {
                    type: "text",
                    placeholder: format().placeholder(),
                    aria_invalid: bytes.read().is_err(),
                    title: bytes.read().as_ref().err().cloned().unwrap_or_default(),
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
                }
                Button {
                    variant: ButtonVariant::Primary,
                    size: ButtonSize::Sm,
                    class: "console-send-button",
                    disabled: !can_send,
                    onclick: move |_| {
                        send_text();
                    },
                    lucide::Send {}
                }
            }
        }
    }
}

fn render_raw_bytes_text(raw_bytes: &[ByteData]) -> String {
    raw_bytes
        .iter()
        .map(|data| String::from_utf8_lossy(data.value()).into_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::ByteData;

    #[test]
    fn render_raw_bytes_text_does_not_repeat_old_chunks() {
        let raw_bytes = vec![
            ByteData::new(1, b"led: on\n".to_vec()),
            ByteData::new(2, b"led: off\n".to_vec()),
        ];

        let rendered = render_raw_bytes_text(&raw_bytes);
        assert_eq!(rendered, "led: on\nled: off\n");
        assert!(!rendered.contains("led: onled: on"));
    }
}
