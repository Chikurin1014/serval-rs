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
        is_open,
        set_open,
        ..
    } = use_context::<SerialContext>();

    let is_open = is_open();
    let port_available = selected_id()
        .and_then(|id| ports().get(&id).and_then(|port| port.baudrate))
        .is_some();

    let icon_color = if port_available {
        if is_open {
            "text-error"
        } else {
            "text-success"
        }
    } else {
        "text-base-content/60"
    };

    rsx! {
        label {
            class: "btn btn-circle btn-sm swap swap-rotate",
            input {
                type: "checkbox",
                disabled: !port_available,
                checked: is_open,
                onclick: move |_| {
                    if let Some(action) = set_open() {
                        action(!is_open);
                    }
                },
            }
            Icon {
                class: "swap-on {icon_color}",
                icon: LdPause {},
            }
            Icon {
                class: "swap-off {icon_color}",
                icon: LdPlay {},
            }
        }
        "is_open: {is_open.to_string()}, port_available: {port_available.to_string()}"
    }
}
