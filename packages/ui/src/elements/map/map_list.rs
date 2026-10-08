use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::components::{
    button::{Button, ButtonSize, ButtonVariant},
    card::{Card, CardContent, CardHeader},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    dropdown_menu::DropdownMenuItem,
    switch::Switch,
    virtual_list::VirtualList,
};
use crate::data::{Conversion, DataContext, DataType, Map, MapContext, MapKind, Segment};
use crate::elements::{HoverMenu, HoverMenus};
use crate::helper::single_line;

use super::field::MapEnabled;

const MAP_LIST_CSS: Asset = asset!("/assets/styling/map-list.css");
// Loaded here so it is ready before a card first opens
const MAP_FORM_CSS: Asset = asset!("/assets/styling/map-form.css");

pub(crate) const BYTES_LABELS_LIST_ID: &str = "map-bytes-labels";
pub(crate) const STRINGS_LABELS_LIST_ID: &str = "map-strings-labels";
pub(crate) const NUMBERS_LABELS_LIST_ID: &str = "map-numbers-labels";

const FOCUS_FIRST_PRESET: &str =
    "document.activeElement?.querySelector('.add-map-preset')?.focus()";
const FOCUS_NEXT_PRESET: &str = "document.activeElement?.nextElementSibling?.focus()";
const FOCUS_PREVIOUS_PRESET: &str = "document.activeElement?.previousElementSibling?.focus()";
const FOCUS_PRESETS_ITEM: &str = "document.activeElement?.closest('[role=option]')?.focus()";

const TYPE_ORDER: [DataType; 3] = [DataType::Bytes, DataType::Number, DataType::String];

/// Lists the maps for editing; they run in `MapProvider` regardless.
#[component]
pub fn MapList() -> Element {
    let data_context = use_context::<DataContext>();
    let context = use_context::<MapContext>();
    let maps = use_memo(move || context.list());
    let bytes_labels = use_memo(move || data_context.labels_of(DataType::Bytes));
    let strings_labels = use_memo(move || data_context.labels_of(DataType::String));
    let numbers_labels = use_memo(move || data_context.labels_of(DataType::Number));
    // So the latest conversions and the add menus' arrows line up
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
            VirtualList {
                class: "map-list",
                count: maps.read().len(),
                render_item: move |index: usize| {
                    let Some(map) = maps.read().get(index).cloned() else {
                        return VNode::empty();
                    };
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

/// A menu of the kinds of map for each output type.
#[component]
fn AddMapBar() -> Element {
    let kinds = use_context::<MapContext>().kinds();
    let menus = use_hook(HoverMenus::new);

    rsx! {
        div {
            class: "add-map-bar",
            span {
                class: "add-map-label",
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
                        menus,
                        menu: index,
                    }
                }
            }
        }
    }
}

#[component]
fn AddMapMenu(to: DataType, mut kinds: Vec<MapKind>, menus: HoverMenus, menu: usize) -> Element {
    let mut context = use_context::<MapContext>();
    kinds.sort_by_key(|kind| {
        kind.from
            .first()
            .and_then(|&from| TYPE_ORDER.iter().position(|&order| order == from))
    });
    let mut in_presets = use_signal(|| false);

    rsx! {
        HoverMenu {
            menus,
            menu,
            disabled: kinds.is_empty(),
            content_class: "add-map-content",
            keep_open: in_presets,
            onkeydown: move |event: KeyboardEvent| {
                if event.key() == Key::ArrowRight {
                    event.prevent_default();
                    document::eval(FOCUS_FIRST_PRESET);
                }
            },
            trigger: rsx! { "{to.name()}" },
            for (index, kind) in kinds.into_iter().enumerate() {
                DropdownMenuItem {
                    "data-presets": !kind.presets.is_empty(),
                    value: kind,
                    index,
                    on_select: move |kind| {
                        context.add(kind);
                        menus.close(menu);
                    },
                    span { class: "add-map-title", "{kind.name}" }
                    MapTypes { from: kind.from, to: kind.to }
                    if !kind.presets.is_empty() {
                        lucide::ChevronRight {}
                        div {
                            class: "add-map-presets",
                            onfocusin: move |_| in_presets.set(true),
                            onfocusout: move |_| in_presets.set(false),
                            onkeydown: move |event: KeyboardEvent| {
                                let script = match event.key() {
                                    Key::ArrowDown => FOCUS_NEXT_PRESET,
                                    Key::ArrowUp => FOCUS_PREVIOUS_PRESET,
                                    Key::ArrowLeft | Key::Escape => FOCUS_PRESETS_ITEM,
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
                                class: "add-map-presets-list",
                                role: "menu",
                                aria_label: "{kind.name} presets",
                                for preset in kind.presets {
                                    button {
                                        class: "add-map-preset",
                                        r#type: "button",
                                        role: "menuitem",
                                        onclick: move |event: MouseEvent| {
                                            event.stop_propagation();
                                            context.add_preset(kind, *preset);
                                            in_presets.set(false);
                                            menus.close(menu);
                                        },
                                        span { class: "add-map-preset-name", "{preset.name}" }
                                        span { class: "add-map-preset-detail", "{preset.detail}" }
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
    use_context_provider(|| MapEnabled(enabled.into()));

    rsx! {
        Collapsible {
            class: "map-card reveals",
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
                        // Its own component: it changes with each conversion,
                        // and the card must not render again with it
                        Latest { latest }
                    }
                    Button {
                        class: "map-remove reveal-on-hover",
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
                            if enabled() {
                                // One for the whole form, in its middle
                                div { class: "map-lock-tip", role: "tooltip", "Turn this Map off to edit" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Roughly how wide a card's title and types are, in rem.
fn heading_width(kind: &MapKind) -> f64 {
    let title = kind.name.chars().count() as f64 * 0.75 * 0.65;
    let from = kind
        .from
        .iter()
        .map(|from| from.name().len())
        .max()
        .unwrap_or(0);
    let types = (from + kind.to.name().len()) as f64 * 0.7 * 0.6;
    let width = title + types + 0.25 * 2.0 + 0.5 + 0.75;
    (width * 4.0).ceil() / 4.0
}

/// `from -> to` of a kind of map.
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

/// The latest conversion of a map, if any.
#[component]
fn Latest(latest: ReadSignal<Option<Conversion>>) -> Element {
    rsx! {
        if let Some(conversion) = latest() {
            LatestConversion { conversion }
        }
    }
}

/// `input -> output`, marking the parts of the output taken from the input.
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
