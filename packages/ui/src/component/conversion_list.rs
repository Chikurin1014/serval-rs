use dioxus::prelude::*;
use dioxus_free_icons::{icons::ld_icons::LdX, Icon};

use super::{ConversionByteToString, ConversionStringToNumber, ConversionStringToString};
use crate::data::{DataContext, TypedData};

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
        div {
            class: "overflow-y-auto min-w-100",
            for mut item in items() {
                div {
                    key: "{item.id}",
                    class: "card w-full shadow",
                    div {
                        class: "card-body",
                        div {
                            class: "flex items-center justify-between gap-2",
                            input {
                                type: "checkbox",
                                class: "toggle toggle-sm toggle-primary",
                                checked: (item.enabled)(),
                                onchange: move |_| item.enabled.set(!(item.enabled)()),
                            }
                            h2 {
                                class: "card-title text-sm",
                                match item.kind {
                                    ConversionKind::ByteToString => rsx!(span {
                                        class: "flex items-center gap-1",
                                        "Split"
                                        div { class: "font-mono", "Bytes" }
                                        "into"
                                        div { class: "font-mono", "String" }
                                    }),
                                    ConversionKind::StringToNumber => rsx!(span {
                                        class: "flex items-center gap-1",
                                        "Convert"
                                        div { class: "font-mono", "String" }
                                        "to"
                                        div { class: "font-mono", "Number" }
                                    }),
                                    ConversionKind::StringToString => rsx!(span {
                                        class: "flex items-center gap-1",
                                        "Convert"
                                        div { class: "font-mono", "String" }
                                        "to"
                                        div { class: "font-mono", "String" }
                                    }),
                                }
                            }
                            button {
                                class: "btn btn-xs btn-ghost btn-error btn-square",
                                onclick: move |_| {
                                    let mut list = items.write();
                                    if let Some(index) = list.iter().position(|value| value.id == item.id) {
                                        list.remove(index);
                                    }
                                },
                                Icon { icon: LdX {} }
                            }
                        }
                        match item.kind {
                            ConversionKind::ByteToString => rsx! {
                                ConversionByteToString {
                                    initial_source_label: if item.id == 0 { "raw_data".to_string() } else { String::new() },
                                    initial_target_label: if item.id == 0 { "raw_str".to_string() } else { String::new() },
                                    initial_delimiter: "\\n".to_string(),
                                    enabled: read_signal(item.enabled),
                                }
                            },
                            ConversionKind::StringToNumber => rsx! { ConversionStringToNumber { enabled: read_signal(item.enabled) } },
                            ConversionKind::StringToString => rsx! { ConversionStringToString { enabled: read_signal(item.enabled) } },
                        }
                    }
                }
            }
            button {
                class: "btn btn-sm btn-block btn-dash",
                popovertarget: "conversion-add-dropdown",
                style: "anchor-name:--anchor-conversion-add-dropdown;",
                "+ Add conversion"
            }
            ul {
                class: "dropdown menu rounded-box z-20 w-56 bg-base-300 shadow-sm",
                popover: true,
                id: "conversion-add-dropdown",
                style: "position-anchor:--anchor-conversion-add-dropdown;",
                for (label, kind) in [
                    ("Split Bytes into String", ConversionKind::ByteToString),
                    ("Convert String to Number", ConversionKind::StringToNumber),
                    ("Convert String to String", ConversionKind::StringToString),
                ] {
                    li {
                            button {
                                class: "w-full text-left",
                                popovertarget: "conversion-add-dropdown",
                                popovertargetaction: "hide",
                                onclick: move |_| {
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
