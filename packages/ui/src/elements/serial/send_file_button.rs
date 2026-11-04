use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::{
    helper::{FileInput, open_picked_file},
    save::PickedFile,
    serial::SerialContext,
};

const FILE_INPUT_ID: &str = "console-file-input";

/// Picks a file to send to the open port as it is, or stops the one being
/// sent. Its own component, as the console renders again with each key typed.
#[component]
pub fn SendFileButton() -> Element {
    let serial = use_context::<SerialContext>();
    let open = serial.is_open();
    let send = move |file: PickedFile| serial.send_file(file.name, file.bytes);

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
            // Through the platform's own way of opening a file, if it has one
            onclick: move |event: MouseEvent| {
                if open && open_picked_file(&[], send) {
                    event.prevent_default();
                }
            },
            FileInput {
                id: FILE_INPUT_ID,
                class: "console-file-input",
                aria_label: "Send a file",
                disabled: !open,
                on_pick: send,
            }
            lucide::Upload {}
        }
    }
}
