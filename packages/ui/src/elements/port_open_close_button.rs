use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{LdPause, LdPlay},
};

use crate::{
    components::button::{Button, ButtonSize, ButtonVariant},
    serial::SerialContext,
};

const PORT_OPEN_CLOSE_BUTTON_CSS: Asset = asset!("/assets/styling/port-open-close-button.css");

#[component]
pub fn PortOpenCloseButton() -> Element {
    let SerialContext {
        ports,
        selected_id,
        is_open,
        set_open,
        ..
    } = use_context::<SerialContext>();
    let currently_open = is_open();
    let port_available = selected_id()
        .and_then(|id| ports().get(&id).and_then(|port| port.baudrate))
        .is_some();

    rsx! {
        document::Link { rel: "stylesheet", href: PORT_OPEN_CLOSE_BUTTON_CSS }

        Button {
            variant: ButtonVariant::Outline,
            size: ButtonSize::IconSm,
            class: "port-open-close",
            "data-open": currently_open,
            disabled: !port_available,
            aria_label: if currently_open { "Close port" } else { "Open port" },
            onclick: move |_| {
                if let Some(action) = set_open() {
                    action(!currently_open);
                }
            },
            if currently_open { Icon { icon: LdPause {} } } else { Icon { icon: LdPlay {} } }
        }
    }
}
