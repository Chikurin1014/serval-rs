use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::{
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger},
    },
    elements::submenu,
    serial::{SerialContext, log::LogFormat},
};

/// The menu's items: a kind of log for a new file, in its plain format with
/// the others beside it, or an existing file in the format it holds.
#[derive(Clone, Copy, PartialEq)]
enum LogItem {
    Text,
    Hex,
    Raw,
    Existing,
}

impl LogItem {
    const ALL: [Self; 4] = [Self::Text, Self::Hex, Self::Raw, Self::Existing];

    fn name(self) -> &'static str {
        match self {
            Self::Text => "Text file",
            Self::Hex => "Hex file",
            Self::Raw => "Raw bytes",
            Self::Existing => "Continue with existing file",
        }
    }

    /// For a new file; `None` for an existing one.
    fn format(self) -> Option<LogFormat> {
        match self {
            Self::Text => Some(LogFormat::Text {
                timestamps: false,
                sent: false,
            }),
            Self::Hex => Some(LogFormat::Hex {
                timestamps: false,
                sent: false,
            }),
            Self::Raw => Some(LogFormat::Raw),
            Self::Existing => None,
        }
    }

    fn variants(self) -> &'static [(&'static str, LogFormat)] {
        match self {
            Self::Text => &[
                (
                    "with time",
                    LogFormat::Text {
                        timestamps: true,
                        sent: false,
                    },
                ),
                (
                    "with time and sent",
                    LogFormat::Text {
                        timestamps: true,
                        sent: true,
                    },
                ),
            ],
            Self::Hex => &[
                (
                    "with time",
                    LogFormat::Hex {
                        timestamps: true,
                        sent: false,
                    },
                ),
                (
                    "with time and sent",
                    LogFormat::Hex {
                        timestamps: true,
                        sent: true,
                    },
                ),
            ],
            Self::Raw | Self::Existing => &[],
        }
    }
}

/// Starts a log file in a format picked from a menu, or stops it. Its own
/// component, as the console renders again with each key typed.
#[component]
pub fn LogButton() -> Element {
    let serial = use_context::<SerialContext>();
    let mut open = use_signal(|| false);
    let mut in_variants = use_signal(|| false);
    // As the user picks: browsers ask for a file only then
    let mut start = move |format: Option<LogFormat>| {
        match format {
            Some(format) => serial.open_log(format),
            None => serial.continue_log(),
        }
        in_variants.set(false);
        open.set(false);
    };

    if serial.log_format().is_some() {
        return rsx! {
            Button {
                class: "log-stop",
                variant: ButtonVariant::Destructive,
                size: ButtonSize::Sm,
                aria_label: "Stop logging",
                onclick: move |_| serial.stop_log(),
                span { class: "log-stop-mark" }
                "STOP"
            }
        };
    }

    rsx! {
        div {
            class: "log-menu",
            onkeydown: move |event: KeyboardEvent| {
                if event.key() == Key::ArrowRight {
                    event.prevent_default();
                    document::eval(submenu::FOCUS_FIRST);
                }
            },
            DropdownMenu {
                open: Some(open()),
                on_open_change: move |value| {
                    if value || !in_variants() {
                        open.set(value);
                    }
                },
                DropdownMenuTrigger {
                    class: "log-rec",
                    aria_label: "Start logging",
                    span { class: "log-rec-mark" }
                    "REC"
                }
                DropdownMenuContent {
                    class: "log-menu-content",
                    for (index, item) in LogItem::ALL.into_iter().enumerate() {
                        DropdownMenuItem {
                            "data-variants": !item.variants().is_empty(),
                            "data-existing": item == LogItem::Existing,
                            value: item.format(),
                            index,
                            on_select: start,
                            match item {
                                LogItem::Text => rsx! { lucide::Type { class: "log-item-icon" } },
                                LogItem::Hex => rsx! { lucide::Hexagon { class: "log-item-icon" } },
                                LogItem::Raw => rsx! { lucide::Binary { class: "log-item-icon" } },
                                LogItem::Existing => rsx! { lucide::File { class: "log-item-icon" } },
                            }
                            span { class: "log-item-name", "{item.name()}" }
                            if !item.variants().is_empty() {
                                lucide::ChevronRight { class: "log-item-chevron" }
                                div {
                                    class: "log-variants",
                                    onfocusin: move |_| in_variants.set(true),
                                    onfocusout: move |_| in_variants.set(false),
                                    onkeydown: move |event: KeyboardEvent| {
                                        let script = match event.key() {
                                            Key::ArrowDown => submenu::FOCUS_NEXT,
                                            Key::ArrowUp => submenu::FOCUS_PREVIOUS,
                                            Key::ArrowLeft | Key::Escape => submenu::FOCUS_ITEM,
                                            Key::Enter => {
                                                event.stop_propagation();
                                                return;
                                            }
                                            _ => return,
                                        };
                                        event.prevent_default();
                                        event.stop_propagation();
                                        document::eval(script);
                                    },
                                    div {
                                        class: "log-variants-list",
                                        role: "menu",
                                        aria_label: "{item.name()} formats",
                                        for &(name, format) in item.variants() {
                                            button {
                                                class: "log-variant",
                                                r#type: "button",
                                                role: "menuitem",
                                                onclick: move |event: MouseEvent| {
                                                    event.stop_propagation();
                                                    start(Some(format));
                                                },
                                                "{name}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
