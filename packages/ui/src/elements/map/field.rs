//! The fields of the maps' settings forms, styled by `.field` (`theme.css`).

use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::components::input::Input;

/// Whether the map whose form this is is on, provided by its card (`MapCard`):
/// a form locks what its map cannot take in while running, e.g. its pattern,
/// compiled when the map is turned on.
#[derive(Clone, Copy)]
pub(super) struct MapEnabled(pub ReadSignal<bool>);

/// Whether the map of the form calling this is on (see [`MapEnabled`]).
pub(super) fn use_map_enabled() -> bool {
    try_use_context::<MapEnabled>().is_some_and(|MapEnabled(enabled)| enabled())
}

/// A field for a label: an optional name (e.g. `a`), the tag icon, then an
/// input offering the labels in the `datalist` with id `list` (see `MapList`).
#[component]
pub(super) fn LabelField(
    value: Signal<String>,
    list: String,
    placeholder: String,
    #[props(default)] name: Option<String>,
) -> Element {
    let mut value = value;

    rsx! {
        label {
            class: "field",
            if let Some(name) = name {
                span { class: "field-label", "{name}" }
            }
            lucide::Tag {}
            Input {
                list,
                placeholder,
                autocomplete: "on",
                value: "{value}",
                oninput: move |event: FormEvent| value.set(event.value()),
            }
        }
    }
}

/// A field for a setting: its name, an optional icon, then an input. Typing in
/// it clears `error`, if given, as what it was about has changed. While
/// `locked`, it can be read and copied but not edited.
#[component]
pub(super) fn TextField(
    name: String,
    value: Signal<String>,
    #[props(default)] placeholder: Option<String>,
    #[props(default = VNode::empty())] icon: Element,
    #[props(default)] error: Option<Signal<Option<String>>>,
    #[props(default)] locked: bool,
) -> Element {
    let mut value = value;

    rsx! {
        label {
            class: "field",
            "data-locked": locked,
            title: if locked { "Turn the map off to edit" },
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
