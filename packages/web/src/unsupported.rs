use dioxus::prelude::*;
use dioxus_icons::lucide;

use ui::components::{
    button::{Button, ButtonSize, ButtonVariant},
    dialog::{Dialog, DialogDescription, DialogTitle},
};

use crate::serial;

const WEB_SERIAL_DOCS: &str = "https://developer.mozilla.org/docs/Web/API/Web_Serial_API";

/// Tells, once the page opens, that this browser cannot open serial ports, and
/// which ones can. Shows nothing where Web Serial is available.
#[component]
pub fn UnsupportedBrowserDialog() -> Element {
    let mut open = use_signal(|| !serial::is_supported());

    rsx! {
        Dialog {
            open: open(),
            on_open_change: move |value| open.set(value),
            // At the top right corner of the dialog
            div {
                position: "absolute",
                top: "0.75rem",
                right: "0.75rem",
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::IconSm,
                    aria_label: "Close",
                    title: "Close",
                    onclick: move |_| open.set(false),
                    lucide::X {}
                }
            }
            DialogTitle { "This browser cannot open serial ports" }
            // Paragraphs as blocks of spans, as the description is a `p`; 30% of
            // a line apart
            DialogDescription {
                span { display: "block", "Serval uses Web Serial API." }
                span {
                    display: "block",
                    margin_top: "0.3lh",
                    "Please open this page with Chrome, Edge or another Chromium-based browser."
                }
                span {
                    display: "block",
                    margin_top: "0.3lh",
                    "About Web Serial API, see "
                    // In a new tab, so this page stays
                    a {
                        href: WEB_SERIAL_DOCS,
                        target: "_blank",
                        rel: "noopener noreferrer",
                        // A URL has no spaces to wrap at, and is wider than the dialog
                        overflow_wrap: "anywhere",
                        "{WEB_SERIAL_DOCS}"
                    }
                }
            }
        }
    }
}
