//! The fields of the maps' forms; read-only while the map is on.

use std::collections::HashSet;

use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::components::{
    input::Input,
    toggle_group::{ToggleGroup, ToggleItem},
};

/// Whether the map of the form is on, provided by its card.
#[derive(Clone, Copy)]
pub(super) struct MapEnabled(pub ReadSignal<bool>);

fn use_map_enabled() -> bool {
    try_use_context::<MapEnabled>().is_some_and(|MapEnabled(enabled)| enabled())
}

/// A field for a label, offering those in the `datalist` with id `list`.
#[component]
pub(super) fn LabelField(
    value: Signal<String>,
    list: String,
    placeholder: String,
    #[props(default)] name: Option<String>,
) -> Element {
    let mut value = value;
    let locked = use_map_enabled();

    rsx! {
        label {
            class: "field",
            "data-locked": locked,
            if let Some(name) = name {
                span { class: "field-label", "{name}" }
            }
            lucide::Tag {}
            Input {
                list,
                placeholder,
                autocomplete: "on",
                readonly: locked,
                value: "{value}",
                oninput: move |event: FormEvent| value.set(event.value()),
            }
        }
    }
}

/// A field for a setting; typing clears `error`.
#[component]
pub(super) fn TextField(
    name: String,
    value: Signal<String>,
    #[props(default)] placeholder: Option<String>,
    #[props(default = VNode::empty())] icon: Element,
    #[props(default)] error: Option<Signal<Option<String>>>,
) -> Element {
    let mut value = value;
    let locked = use_map_enabled();

    rsx! {
        label {
            class: "field",
            "data-locked": locked,
            span { class: "field-label", "{name}" }
            {icon}
            Input {
                placeholder,
                readonly: locked,
                value: "{value}",
                oninput: move |event: FormEvent| {
                    value.set(event.value());
                    if let Some(mut error) = error {
                        error.set(None);
                    }
                },
            }
        }
    }
}

/// A field for one or more of `options`, chosen by their indices: unchoosing
/// the last is ignored.
#[component]
pub(super) fn ChoicesField(
    name: String,
    options: Vec<String>,
    chosen: Vec<usize>,
    on_change: EventHandler<Vec<usize>>,
) -> Element {
    let locked = use_map_enabled();
    let pressed = chosen.into_iter().collect::<HashSet<_>>();

    rsx! {
        div {
            class: "field field-choices",
            "data-locked": locked,
            role: "group",
            aria_label: "{name}",
            span { class: "field-label", "{name}" }
            ToggleGroup {
                horizontal: true,
                allow_multiple_pressed: true,
                disabled: locked,
                pressed: Some(pressed),
                on_pressed_change: move |pressed: HashSet<usize>| {
                    if !pressed.is_empty() {
                        let mut chosen = pressed.into_iter().collect::<Vec<_>>();
                        chosen.sort_unstable();
                        on_change.call(chosen);
                    }
                },
                for (index, option) in options.iter().enumerate() {
                    ToggleItem { index, "{option}" }
                }
            }
        }
    }
}
