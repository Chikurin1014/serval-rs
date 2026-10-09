use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::{
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger},
    },
    serial::{Restart, SerialContext},
};

const PORT_SIGNAL_BUTTON_CSS: Asset = asset!("/assets/styling/port-signal-button.css");

/// What the button does with DTR and RTS.
#[derive(Clone, Copy, PartialEq)]
enum SignalAction {
    Restart(Restart),
    /// A toggle for each.
    Manual,
}

impl SignalAction {
    const ALL: [Self; 3] = [
        Self::Restart(Restart::Arduino),
        Self::Restart(Restart::Espressif),
        Self::Manual,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::Restart(Restart::Arduino) => "Restart (Arduino)",
            Self::Restart(Restart::Espressif) => "Restart (Espressif)",
            Self::Manual => "Manual",
        }
    }
}

/// Restarts the board on the open port, or sets its DTR and RTS: as picked
/// from the menu beside it.
#[component]
pub fn PortSignalButton() -> Element {
    let serial = use_context::<SerialContext>();
    let mut action = use_signal(|| SignalAction::Restart(Restart::Arduino));
    let open = serial.is_open();
    let signals = serial.signals();

    rsx! {
        document::Link { rel: "stylesheet", href: PORT_SIGNAL_BUTTON_CSS }

        div {
            class: "port-signal",
            role: "group",
            aria_label: "Signals",
            match action() {
                SignalAction::Restart(restart) => rsx! {
                    Button {
                        class: "port-signal-action",
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Sm,
                        disabled: !open,
                        onclick: move |_| serial.restart(restart),
                        lucide::RotateCcw {}
                        "{action().name()}"
                    }
                },
                SignalAction::Manual => rsx! {
                    for (name, on, dtr, rts) in [
                        ("DTR", signals.dtr, Some(!signals.dtr), None),
                        ("RTS", signals.rts, None, Some(!signals.rts)),
                    ] {
                        Button {
                            class: "port-signal-action port-signal-toggle",
                            variant: ButtonVariant::Outline,
                            size: ButtonSize::Sm,
                            disabled: !open,
                            aria_pressed: on,
                            "data-on": on,
                            onclick: move |_| serial.set_signals(dtr, rts),
                            span { class: "port-signal-lamp" }
                            "{name}"
                        }
                    }
                },
            }
            DropdownMenu {
                class: "port-signal-menu",
                DropdownMenuTrigger {
                    class: "port-signal-trigger",
                    aria_label: "Signal action",
                    lucide::ChevronDown {}
                }
                DropdownMenuContent {
                    class: "port-signal-menu-content",
                    for (index, choice) in SignalAction::ALL.into_iter().enumerate() {
                        DropdownMenuItem {
                            value: choice,
                            index,
                            "data-chosen": choice == action(),
                            on_select: move |choice| action.set(choice),
                            "{choice.name()}"
                        }
                    }
                }
            }
        }
    }
}
