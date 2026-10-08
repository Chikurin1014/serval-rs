use std::{cell::RefCell, rc::Rc};

use dioxus::prelude::*;

use crate::{
    data::{ByteData, DataContext, NewEntries, RAW_BYTES_LABEL, SourceCursor},
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

/// A terminal of the received bytes; what is typed in it is sent as it is.
#[component]
pub fn PortIoConsole() -> Element {
    let serial = use_context::<SerialContext>();
    let data_context = use_context::<DataContext>();
    let terminal = use_hook(|| document::eval(CONSOLE_JS));

    // In a hook: the effect takes each render's closure, and a fresh cursor
    // would redraw the terminal from the start
    let cursor = use_hook(|| Rc::new(RefCell::new(SourceCursor::default())));
    use_effect(move || {
        let mut cursor = cursor.borrow_mut();
        let Some(NewEntries {
            entries, restarted, ..
        }) = cursor.new_entries::<ByteData>(&data_context, RAW_BYTES_LABEL)
        else {
            cursor.reset();
            let _ = terminal.send((true, Vec::<u8>::new()));
            return;
        };
        if restarted || !entries.is_empty() {
            let _ = terminal.send((restarted, raw_bytes(&entries)));
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
        }
    }
}

fn raw_bytes(entries: &[ByteData]) -> Vec<u8> {
    entries
        .iter()
        .flat_map(|entry| entry.value().iter().copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::raw_bytes;
    use crate::data::ByteData;

    #[test]
    fn raw_bytes_joins_the_chunks_in_order() {
        let entries = [
            ByteData::new(1, b"led: o".to_vec()),
            ByteData::new(2, b"n\n".to_vec()),
        ];
        assert_eq!(raw_bytes(&entries), b"led: on\n");
    }
}
