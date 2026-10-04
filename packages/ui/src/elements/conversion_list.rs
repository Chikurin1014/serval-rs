use dioxus::prelude::*;
use dioxus_free_icons::{Icon, icons::ld_icons::LdX};

use crate::components::{
    button::{Button, ButtonSize, ButtonVariant},
    card::{Card, CardContent, CardHeader, CardTitle},
    dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger},
    switch::Switch,
    virtual_list::VirtualList,
};
use crate::data::{ConversionContext, DataContext, TypedData};

const CONVERSION_LIST_CSS: Asset = asset!("/assets/styling/conversion-list.css");

/// Lists the conversions in `ConversionContext` for editing.
/// The conversions run in `ConversionProvider`, whether or not this is mounted.
#[component]
pub fn ConversionList() -> Element {
    let data_context = use_context::<DataContext>();
    let mut context = use_context::<ConversionContext>();
    // Conversions compare by id, so typing in a form does not re-render the list
    let conversions = use_memo(move || context.list());

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
                    for (index, kind) in context.kinds().into_iter().enumerate() {
                        DropdownMenuItem {
                            value: kind,
                            index,
                            on_select: move |kind| {
                                context.add(kind);
                            },
                            "{kind.name}"
                        }
                    }
                }
            }
            // Only the cards in view are rendered
            VirtualList {
                class: "conversion-list",
                count: conversions.read().len(),
                render_item: move |index: usize| {
                    let Some(conversion) = conversions.read().get(index).cloned() else {
                        return VNode::empty();
                    };
                    rsx! {
                        Card {
                            // By id, so removing a conversion does not hand its card to the next one
                            key: "{conversion.id}",
                            CardHeader {
                                Switch {
                                    checked: (conversion.enabled)(),
                                    on_checked_change: {
                                        let mut enabled = conversion.enabled;
                                        move |checked| enabled.set(checked)
                                    },
                                    aria_label: "Toggle conversion",
                                }
                                CardTitle { "{conversion.kind.name}" }
                                Button {
                                    variant: ButtonVariant::Ghost,
                                    size: ButtonSize::IconSm,
                                    aria_label: "Delete conversion",
                                    onclick: {
                                        let id = conversion.id;
                                        move |_| context.remove(id)
                                    },
                                    Icon { icon: LdX {} }
                                }
                            }
                            CardContent {
                                div {
                                    class: "conversion-content",
                                    {conversion.form()}
                                }
                            }
                        }
                    }
                },
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
