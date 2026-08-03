use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{LdPause, LdPlay},
    Icon,
};

use crate::serial::SerialContext;

#[component]
pub fn PortOpenCloseButton() -> Element {
    let SerialContext {
        ports,
        selected_id,
        mut is_open,
        set_open,
        ..
    } = use_context::<SerialContext>();

    let port_available = selected_id()
        .and_then(|id| ports().get(&id).and_then(|port| port.baudrate))
        .is_some();

    rsx! {
        label {
            class: "btn btn-circle btn-sm swap swap-rotate",
            input {
                type: "checkbox",
                disabled: !port_available,
                onclick: move |_| {
                    let will_open = !is_open();

                    if let Some(action) = set_open() {
                        action(will_open);
                    } else {
                        *is_open.write() = will_open;
                    }
                }
            }
            if is_open() {
                Icon {
                    class: "swap-on text-error",
                    icon: LdPause {},
                }
            } else if port_available {
                Icon {
                    class: "swap-off text-success",
                    icon: LdPlay {},
                }
            } else {
                Icon {
                    class: "swap-off text-base-content/60",
                    icon: LdPlay {},
                }
            }
        }
    }
}
