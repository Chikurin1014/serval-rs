//! The fields of the maps' settings forms, styled by `.field` (`theme.css`).
//!
//! A map's settings are fixed while it is on (it takes them in, e.g. compiles
//! its pattern, when turned on): its fields can then be read and copied but
//! not edited, and say so on hover.

use dioxus::prelude::*;
use dioxus_icons::lucide;
use dioxus_primitives::ContentSide;

use crate::components::{
    input::Input,
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
};

/// Whether the map whose form this is is on, provided by its card (`MapCard`).
#[derive(Clone, Copy)]
pub(super) struct MapEnabled(pub ReadSignal<bool>);

/// Whether the map of the form calling this is on (see [`MapEnabled`]), when
/// its fields are locked.
fn use_map_enabled() -> bool {
    try_use_context::<MapEnabled>().is_some_and(|MapEnabled(enabled)| enabled())
}

/// `field`, telling on hover while `locked` that it is so, and how to edit it.
#[component]
fn LockTooltip(locked: bool, field: Element) -> Element {
    rsx! {
        Tooltip {
            class: "field-lock",
            disabled: !locked,
            TooltipTrigger {
                r#as: move |attributes: Vec<Attribute>| rsx! {
                    // Classed by the tooltip (styled as `.field-lock > div`)
                    div { ..attributes, {field.clone()} }
                },
            }
            TooltipContent { side: ContentSide::Top, "Turn this Map off to edit" }
        }
    }
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
    let locked = use_map_enabled();

    rsx! {
        LockTooltip {
            locked,
            field: rsx! {
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
            },
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
    let locked = use_map_enabled();

    rsx! {
        LockTooltip {
            locked,
            field: rsx! {
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
            },
        }
    }
}
