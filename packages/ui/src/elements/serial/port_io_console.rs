use std::{cell::Cell, rc::Rc};

use dioxus::prelude::*;
use dioxus_icons::lucide;

use super::{RecordButton, SendFileButton, send_file_button::send_picked_file};
use crate::{
    helper::{concat_scripts, format_bytes, hex, use_script, visible},
    serial::{OutgoingFile, SerialContext},
    time::TimeContext,
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

const CONSOLE_JS: &str = concat_scripts!(
    "theme.js",
    "helper/script/css_colors.js",
    "helper/script/line_column.js",
    "elements/serial/port_io_console.js",
);

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
    let time = use_context::<TimeContext>();
    let terminal = use_script(CONSOLE_JS);
    let mut view = use_signal(|| ConsoleView::Text);
    // Bumped as the script asks for all again, to draw it anew
    let mut redraws = use_signal(|| 0_u32);
    // Over all the window, as Tera Term's Alt+Enter
    let mut maximized = use_signal(|| false);
    // Drags over the console, entered and not yet left: each of its parts the
    // pointer goes over enters it anew, before the one it left is left
    let mut drags = use_signal(|| 0_i32);
    // A file dropped is sent as one picked, if one may be
    let takes_file = move || serial.is_open() && !serial.is_sending_file();

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
                // Each chunk's time as the app shows times
                let times: Vec<_> = (unread.times.iter())
                    .map(|&(at, ms)| (at, time.format_millis(ms)))
                    .collect();
                let _ = terminal.send(("data", unread.restarted, unread.bytes, times));
            }
        }
    });

    // From the script: `["typed", text]` to send, `["redraw"]`, `["break"]` or
    // `["maximize"]`
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
                    [kind] if kind == "break" => serial.send_break(),
                    [kind] if kind == "maximize" => maximized.toggle(),
                    _ => {}
                }
            }
        }
    });

    rsx! {
        document::Link { rel: "stylesheet", href: XTERM_CSS }
        document::Link { rel: "stylesheet", href: PORT_IO_CONSOLE_CSS }
        document::Script { src: XTERM_JS }
        document::Script { src: XTERM_FIT_JS }
        document::Script { src: XTERM_WEBGL_JS }

        div {
            class: "console",
            "data-maximized": maximized(),
            "data-dropping": drags() > 0 && takes_file(),
            ondragenter: move |_| drags += 1,
            ondragleave: move |_| drags.set((drags() - 1).max(0)),
            // Never to the browser, which would open the file in place of the app
            ondragover: move |event: DragEvent| {
                event.prevent_default();
                event.data_transfer().set_drop_effect(if takes_file() { "copy" } else { "none" });
            },
            ondrop: move |event: DragEvent| {
                event.prevent_default();
                drags.set(0);
                if !takes_file() {
                    return;
                }
                if let Some(file) = event.data_transfer().files().into_iter().next() {
                    send_picked_file(serial, file);
                }
            },
            div {
                class: "console-screen",
                div {
                    class: "console-tools",
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
                    // What was received, gone from the console (not from the data)
                    button {
                        class: "console-clear",
                        r#type: "button",
                        aria_label: "Clear the console",
                        title: "Clear the console",
                        onclick: move |_| serial.clear_history(),
                        lucide::BrushCleaning {}
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
                SendFileButton {}
                if let Some(file) = serial.sending_file() {
                    // A file is so much and no more: all the bar is its progress
                    FileProgress { file }
                } else {
                    div {
                        class: "console-send-bar",
                        div {
                            class: "console-send-buffer",
                            aria_label: "Send buffer",
                            div {
                                class: "console-send-text",
                                for outgoing in serial.outgoing() {
                                    if let Some(file) = &outgoing.file {
                                        // Its name, the part sent filled in as a bar
                                        span {
                                            class: "console-send-file",
                                            "data-sent": outgoing.sent,
                                            "data-stopped": file.stopped,
                                            style: "--sent: {100 * file.sent / file.size.max(1)}%",
                                            "{file.name} ({format_bytes(file.size as u64)})"
                                            if file.stopped {
                                                span { class: "console-send-file-note", " stopped" }
                                            }
                                        }
                                    } else {
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
                RecordButton {}
            }
        }
    }
}

/// The file being sent, its progress on all the bar: its text in one color on
/// the part sent, in another on the rest.
#[component]
fn FileProgress(file: OutgoingFile) -> Element {
    let percent = file.sent * 100 / file.size.max(1);
    let text = format!(
        "{} · {} / {} ({percent}%)",
        file.name,
        format_bytes(file.sent as u64),
        format_bytes(file.size as u64)
    );

    rsx! {
        div {
            class: "console-send-progress",
            role: "progressbar",
            aria_label: "Sending {file.name}",
            "aria-valuemin": 0,
            "aria-valuemax": 100,
            "aria-valuenow": percent,
            style: "--sent: {percent}%",
            div { class: "console-send-progress-fill" }
            span { class: "console-send-progress-text", "{text}" }
            span {
                class: "console-send-progress-text console-send-progress-on-fill",
                aria_hidden: "true",
                "{text}"
            }
        }
    }
}
