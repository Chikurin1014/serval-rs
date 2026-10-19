use dioxus::prelude::*;

use crate::components::drag_and_drop_list::DragAndDropList;
use crate::data::{DataContext, DataType, MapContext, MapKind};
use dioxus_primitives::drag_and_drop_list::use_drag_and_drop_list_items;

use super::add_menu::AddMapBar;
use super::map_card::{MapCard, heading_width};

const MAP_LIST_CSS: Asset = asset!("/assets/styling/map-list.css");
// Loaded here so it is ready before a card first opens
const MAP_FORM_CSS: Asset = asset!("/assets/styling/map-form.css");

/// The `datalist` of the labels of `data_type`, for the forms' label fields.
pub(crate) const fn labels_list(data_type: DataType) -> &'static str {
    match data_type {
        DataType::Bytes => "map-bytes-labels",
        DataType::Log => "map-log-labels",
        DataType::Number => "map-numbers-labels",
        DataType::Vector2d => "map-vector2d-labels",
        DataType::Vector3d => "map-vector3d-labels",
    }
}

/// An open card is not dragged: its form is being used, its text chosen by the
/// pointer. Its item made undraggable as the pointer is pressed on it, before a
/// drag would start (for all browsers: Chromium, but Firefox too); and a drag
/// of it still begun taken before the list hears of it. Once for the page.
const OPEN_CARD_UNDRAGGABLE_JS: &str = r#"
if (!window.servalOpenCardUndraggable) {
  window.servalOpenCardPressed = (event) => {
    const item = event.target.closest?.('.map-list li[aria-roledescription="sortable item"]');
    if (item) {
      item.draggable = !item.querySelector('.map-card[data-open="true"]');
    }
  };
  document.addEventListener("pointerdown", window.servalOpenCardPressed, true);
  window.servalOpenCardUndraggable = (event) => {
    const item = event.target.closest?.('.map-list li[aria-roledescription="sortable item"]');
    if (item?.querySelector('.map-card[data-open="true"]')) {
      event.preventDefault();
      event.stopPropagation();
    }
  };
  document.addEventListener("dragstart", window.servalOpenCardUndraggable, true);
}
"#;

/// Lists the maps for editing; they run in `MapProvider` regardless.
#[component]
pub fn MapList() -> Element {
    let context = use_context::<MapContext>();
    use_effect(|| {
        document::eval(OPEN_CARD_UNDRAGGABLE_JS);
    });
    let maps = use_memo(move || context.list());
    // Which maps there are, in no order
    let map_set = use_memo(move || {
        let mut ids: Vec<usize> = maps.read().iter().map(|map| map.id).collect();
        ids.sort_unstable();
        ids
    });
    // Bumped to make the list anew with all the maps, as it left one out
    let generation = use_signal(|| 0_u32);
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
            for data_type in DataType::ALL {
                LabelsList { data_type }
            }
        }
    }
}

/// The labels of `data_type` there are, as `labels_list` names them.
#[component]
fn LabelsList(data_type: DataType) -> Element {
    let data_context = use_context::<DataContext>();
    let labels = use_memo(move || data_context.labels_of(data_type));

    rsx! {
        datalist {
            id: labels_list(data_type),
            for label in labels() {
                option { value: "{label}" }
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
