use std::{cell::Cell, rc::Rc};

use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::{helper::visible, serial::SerialContext};

const PORT_IO_CONSOLE_CSS: Asset = asset!("/assets/styling/port-io-console.css");
const XTERM_CSS: Asset = asset!("/assets/vendor/xterm/xterm.css");
const XTERM_JS: Asset = asset!(
    "/assets/vendor/xterm/xterm.js",
    AssetOptions::js().with_minify(false)
);
const XTERM_FIT_JS: Asset = asset!(
    "/assets/vendor/xterm-addon-fit/addon-fit.js",
    AssetOptions::js().with_minify(false)
);
const XTERM_WEBGL_JS: Asset = asset!(
    "/assets/vendor/xterm-addon-webgl/addon-webgl.js",
    AssetOptions::js().with_minify(false)
);

const CONSOLE_JS: &str = include_str!("port_io_console.js");

/// One period down the send buffer's height, in an 8 by 10 box; as the masks in
/// `port-io-console.css`.
const WAVE: &str = "M4 0Q8 2.5 4 5T4 10";

/// A terminal of the received bytes (from the port's history, not the data);
/// what is typed in it is sent as it is, and shown below it until the port has
/// taken it.
#[component]
pub fn PortIoConsole() -> Element {
    let serial = use_context::<SerialContext>();
    let terminal = use_hook(|| document::eval(CONSOLE_JS));

    // In a hook: the effect takes each render's closure, and a fresh position
    // would redraw the terminal from the start
    let read_to = use_hook(|| Rc::new(Cell::new(None)));
    use_effect(move || {
        let unread = serial.received_since(read_to.get());
        read_to.set(Some(unread.end));
        if unread.restarted || !unread.bytes.is_empty() {
            let _ = terminal.send((unread.restarted, unread.bytes));
        }
    });

    use_future(move || async move {
        let mut terminal = terminal;
        while let Ok(typed) = terminal.recv::<String>().await {
            serial.send(typed.into_bytes());
        }
    });

    use_drop(move || {
        let _ = terminal.send(());
    });

    rsx! {
        document::Link { rel: "stylesheet", href: XTERM_CSS }
        document::Link { rel: "stylesheet", href: PORT_IO_CONSOLE_CSS }
        document::Script { src: XTERM_JS }
        document::Script { src: XTERM_FIT_JS }
        document::Script { src: XTERM_WEBGL_JS }

        div {
            class: "console",
            div {
                class: "console-output",
                id: "console-output",
                onmounted: move |_| {
                    let _ = terminal.send("console-output");
                },
            }
            div {
                class: "console-send",
                lucide::Send {}
                div {
                    class: "console-send-bar",
                    div {
                        class: "console-send-buffer",
                        aria_label: "Send buffer",
                        div {
                            class: "console-send-text",
                            for outgoing in serial.outgoing() {
                                span {
                                    "data-sent": outgoing.sent,
                                    {visible(&outgoing.bytes)}
                                }
                            }
                        }
                    }
                    div { class: "console-send-end", aria_hidden: "true" }
                    for side in ["buffer", "end"] {
                        svg {
                            class: "console-send-cut",
                            "data-side": side,
                            "aria-hidden": "true",
                            view_box: "0 0 8 10",
                            preserve_aspect_ratio: "none",
                            path { d: WAVE }
                        }
                    }
                }
            }
        }
    }
}
