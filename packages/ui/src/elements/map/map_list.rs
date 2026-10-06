use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::components::{
    button::{Button, ButtonSize, ButtonVariant},
    card::{Card, CardContent, CardHeader},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger},
    switch::Switch,
    virtual_list::VirtualList,
};
use crate::data::{Conversion, DataContext, DataType, Map, MapContext, MapKind, Segment};

const MAP_LIST_CSS: Asset = asset!("/assets/styling/map-list.css");
// Linked by the forms too; here so it is loaded before a card first opens
const MAP_FORM_CSS: Asset = asset!("/assets/styling/map-form.css");

// `datalist` ids of the labels of each type, offered by the maps' forms
pub(crate) const BYTES_LABELS_LIST_ID: &str = "map-bytes-labels";
pub(crate) const STRINGS_LABELS_LIST_ID: &str = "map-strings-labels";
pub(crate) const NUMBERS_LABELS_LIST_ID: &str = "map-numbers-labels";

/// The order of the types in the add menus, by output and then by input.
const TYPE_ORDER: [DataType; 3] = [DataType::Bytes, DataType::Number, DataType::String];

/// Lists the maps in `MapContext` for editing.
/// The maps run in `MapProvider`, whether or not this is mounted.
#[component]
pub fn MapList() -> Element {
    let data_context = use_context::<DataContext>();
    let context = use_context::<MapContext>();
    // Maps compare by id, so typing in a form does not re-render the list
    let maps = use_memo(move || context.list());
    // Memos, so the list re-renders when labels come and go, not on every entry
    let bytes_labels = use_memo(move || data_context.labels_of(DataType::Bytes));
    let strings_labels = use_memo(move || data_context.labels_of(DataType::String));
    let numbers_labels = use_memo(move || data_context.labels_of(DataType::Number));
    // The widest title with its types, so the cards' latest conversions line up,
    // and the longest type names, so the add menus' arrows do (see `map-list.css`)
    let columns = use_memo(move || {
        let kinds = context.kinds();
        let longest = |length: fn(&MapKind) -> usize| kinds.iter().map(length).max().unwrap_or(0);
        format!(
            "--map-heading-width: {}rem; --map-from-chars: {}; --map-to-chars: {};",
            kinds.iter().map(heading_width).fold(0.0, f64::max),
            longest(|kind| kind
                .from
                .iter()
                .map(|from| from.name().len())
                .max()
                .unwrap_or(0)),
            longest(|kind| kind.to.name().len()),
        )
    });

    rsx! {
        document::Link { rel: "stylesheet", href: MAP_LIST_CSS }
        document::Link { rel: "stylesheet", href: MAP_FORM_CSS }

        div {
            class: "map-panel",
            style: columns,
            AddMapBar {}
            // Only the cards in view are rendered
            VirtualList {
                class: "map-list",
                count: maps.read().len(),
                render_item: move |index: usize| {
                    let Some(map) = maps.read().get(index).cloned() else {
                        return VNode::empty();
                    };
                    // By id, so removing a map does not hand its card to the next one
                    rsx! { MapCard { key: "{map.id}", map } }
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

/// "+ Map to Bytes / Number / String": a menu of the kinds of map for each
/// output type, each listing them by input type, opening on hover.
#[component]
fn AddMapBar() -> Element {
    let kinds = use_context::<MapContext>().kinds();
    // Which output type's menu is open, and which one the pointer is over
    let open = use_signal(|| None::<DataType>);
    let hovered = use_signal(|| None::<DataType>);

    rsx! {
        div {
            class: "add-map-bar",
            span {
                class: "add-map-label",
                lucide::Plus {}
                "Map to"
            }
            div {
                class: "add-map-menus",
                for (index, to) in TYPE_ORDER.into_iter().enumerate() {
                    if index > 0 {
                        span { class: "add-map-separator", "/" }
                    }
                    AddMapMenu {
                        to,
                        kinds: kinds
                            .iter()
                            .filter(|kind| kind.to == to)
                            .copied()
                            .collect::<Vec<_>>(),
                        open,
                        hovered,
                    }
                }
            }
        }
    }
}

/// The menu of the kinds of map giving `to`, ordered by input type.
#[component]
fn AddMapMenu(
    to: DataType,
    mut kinds: Vec<MapKind>,
    mut open: Signal<Option<DataType>>,
    mut hovered: Signal<Option<DataType>>,
) -> Element {
    let mut context = use_context::<MapContext>();
    // By the first input, then the next
    kinds.sort_by_key(|kind| {
        kind.from
            .iter()
            .map(|&from| TYPE_ORDER.iter().position(|&order| order == from))
            .collect::<Vec<_>>()
    });
    let disabled = kinds.is_empty();
    let is_open = open() == Some(to);
    let mut close = move || {
        if *open.peek() == Some(to) {
            open.set(None);
        }
    };

    rsx! {
        div {
            class: "add-map-menu",
            onmouseenter: move |_| {
                hovered.set(Some(to));
                if !disabled {
                    open.set(Some(to));
                }
            },
            onmouseleave: move |_| {
                hovered.set(None);
                close();
            },
            DropdownMenu {
                open: Some(is_open),
                on_open_change: move |value| {
                    if value {
                        open.set(Some(to));
                    } else if hovered() != Some(to) {
                        // While hovered it stays open, e.g. when its trigger is clicked
                        close();
                    }
                },
                disabled,
                DropdownMenuTrigger {
                    class: "add-map",
                    "{to.name()}"
                    if is_open {
                        lucide::ChevronUp {}
                    } else {
                        lucide::ChevronDown {}
                    }
                }
                DropdownMenuContent {
                    class: "add-map-content",
                    for (index, kind) in kinds.into_iter().enumerate() {
                        DropdownMenuItem {
                            value: kind,
                            index,
                            on_select: move |kind| {
                                context.add(kind);
                                close();
                            },
                            // Title, then `from -> to`
                            span { class: "add-map-title", "{kind.name}" }
                            MapTypes { from: kind.from, to: kind.to }
                        }
                    }
                }
            }
        }
    }
}

/// A map's card: its title and latest conversion, opening to its settings.
#[component]
fn MapCard(map: Map) -> Element {
    let mut context = use_context::<MapContext>();
    let Map {
        id,
        kind,
        mut enabled,
        mut open,
        latest,
        ..
    } = map;

    rsx! {
        Collapsible {
            class: "map-card",
            open: Some(open()),
            on_open_change: move |value| open.set(value),
            Card {
                CardHeader {
                    Switch {
                        checked: enabled(),
                        on_checked_change: move |checked| enabled.set(checked),
                        aria_label: "Toggle map",
                    }
                    CollapsibleTrigger {
                        class: "map-title",
                        span {
                            class: "map-heading",
                            span { class: "map-title-text", "{kind.name}" }
                            MapTypes { from: kind.from, to: kind.to }
                        }
                        if let Some(conversion) = latest() {
                            LatestConversion { conversion }
                        }
                    }
                    // Shown while the card is hovered
                    Button {
                        class: "map-remove",
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::IconSm,
                        aria_label: "Delete map",
                        title: "Delete map",
                        onclick: move |_| context.remove(id),
                        lucide::X {}
                    }
                }
                CollapsibleContent {
                    CardContent {
                        div {
                            class: "map-content",
                            {map.form()}
                        }
                    }
                }
            }
        }
    }
}

/// Roughly how wide a card's title and types are, in rem: by the average
/// width of a character in their fonts (see `.map-heading` in `map-list.css`),
/// rounded up to a quarter.
fn heading_width(kind: &MapKind) -> f64 {
    let title = kind.name.chars().count() as f64 * 0.75 * 0.65;
    // The inputs one above the other, so only the longest counts
    let from = kind
        .from
        .iter()
        .map(|from| from.name().len())
        .max()
        .unwrap_or(0);
    let types = (from + kind.to.name().len()) as f64 * 0.7 * 0.6;
    // The gaps around the arrow and after the title, and the arrow
    let width = title + types + 0.25 * 2.0 + 0.5 + 0.75;
    (width * 4.0).ceil() / 4.0
}

/// `from -> to`: the types a kind of map turns data from and to, its inputs
/// one above the other.
#[component]
fn MapTypes(from: &'static [DataType], to: DataType) -> Element {
    rsx! {
        span {
            class: "map-types",
            span {
                class: "map-types-from",
                for from in from {
                    span { "{from.name()}" }
                }
            }
            lucide::MoveRight {}
            span { "{to.name()}" }
        }
    }
}

/// `input value -> output value`, with the parts of the output taken from
/// the input marked; the values of several inputs one above the other.
#[component]
fn LatestConversion(conversion: Conversion) -> Element {
    let Conversion {
        from,
        to_label,
        to_value,
    } = conversion;

    rsx! {
        span {
            class: "map-latest",
            span {
                class: "map-latest-from",
                for input in from {
                    span { class: "map-latest-label", "{input.label}" }
                    span { class: "map-latest-value", {single_line(&input.value)} }
                }
            }
            lucide::MoveRight {}
            span {
                class: "map-latest-label",
                Segments { segments: to_label }
            }
            span {
                class: "map-latest-value",
                Segments { segments: to_value }
            }
        }
    }
}

#[component]
fn Segments(segments: Vec<Segment>) -> Element {
    rsx! {
        for segment in segments {
            span {
                class: "map-segment",
                "data-from-input": segment.from_input,
                {single_line(&segment.text)}
            }
        }
    }
}

/// `text` with its line breaks and tabs escaped, to fit on one line.
fn single_line(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::single_line;

    #[test]
    fn single_line_escapes_line_breaks() {
        assert_eq!(single_line("led: on\r\n"), "led: on\\r\\n");
        assert_eq!(single_line("a\tb\\"), "a\\tb\\\\");
    }
}
