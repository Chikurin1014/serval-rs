//! The fields of the maps' forms; read-only while the map is on.

use dioxus::prelude::*;
use dioxus_icons::lucide;
use dioxus_primitives::ContentSide;

use crate::components::{
    input::Input,
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
};

/// Whether the map of the form is on, provided by its card.
#[derive(Clone, Copy)]
pub(super) struct MapEnabled(pub ReadSignal<bool>);

fn use_map_enabled() -> bool {
    try_use_context::<MapEnabled>().is_some_and(|MapEnabled(enabled)| enabled())
}

#[component]
fn LockTooltip(locked: bool, field: Element) -> Element {
    rsx! {
        Tooltip {
            class: "field-lock",
            disabled: !locked,
            TooltipTrigger {
                r#as: move |attributes: Vec<Attribute>| rsx! {
                    // Classed by the tooltip; styled as `.field-lock > div`
                    div { ..attributes, {field.clone()} }
                },
            }
            TooltipContent { side: ContentSide::Top, "Turn this Map off to edit" }
        }
    }
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
