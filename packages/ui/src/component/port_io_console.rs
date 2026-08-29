use dioxus::prelude::*;
use dioxus_free_icons::{icons::ld_icons::LdSend, Icon};

use crate::data::DataContext;
use crate::serial::SerialContext;

#[component]
pub fn PortIoConsole() -> Element {
    let SerialContext {
        is_open, tx_send, ..
    } = use_context::<SerialContext>();
    let data_context = use_context::<DataContext>();

    let mut last_timestamp = use_signal(|| 0i64);
    let mut text_to_show = use_signal(String::new);
    let mut text_to_send = use_signal(String::new);

    use_effect(move || {
        let Some(raw_data) = data_context.raw_data() else {
            return;
        };
        if raw_data.is_empty() {
            return;
        }

        let next_text = raw_data
            .iter()
            .filter_map(|data| {
                // Read last timestamp without consuming the signal
                // Consuming the signal would cause the effect to re-run and an infinite loop
                if *last_timestamp.peek() < data.timestamp() {
                    Some(String::from_utf8_lossy(data.value()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");
        text_to_show.write().push_str(next_text.as_str());

        last_timestamp.set(raw_data.last().unwrap().timestamp());
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
        div {
            class: "join join-vertical",
            height: "20rem",
            width: "50rem",

            div {
                class: "mockup-code overflow-auto text-sm join-item h-full w-full",
                {
                    text_to_show().lines().map(|line| rsx! {
                        pre {
                            code { "{line}" }
                        }
                    })
                }
            }
            div {
                class: "join",
                input {
                    type: "text",
                    placeholder: "Type text to send to the active port",
                    class: "input input-sm w-full",
                    value: "{text_to_send()}",
                    oninput: move |event| {
                        *text_to_send.write() = event.value();
                    },
                    onkeydown: move |event| {
                        if event.key() == Key::Enter {
                            send_text();
                        }
                    }
                }
                button {
                    class: "btn btn-success btn-sm join-item",
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
