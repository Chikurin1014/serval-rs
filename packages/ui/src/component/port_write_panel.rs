use dioxus::prelude::*;

use crate::serial::SerialContext;

#[component]
pub fn PortWritePanel() -> Element {
    let SerialContext {
        is_open, tx_send, ..
    } = use_context::<SerialContext>();
    let mut tx_text = use_signal(String::new);

    rsx! {
        div {
            class: "card bg-base-100 shadow-xl",
            div {
                class: "card-body gap-4",
                h2 { class: "card-title", "Send Data" }
                textarea {
                    class: "textarea textarea-bordered min-h-36 w-full",
                    placeholder: "Type text to send to the active port",
                    value: "{tx_text()}",
                    oninput: move |event| {
                        *tx_text.write() = event.value();
                    }
                }
                div {
                    class: "flex flex-wrap gap-3",
                    button {
                        class: "btn btn-primary",
                        disabled: !is_open() || tx_text().trim().is_empty(),
                        onclick: move |_| {
                            if !is_open() || tx_text().trim().is_empty() {
                                return;
                            }

                            if let Some(action) = tx_send() {
                                action(tx_text().as_bytes().to_vec());
                            }

                            *tx_text.write() = String::new();
                        },
                        "Send"
                    }
                    button {
                        class: "btn btn-ghost",
                        onclick: move |_| {
                            *tx_text.write() = String::new();
                        },
                        "Clear"
                    }
                }
            }
        }
    }
}
