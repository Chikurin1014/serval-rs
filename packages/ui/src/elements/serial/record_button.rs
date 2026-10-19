use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::{
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger},
    },
    elements::submenu::{Submenu, open_submenu},
    serial::{SerialContext, record::RecordFormat},
};

/// The menu's items: a kind of record for a new file, in its plain format with
/// the others beside it, or an existing file in the format it holds.
#[derive(Clone, Copy, PartialEq)]
enum RecordItem {
    Text,
    Hex,
    Raw,
    Existing,
}

impl RecordItem {
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
    fn format(self) -> Option<RecordFormat> {
        match self {
            Self::Text => Some(RecordFormat::Text {
                timestamps: false,
                sent: false,
            }),
            Self::Hex => Some(RecordFormat::Hex {
                timestamps: false,
                sent: false,
            }),
            Self::Raw => Some(RecordFormat::Raw),
            Self::Existing => None,
        }
    }

    fn variants(self) -> &'static [(&'static str, RecordFormat)] {
        match self {
            Self::Text => &[
                (
                    "with time",
                    RecordFormat::Text {
                        timestamps: true,
                        sent: false,
                    },
                ),
                (
                    "with time and sent",
                    RecordFormat::Text {
                        timestamps: true,
                        sent: true,
                    },
                ),
            ],
            Self::Hex => &[
                (
                    "with time",
                    RecordFormat::Hex {
                        timestamps: true,
                        sent: false,
                    },
                ),
                (
                    "with time and sent",
                    RecordFormat::Hex {
                        timestamps: true,
                        sent: true,
                    },
                ),
            ],
            Self::Raw | Self::Existing => &[],
        }
    }
}

/// Starts a record file in a format picked from a menu, or stops it. Its own
/// component, as the console renders again with each key typed.
#[component]
pub fn RecordButton() -> Element {
    let serial = use_context::<SerialContext>();
    let mut open = use_signal(|| false);
    let mut in_variants = use_signal(|| false);
    // As the user picks: browsers ask for a file only then
    let mut start = move |format: Option<RecordFormat>| {
        match format {
            Some(format) => serial.open_record(format),
            None => serial.continue_record(),
        }
        in_variants.set(false);
        open.set(false);
    };

    if serial.record_format().is_some() {
        return rsx! {
            Button {
                class: "record-stop",
                variant: ButtonVariant::Destructive,
                size: ButtonSize::Sm,
                aria_label: "Stop recording",
                onclick: move |_| serial.stop_record(),
                span { class: "record-stop-mark" }
                "STOP"
            }
        };
    }

    rsx! {
        div {
            class: "record-menu",
            onkeydown: open_submenu,
            DropdownMenu {
                open: Some(open()),
                on_open_change: move |value| {
                    if value || !in_variants() {
                        open.set(value);
                    }
                },
                DropdownMenuTrigger {
                    class: "record-start",
                    aria_label: "Start recording",
                    span { class: "record-start-mark" }
                    "REC"
                }
                DropdownMenuContent {
                    class: "record-menu-content",
                    for (index, item) in RecordItem::ALL.into_iter().enumerate() {
                        DropdownMenuItem {
                            "data-variants": !item.variants().is_empty(),
                            "data-existing": item == RecordItem::Existing,
                            value: item.format(),
                            index,
                            on_select: start,
                            match item {
                                RecordItem::Text => rsx! { lucide::Type { class: "record-item-icon" } },
                                RecordItem::Hex => rsx! { lucide::Hexagon { class: "record-item-icon" } },
                                RecordItem::Raw => rsx! { lucide::Binary { class: "record-item-icon" } },
                                RecordItem::Existing => rsx! { lucide::File { class: "record-item-icon" } },
                            }
                            span { class: "record-item-name", "{item.name()}" }
                            if !item.variants().is_empty() {
                                lucide::ChevronRight { class: "record-item-chevron" }
                                Submenu {
                                    class: "record-variants",
                                    list_class: "record-variants-list",
                                    label: "{item.name()} formats",
                                    focused: in_variants,
                                    for &(name, format) in item.variants() {
                                        button {
                                            class: "record-variant",
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

/// Asks before the page closes while a record is being written: what came since
/// the file last took it would be lost. Once in the app, as a record goes on
/// across the tabs.
#[component]
pub fn RecordUnloadGuard() -> Element {
    let serial = use_context::<SerialContext>();
    use_effect(move || {
        let method = if serial.record_format().is_some() {
            "addEventListener"
        } else {
            "removeEventListener"
        };
        document::eval(&format!(
            "window.servalRecordGuard ??= (event) => {{ event.preventDefault(); event.returnValue = ''; }};
            window.{method}('beforeunload', window.servalRecordGuard);"
        ));
    });
    rsx! {}
}
