use std::{cell::Cell, rc::Rc};

use dioxus::prelude::*;
use dioxus_icons::lucide;

use super::LogButton;
use crate::{
    helper::{hex, visible},
    serial::SerialContext,
};

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

/// How the console shows what is received.
#[derive(Clone, Copy, PartialEq)]
enum ConsoleView {
    /// As a terminal does.
    Text,
    /// Its bytes in hex, beside them as characters (as `hexdump -C`).
    Hex,
}

impl ConsoleView {
    const ALL: [Self; 2] = [Self::Text, Self::Hex];

    fn name(self) -> &'static str {
        match self {
            Self::Text => "Text",
            Self::Hex => "HEX",
        }
    }

    /// As the terminal's script knows it.
    fn key(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Hex => "hex",
        }
    }
}

/// A terminal of the received bytes (from the port's history, not the data),
/// as text or in hex; what is typed in it is sent as it is, and shown below it
/// until the port has taken it.
#[component]
pub fn PortIoConsole() -> Element {
    let serial = use_context::<SerialContext>();
    let terminal = use_hook(|| document::eval(CONSOLE_JS));
    let mut view = use_signal(|| ConsoleView::Text);
    // Bumped as the script asks for all again, to draw it anew
    let mut redraws = use_signal(|| 0_u32);

    // In hooks: the effect takes each render's closure, and fresh ones would
    // redraw the terminal from the start
    let read_to = use_hook(|| Rc::new(Cell::new(None)));
    let shown = use_hook(|| Rc::new(Cell::new(None)));
    use_effect({
        let read_to = read_to.clone();
        move || {
            let view = view();
            redraws();
            if shown.get() != Some(view) {
                shown.set(Some(view));
                let _ = terminal.send(("view", view.key()));
                read_to.set(None);
            }
            let unread = serial.received_since(read_to.get());
            read_to.set(Some(unread.end));
            if unread.restarted || !unread.bytes.is_empty() {
                let _ = terminal.send(("data", unread.restarted, unread.bytes, unread.times));
            }
        }
    });

    // From the script: `["typed", text]` to send, or `["redraw"]`
    use_future(move || {
        let read_to = read_to.clone();
        async move {
            let mut terminal = terminal;
            while let Ok(message) = terminal.recv::<Vec<String>>().await {
                match message.as_slice() {
                    [kind, typed] if kind == "typed" => serial.send(typed.clone().into_bytes()),
                    [kind] if kind == "redraw" => {
                        read_to.set(None);
                        redraws += 1;
                    }
                    _ => {}
                }
            }
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
                class: "console-screen",
                div {
                    class: "console-views",
                    role: "tablist",
                    aria_label: "Console view",
                    for choice in ConsoleView::ALL {
                        button {
                            class: "console-view",
                            r#type: "button",
                            role: "tab",
                            aria_selected: view() == choice,
                            onclick: move |_| view.set(choice),
                            "{choice.name()}"
                        }
                    }
                }
                // When each line came, beside it: drawn by the terminal's script
                div { class: "console-times", id: "console-times", aria_hidden: "true" }
                div {
                    class: "console-output",
                    id: "console-output",
                    onmounted: move |_| {
                        let _ = terminal.send("console-output");
                    },
                }
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
                                    // Each byte with its space after it, in the bar
                                    if view() == ConsoleView::Hex {
                                        "{hex(&outgoing.bytes)} "
                                    } else {
                                        {visible(&outgoing.bytes)}
                                    }
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
                LogButton {}
            }
        }
    }
}
