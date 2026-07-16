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
