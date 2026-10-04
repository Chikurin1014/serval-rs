use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::components::{
    button::{Button, ButtonSize, ButtonVariant},
    card::{Card, CardContent, CardHeader, CardTitle},
    dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger},
    switch::Switch,
    virtual_list::VirtualList,
};
use crate::data::{DataContext, MapContext, TypedData};

const MAP_LIST_CSS: Asset = asset!("/assets/styling/map-list.css");

/// Lists the maps in `MapContext` for editing.
/// The maps run in `MapProvider`, whether or not this is mounted.
#[component]
pub fn MapList() -> Element {
    let data_context = use_context::<DataContext>();
    let mut context = use_context::<MapContext>();
    // Maps compare by id, so typing in a form does not re-render the list
    let maps = use_memo(move || context.list());

    rsx! {
        document::Link { rel: "stylesheet", href: MAP_LIST_CSS }

        div {
            class: "map-panel",
            DropdownMenu {
                class: "add-map-menu",
                DropdownMenuTrigger {
                    class: "add-map",
                    "+ Add map"
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
                class: "map-list",
                count: maps.read().len(),
                render_item: move |index: usize| {
                    let Some(map) = maps.read().get(index).cloned() else {
                        return VNode::empty();
                    };
                    rsx! {
                        Card {
                            // By id, so removing a map does not hand its card to the next one
                            key: "{map.id}",
                            CardHeader {
                                Switch {
                                    checked: (map.enabled)(),
                                    on_checked_change: {
                                        let mut enabled = map.enabled;
                                        move |checked| enabled.set(checked)
                                    },
                                    aria_label: "Toggle map",
                                }
                                CardTitle { "{map.kind.name}" }
                                Button {
                                    variant: ButtonVariant::Ghost,
                                    size: ButtonSize::IconSm,
                                    aria_label: "Delete map",
                                    onclick: {
                                        let id = map.id;
                                        move |_| context.remove(id)
                                    },
                                    lucide::X {}
                                }
                            }
                            CardContent {
                                div {
                                    class: "map-content",
                                    {map.form()}
                                }
                            }
                        }
                    }
                },
            }
            datalist {
                id: "map-bytes-labels",
                for (label, _) in data_context.data_with_labels().iter().filter(|(_, data)| matches!(data, TypedData::Bytes(_))) {
                    option { value: "{label}" }
                }
            }
            datalist {
                id: "map-strings-labels",
                for (label, _) in data_context.data_with_labels().iter().filter(|(_, data)| matches!(data, TypedData::String(_))) {
                    option { value: "{label}" }
                }
            }
            datalist {
                id: "map-numbers-labels",
                for (label, _) in data_context.data_with_labels().iter().filter(|(_, data)| matches!(data, TypedData::Number(_))) {
                    option { value: "{label}" }
                }
            }
        }
    }
}
