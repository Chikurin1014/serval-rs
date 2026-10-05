use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::components::{
    button::{Button, ButtonSize, ButtonVariant},
    card::{Card, CardContent, CardHeader, CardTitle},
    dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger},
    switch::Switch,
    virtual_list::VirtualList,
};
use crate::data::{DataContext, DataType, MapContext};

const MAP_LIST_CSS: Asset = asset!("/assets/styling/map-list.css");

// `datalist` ids of the labels of each type, offered by the maps' forms
pub(crate) const BYTES_LABELS_LIST_ID: &str = "map-bytes-labels";
pub(crate) const STRINGS_LABELS_LIST_ID: &str = "map-strings-labels";
pub(crate) const NUMBERS_LABELS_LIST_ID: &str = "map-numbers-labels";

/// Lists the maps in `MapContext` for editing.
/// The maps run in `MapProvider`, whether or not this is mounted.
#[component]
pub fn MapList() -> Element {
    let data_context = use_context::<DataContext>();
    let mut context = use_context::<MapContext>();
    // Maps compare by id, so typing in a form does not re-render the list
    let maps = use_memo(move || context.list());
    // Memos, so the list re-renders when labels come and go, not on every entry
    let bytes_labels = use_memo(move || data_context.labels_of(DataType::Bytes));
    let strings_labels = use_memo(move || data_context.labels_of(DataType::String));
    let numbers_labels = use_memo(move || data_context.labels_of(DataType::Number));

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
                id: BYTES_LABELS_LIST_ID,
                for label in bytes_labels() {
                    option { value: "{label}" }
                }
            }
            datalist {
                id: STRINGS_LABELS_LIST_ID,
                for label in strings_labels() {
                    option { value: "{label}" }
                }
            }
            datalist {
                id: NUMBERS_LABELS_LIST_ID,
                for label in numbers_labels() {
                    option { value: "{label}" }
                }
            }
        }
    }
}
