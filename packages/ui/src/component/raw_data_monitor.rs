use dioxus::prelude::*;

use crate::data::DataContext;

#[component]
pub fn RawDataMonitor() -> Element {
    let mut data_context = use_context::<DataContext>();
    let raw_data = data_context.raw_data();
    let text = raw_data
        .as_ref()
        .map(|data| {
            data.iter()
                .map(|data| String::from_utf8_lossy(data))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

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
                        disabled: raw_data.is_none(),
                        onclick: move |_| {
                            data_context.clear_raw();
                        },
                        "Clear"
                    }
                }

                pre {
                    class: "mockup-code max-h-96 overflow-auto whitespace-pre-wrap break-words rounded-box p-4 text-sm",
                    code { "{text}" }
                }
            }
        }
    }
}
