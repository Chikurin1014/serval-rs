use dioxus::prelude::*;
use dioxus_free_icons::{icons::ld_icons::LdTrash, Icon};
use dioxus_primitives::scroll_area::ScrollDirection;

use super::{ConversionByteToString, ConversionStringToNumber, ConversionStringToString};
use crate::components::{
    button::{Button, ButtonSize, ButtonVariant},
    card::{Card, CardContent, CardHeader, CardTitle},
    dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger},
    scroll_area::ScrollArea,
    switch::Switch,
};
use crate::data::{DataContext, TypedData};

const CONVERSION_LIST_CSS: Asset = asset!("/assets/styling/conversion-list.css");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConversionKind {
    ByteToString,
    StringToNumber,
    StringToString,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ConversionItem {
    id: usize,
    kind: ConversionKind,
    enabled: Signal<bool>,
}

#[component]
pub fn ConversionList() -> Element {
    let data_context = use_context::<DataContext>();
    let mut items = use_signal(|| {
        vec![ConversionItem {
            id: 0,
            kind: ConversionKind::ByteToString,
            enabled: Signal::new(true),
        }]
    });
    let mut next_id = use_signal(|| 1usize);

    rsx! {
        document::Link { rel: "stylesheet", href: CONVERSION_LIST_CSS }

        div {
            class: "conversion-panel",
            DropdownMenu {
                class: "add-conversion-menu",
                DropdownMenuTrigger {
                    class: "add-conversion",
                    "+ Add conversion"
                }
                DropdownMenuContent {
                    for (index, (label, kind)) in [
                        ("Split Bytes into String", ConversionKind::ByteToString),
                        ("Convert String to Number", ConversionKind::StringToNumber),
                        ("Convert String to String", ConversionKind::StringToString),
                    ].into_iter().enumerate() {
                        DropdownMenuItem {
                            value: kind,
                            index,
                            on_select: move |kind| {
                                let id = next_id();
                                next_id.set(id + 1);
                                items.write().push(ConversionItem {
                                    id,
                                    kind,
                                    enabled: Signal::new(false),
                                });
                            },
                            "{label}"
                        }
                    }
                }
            }
            ScrollArea {
                max_height: "50vh",
                direction: ScrollDirection::Vertical,
                for mut item in items() {
                    Card {
                        key: "{item.id}",
                        CardHeader {
                            Switch {
                                checked: (item.enabled)(),
                                on_checked_change: move |new_checked| item.enabled.set(new_checked),
                                aria_label: "Toggle conversion",
                            }
                            CardTitle {
                                match item.kind {
                                    ConversionKind::ByteToString => "Split Bytes into String",
                                    ConversionKind::StringToNumber => "Convert String to Number",
                                    ConversionKind::StringToString => "Convert String to String",
                                }
                            }
                            Button {
                                variant: ButtonVariant::Ghost,
                                size: ButtonSize::IconSm,
                                aria_label: "Delete conversion",
                                onclick: move |_| {
                                    let mut list = items.write();
                                    if let Some(index) = list.iter().position(|value| value.id == item.id) {
                                        list.remove(index);
                                    }
                                },
                                Icon { icon: LdTrash {} }
                            }
                        }
                        CardContent {
                            div {
                                class: "conversion-content",
                                match item.kind {
                                    ConversionKind::ByteToString => rsx! {
                                        ConversionByteToString {
                                            initial_source_label: if item.id == 0 { "raw_data".to_string() } else { String::new() },
                                            initial_target_label: if item.id == 0 { "raw_str".to_string() } else { String::new() },
                                            initial_delimiter: "\\n".to_string(),
                                            enabled: read_signal(item.enabled),
                                        }
                                    },
                                    ConversionKind::StringToNumber => rsx! {
                                        ConversionStringToNumber { enabled: read_signal(item.enabled) }
                                    },
                                    ConversionKind::StringToString => rsx! {
                                        ConversionStringToString { enabled: read_signal(item.enabled) }
                                    },
                                }
                            }
                        }
                    }
                }
            }
            datalist {
                id: "conversion-bytes-labels",
                for (label, _) in data_context.data_with_labels().iter().filter(|(_, data)| matches!(data, TypedData::Bytes(_))) {
                    option { value: "{label}" }
                }
            }
            datalist {
                id: "conversion-strings-labels",
                for (label, _) in data_context.data_with_labels().iter().filter(|(_, data)| matches!(data, TypedData::String(_))) {
                    option { value: "{label}" }
                }
            }
            datalist {
                id: "conversion-numbers-labels",
                for (label, _) in data_context.data_with_labels().iter().filter(|(_, data)| matches!(data, TypedData::Number(_))) {
                    option { value: "{label}" }
                }
            }
        }
    }
}

fn read_signal(signal: Signal<bool>) -> ReadSignal<bool> {
    signal.into()
}
