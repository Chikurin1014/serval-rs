use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::components::{
    button::{Button, ButtonSize, ButtonVariant},
    card::{Card, CardContent, CardHeader},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    drag_and_drop_list::DragAndDropList,
    dropdown_menu::DropdownMenuItem,
    switch::Switch,
};
use crate::data::{Conversion, DataContext, DataType, Map, MapContext, MapKind, Segment};
use crate::elements::{HoverMenu, HoverMenus, submenu};
use crate::helper::single_line;
use dioxus_primitives::drag_and_drop_list::use_drag_and_drop_list_items;

use super::field::MapEnabled;

const MAP_LIST_CSS: Asset = asset!("/assets/styling/map-list.css");
// Loaded here so it is ready before a card first opens
const MAP_FORM_CSS: Asset = asset!("/assets/styling/map-form.css");

pub(crate) const BYTES_LABELS_LIST_ID: &str = "map-bytes-labels";
pub(crate) const STRINGS_LABELS_LIST_ID: &str = "map-strings-labels";
pub(crate) const NUMBERS_LABELS_LIST_ID: &str = "map-numbers-labels";
pub(crate) const VECTOR2D_LABELS_LIST_ID: &str = "map-vector2d-labels";
pub(crate) const VECTOR3D_LABELS_LIST_ID: &str = "map-vector3d-labels";
pub(crate) const LOG_LABELS_LIST_ID: &str = "map-log-labels";

/// The add menus, each of the maps to the types it names.
const ADD_MENUS: [(&str, &[DataType]); 5] = [
    ("Bytes", &[DataType::Bytes]),
    ("Number", &[DataType::Number]),
    ("String", &[DataType::String]),
    ("Vector", &[DataType::Vector2d, DataType::Vector3d]),
    ("Log", &[DataType::Log]),
];

/// Lists the maps for editing; they run in `MapProvider` regardless.
#[component]
pub fn MapList() -> Element {
    let data_context = use_context::<DataContext>();
    let context = use_context::<MapContext>();
    let maps = use_memo(move || context.list());
    // Which maps there are, in no order
    let map_set = use_memo(move || {
        let mut ids: Vec<usize> = maps.read().iter().map(|map| map.id).collect();
        ids.sort_unstable();
        ids
    });
    // Bumped to make the list anew with all the maps, as it left one out
    let generation = use_signal(|| 0_u32);
    let bytes_labels = use_memo(move || data_context.labels_of(DataType::Bytes));
    let strings_labels = use_memo(move || data_context.labels_of(DataType::String));
    let numbers_labels = use_memo(move || data_context.labels_of(DataType::Number));
    let vector2d_labels = use_memo(move || data_context.labels_of(DataType::Vector2d));
    let vector3d_labels = use_memo(move || data_context.labels_of(DataType::Vector3d));
    let log_labels = use_memo(move || data_context.labels_of(DataType::Log));
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
            div {
                class: "map-list",
                // The list keeps its own order from its first items on: made
                // anew as maps are added or removed (not as they are reordered)
                for key in [format!("{generation}-{:?}", map_set())] {
                    DragAndDropList {
                        key: "{key}",
                        aria_label: "Maps",
                        items: maps
                            .read()
                            .iter()
                            .map(|map| rsx! { MapCard { key: "{map.id}", map: map.clone() } })
                            .collect::<Vec<_>>(),
                        MapOrder { generation }
                    }
                }
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
            datalist {
                id: VECTOR2D_LABELS_LIST_ID,
                for label in vector2d_labels() {
                    option { value: "{label}" }
                }
            }
            datalist {
                id: VECTOR3D_LABELS_LIST_ID,
                for label in vector3d_labels() {
                    option { value: "{label}" }
                }
            }
            datalist {
                id: LOG_LABELS_LIST_ID,
                for label in log_labels() {
                    option { value: "{label}" }
                }
            }
        }
    }
}

/// Puts the maps in the order they are dragged to. Should the list leave one
/// out (as Delete removes a focused item), makes it anew with all of them: a
/// map is deleted by its card's button.
#[component]
fn MapOrder(mut generation: Signal<u32>) -> Element {
    let mut context = use_context::<MapContext>();
    let shown: Vec<usize> = use_drag_and_drop_list_items()
        .iter()
        .filter_map(|item| item.key.parse().ok())
        .collect();
    use_effect(use_reactive!(|shown| {
        if !context.reorder(&shown) {
            generation += 1;
        }
    }));
    rsx! {}
}

/// As an add menu's item is pointed at or focused: its submenu to the left,
/// should it go past the window to the right (or, in a submenu to the left, as
/// that is, should it fit). Once for the page.
const SUBMENU_SIDE_JS: &str = r#"
if (!window.servalSubmenuSide) {
  window.servalSubmenuSide = (event) => {
    const item = event.target.closest?.('.add-map-content [data-presets="true"]');
    const submenu = item?.querySelector(":scope > .add-map-presets");
    if (!submenu) {
      return;
    }
    // Measured shown to the right, unseen
    delete submenu.dataset.side;
    const { display, visibility } = submenu.style;
    Object.assign(submenu.style, { display: "block", visibility: "hidden" });
    const { right, width } = submenu.getBoundingClientRect();
    Object.assign(submenu.style, { display, visibility });
    const fitsLeft = item.getBoundingClientRect().left - width >= 0;
    const inLeft = item.parentElement?.closest(".add-map-presets")?.dataset.side === "left";
    if ((inLeft && fitsLeft) || right > document.documentElement.clientWidth) {
      submenu.dataset.side = "left";
    }
  };
  document.addEventListener("mouseover", window.servalSubmenuSide);
  document.addEventListener("focusin", window.servalSubmenuSide);
}
"#;

/// A menu of the kinds of map for each output type.
#[component]
fn AddMapBar() -> Element {
    let kinds = use_context::<MapContext>().kinds();
    let menus = use_hook(HoverMenus::new);
    use_effect(|| {
        document::eval(SUBMENU_SIDE_JS);
    });

    rsx! {
        div {
            class: "add-map-bar",
            span {
                class: "add-map-label",
                "Map to"
            }
            div {
                class: "add-map-menus",
                for (index, (name, types)) in ADD_MENUS.into_iter().enumerate() {
                    if index > 0 {
                        span { class: "add-map-separator", "/" }
                    }
                    AddMapMenu {
                        name,
                        kinds: kinds
                            .iter()
                            .filter(|kind| types.contains(&kind.to))
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

/// The kinds of map to the types of the menu `name`: those of one name to
/// several of them (e.g. a 2D and a 3D vector) as one item, its variants
/// beside it, as presets are.
#[component]
fn AddMapMenu(
    name: &'static str,
    mut kinds: Vec<MapKind>,
    menus: HoverMenus,
    menu: usize,
) -> Element {
    let mut context = use_context::<MapContext>();
    // By their inputs' last menu: a number times a vector with the vector maps,
    // and those of 2D and 3D vectors together
    kinds.sort_by_key(|kind| {
        kind.from
            .iter()
            .filter_map(|&from| {
                ADD_MENUS
                    .iter()
                    .position(|(_, types)| types.contains(&from))
            })
            .max()
    });
    let mut items: Vec<Vec<MapKind>> = Vec::new();
    for kind in kinds {
        match items.iter_mut().find(|item| item[0].name == kind.name) {
            Some(item) => item.push(kind),
            None => items.push(vec![kind]),
        }
    }
    let mut in_presets = use_signal(|| false);

    rsx! {
        HoverMenu {
            menus,
            menu,
            disabled: items.is_empty(),
            content_class: "add-map-content",
            keep_open: in_presets,
            onkeydown: move |event: KeyboardEvent| {
                if event.key() == Key::ArrowRight {
                    event.prevent_default();
                    document::eval(submenu::FOCUS_FIRST);
                }
            },
            trigger: rsx! { "{name}" },
            for (index, variants) in items.into_iter().enumerate() {
                // The first, as its item is picked itself
                {
                    let kind = variants[0];
                    let has_variants = variants.len() > 1;
                    // Of the variants together: by their menus' names (e.g. Vector), once each
                    let mut from: Vec<&str> = Vec::new();
                    for &each in variants.iter().flat_map(|variant| variant.from) {
                        let each = if has_variants { menu_name(each) } else { each.name() };
                        if has_variants && from.contains(&each) {
                            continue;
                        }
                        from.push(each);
                    }
                    rsx! {
                        DropdownMenuItem {
                            "data-presets": has_variants || !kind.presets.is_empty(),
                            value: kind,
                            index,
                            on_select: move |kind| {
                                context.add(kind);
                                menus.close(menu);
                            },
                            span { class: "add-map-title", "{kind.name}" }
                            MapTypes {
                                from,
                                to: if has_variants { name } else { kind.to.name() },
                            }
                            if has_variants {
                                lucide::ChevronRight {}
                                AddMapSubmenu {
                                    label: "{kind.name} variants",
                                    open: in_presets,
                                    for variant in variants {
                                        // Its presets beside it, as the item's are
                                        div {
                                            class: "add-map-variant",
                                            "data-presets": !variant.presets.is_empty(),
                                            button {
                                                class: "add-map-preset",
                                                r#type: "button",
                                                role: "menuitem",
                                                aria_haspopup: if !variant.presets.is_empty() { "menu" },
                                                onclick: move |event: MouseEvent| {
                                                    event.stop_propagation();
                                                    context.add(variant);
                                                    in_presets.set(false);
                                                    menus.close(menu);
                                                },
                                                span { class: "add-map-preset-name", "{variant_name(variant)}" }
                                                MapTypes { from: type_names(variant.from), to: variant.to.name() }
                                                if !variant.presets.is_empty() {
                                                    lucide::ChevronRight {}
                                                }
                                            }
                                            if !variant.presets.is_empty() {
                                                AddMapSubmenu {
                                                    label: "{kind.name} {variant_name(variant)} presets",
                                                    open: in_presets,
                                                    for preset in variant.presets {
                                                        button {
                                                            class: "add-map-preset",
                                                            r#type: "button",
                                                            role: "menuitem",
                                                            onclick: move |event: MouseEvent| {
                                                                event.stop_propagation();
                                                                context.add_preset(variant, *preset);
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
                            } else if !kind.presets.is_empty() {
                                lucide::ChevronRight {}
                                AddMapSubmenu {
                                    label: "{kind.name} presets",
                                    open: in_presets,
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
}

/// In a submenu: the item `step` from the focused one, of its own (not those of
/// a submenu in it).
fn focus_sibling(step: i32) -> String {
    format!(
        r#"const current = document.activeElement;
const menu = current?.closest("[role=menu]");
if (menu) {{
  const items = [...menu.querySelectorAll("[role=menuitem]")]
    .filter((item) => item.closest("[role=menu]") === menu);
  items[items.indexOf(current) + ({step})]?.focus();
}}"#
    )
}

/// From a submenu's item into the submenu beside it, if it has one.
const FOCUS_NESTED: &str = r#"document.activeElement?.parentElement
  ?.querySelector(":scope > .add-map-presets [role=menuitem]")?.focus();"#;

/// Back to what the focused submenu is beside: the menu's item, or another
/// submenu's.
const FOCUS_OWNER: &str = r#"const owner = document.activeElement?.closest(".add-map-presets")?.parentElement;
(owner?.matches("[role=option]") ? owner : owner?.querySelector(":scope > [role=menuitem]"))?.focus();"#;

/// Beside an add menu's item: its presets or variants, the menu kept `open`
/// while one is focused.
#[component]
fn AddMapSubmenu(label: String, open: Signal<bool>, children: Element) -> Element {
    let mut open = open;

    rsx! {
        div {
            class: "add-map-presets",
            onfocusin: move |_| open.set(true),
            onfocusout: move |_| open.set(false),
            onkeydown: move |event: KeyboardEvent| {
                let script = match event.key() {
                    Key::ArrowDown => focus_sibling(1),
                    Key::ArrowUp => focus_sibling(-1),
                    Key::ArrowRight => FOCUS_NESTED.to_string(),
                    Key::ArrowLeft | Key::Escape => FOCUS_OWNER.to_string(),
                    Key::Enter => {
                        event.stop_propagation();
                        return;
                    }
                    _ => return,
                };
                event.prevent_default();
                // Not for the submenu (or menu) it is in
                event.stop_propagation();
                document::eval(&script);
            },
            div {
                class: "add-map-presets-list",
                role: "menu",
                aria_label: label,
                {children}
            }
        }
    }
}

/// What tells a variant from the others of its kind: the dimensions of the
/// vectors it gives, or else takes (as a dot product); or what it gives.
fn variant_name(kind: MapKind) -> &'static str {
    let dimensions = |data_type| match data_type {
        DataType::Vector2d => Some("2D"),
        DataType::Vector3d => Some("3D"),
        _ => None,
    };
    dimensions(kind.to)
        .or_else(|| kind.from.iter().copied().find_map(dimensions))
        .unwrap_or(kind.to.name())
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
        // Keys typed in the card are its own: not for the list's item, which
        // takes them to reorder (or Delete to remove it)
        div {
            class: "map-item",
            onkeydown: move |event: KeyboardEvent| event.stop_propagation(),
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
                                MapTypes { from: type_names(kind.from), to: kind.to.name() }
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

fn type_names(types: &[DataType]) -> Vec<&'static str> {
    types.iter().map(DataType::name).collect()
}

/// The name of the add menu of `data_type`, e.g. Vector for a 2D vector.
fn menu_name(data_type: DataType) -> &'static str {
    ADD_MENUS
        .iter()
        .find(|(_, types)| types.contains(&data_type))
        .map_or(data_type.name(), |(name, _)| name)
}

/// `from -> to` of a kind of map.
#[component]
fn MapTypes(from: Vec<&'static str>, to: &'static str) -> Element {
    rsx! {
        span {
            class: "map-types",
            span {
                class: "map-types-from",
                for from in from {
                    span { "{from}" }
                }
            }
            lucide::MoveRight {}
            span { "{to}" }
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
    let Conversion { from, to } = conversion;

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
            // As the inputs: each label beside its value, one above another
            span {
                class: "map-latest-to",
                for output in to {
                    span {
                        class: "map-latest-label",
                        Segments { segments: output.label }
                    }
                    span {
                        class: "map-latest-value",
                        Segments { segments: output.value }
                    }
                }
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
