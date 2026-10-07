//! The fields of the maps' settings forms, styled by `.field` (`theme.css`).

use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::components::input::Input;

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
/// it clears `error`, if given, as what it was about has changed.
#[component]
pub(super) fn TextField(
    name: String,
    value: Signal<String>,
    #[props(default)] placeholder: Option<String>,
    #[props(default = VNode::empty())] icon: Element,
    #[props(default)] error: Option<Signal<Option<String>>>,
) -> Element {
    let mut value = value;

    rsx! {
        label {
            class: "field",
            span { class: "field-label", "{name}" }
            {icon}
            Input {
                placeholder,
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
