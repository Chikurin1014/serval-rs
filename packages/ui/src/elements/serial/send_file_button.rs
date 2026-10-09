use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::serial::SerialContext;

/// Picks a file to send to the open port as it is, or stops the one being
/// sent. Its own component, as the console renders again with each key typed.
#[component]
pub fn SendFileButton() -> Element {
    let serial = use_context::<SerialContext>();
    // A fresh input for each file, so the same one may be picked again
    let mut picks = use_signal(|| 0_u32);
    let open = serial.is_open();

    if serial.is_sending_file() {
        return rsx! {
            button {
                class: "console-file console-file-stop",
                r#type: "button",
                title: "Stop sending the file",
                aria_label: "Stop sending the file",
                onclick: move |_| serial.stop_file(),
                lucide::X {}
            }
        };
    }

    rsx! {
        label {
            class: "console-file",
            title: "Send a file",
            "data-disabled": !open,
            input {
                key: "{picks}",
                class: "console-file-input",
                r#type: "file",
                aria_label: "Send a file",
                disabled: !open,
                onchange: move |event: FormEvent| {
                    let Some(file) = event.files().into_iter().next() else {
                        return;
                    };
                    picks += 1;
                    spawn(async move {
                        match file.read_bytes().await {
                            Ok(bytes) => serial.send_file(file.name(), bytes.to_vec()),
                            Err(error) => serial.report_error("Failed to read the file", &error.to_string()),
                        }
                    });
                },
            }
            lucide::Upload {}
        }
    }
}
