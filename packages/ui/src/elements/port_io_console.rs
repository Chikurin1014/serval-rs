use dioxus::prelude::*;
use dioxus_free_icons::{icons::ld_icons::LdSend, Icon};

use crate::{
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        input::Input,
    },
    data::{ByteData, DataContext},
    serial::SerialContext,
};

const PORT_IO_CONSOLE_CSS: Asset = asset!("/assets/styling/port-io-console.css");

#[component]
pub fn PortIoConsole() -> Element {
    let SerialContext {
        is_open, tx_send, ..
    } = use_context::<SerialContext>();
    let data_context = use_context::<DataContext>();

    let mut text_to_show = use_signal(String::new);
    let mut text_to_send = use_signal(String::new);
    let mut scroll_to_bottom = use_signal(|| false);

    use_effect(move || {
        let Some(raw_data) = data_context.raw_data() else {
            text_to_show.set(String::new());
            return;
        };

        text_to_show.set(render_raw_data_text(&raw_data));
        scroll_to_bottom.set(true);
    });

    use_effect(move || {
        if !scroll_to_bottom() {
            return;
        }

        let _ = dioxus::document::eval(
            r#"
                const console = document.querySelector('[data-port-io-console]');
                if (console) {
                    console.scrollTop = console.scrollHeight;
                }
            "#,
        );
        scroll_to_bottom.set(false);
    });

    let mut send_text = move || {
        if !is_open() || text_to_send().trim().is_empty() {
            return;
        }
        if let Some(send_action) = tx_send() {
            send_action(text_to_send().as_bytes().to_vec());
        }
        *text_to_send.write() = String::new();
    };

    rsx! {
        document::Link { rel: "stylesheet", href: PORT_IO_CONSOLE_CSS }

        div {
            class: "console",
            pre {
                "data-port-io-console": true,
                class: "console-output",
                "{text_to_show()}"
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
                    disabled: !is_open() || text_to_send().trim().is_empty(),
                    onclick: move |_| {
                        send_text();
                    },
                    Icon {
                        icon: LdSend {}
                    }
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
