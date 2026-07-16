use dioxus::prelude::*;

use super::serial::SerialContext;

#[component]
pub fn ConnectionStatus() -> Element {
    let SerialContext { status_message, .. } = use_context::<SerialContext>();

    rsx! {
        div {
            class: "alert alert-info",
            span { class: "font-medium", "Status:" }
            span { "{status_message}" }
        }
    }
}

#[component]
pub fn TextMonitor() -> Element {
    let SerialContext {
        mut received_text,
        mut status_message,
        ..
    } = use_context::<SerialContext>();

    rsx! {
        div {
            class: "card bg-base-100 shadow-xl",
            div {
                class: "card-body gap-4",
                div {
                    class: "flex items-center justify-between gap-3",
                    h2 { class: "card-title", "Received Data" }
                    button {
                        class: "btn btn-ghost btn-sm",
                        onclick: move |_| {
                            *received_text.write() = String::new();
                            *status_message.write() = "Cleared received data.".to_string();
                        },
                        "Clear"
                    }
                }

                if received_text().is_empty() {
                    p { class: "text-base-content/60", "No serial data has been received yet." }
                } else {
                    pre {
                        class: "mockup-code max-h-96 overflow-auto whitespace-pre-wrap break-words rounded-box p-4 text-sm",
                        code { "{received_text()}" }
                    }
                }
            }
        }
    }
}
