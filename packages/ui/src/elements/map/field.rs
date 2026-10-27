//! The fields of the maps' forms; read-only while the map is on.

use std::collections::HashSet;

use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::components::{
    input::Input,
    popover::{PopoverContent, PopoverRoot, PopoverTrigger},
    toggle_group::{ToggleGroup, ToggleItem},
};
use crate::data::LogLevel;
use crate::elements::LogLevelTag;
use dioxus_primitives::{ContentAlign, ContentSide};

/// Whether the map of the form is on, provided by its card.
#[derive(Clone, Copy)]
pub(super) struct MapEnabled(pub ReadSignal<bool>);

fn use_map_enabled() -> bool {
    try_use_context::<MapEnabled>().is_some_and(|MapEnabled(enabled)| enabled())
}

/// The placeholder of a field of a Number label or a number.
pub(super) const NUMBER_PLACEHOLDER: &str = "Label, number or #pi";

/// What a field of a Number label or a number takes, on hovering it.
pub(super) const NUMBER_HINT: &str = "A Number label, a number, or a constant: #pi, #e";

/// What a field of a vector label or a vector takes, on hovering it.
pub(super) const VECTOR_HINT: &str =
    "A vector label, or a vector as (x, y) or (x, y, z), of numbers or constants (#pi, #e)";

/// A field for a label, offering those in the `datalist` with id `list`; for an
/// input of logs, the weakest level taken chosen beside it.
#[component]
pub(super) fn LabelField(
    value: Signal<String>,
    list: String,
    placeholder: String,
    #[props(default)] name: Option<String>,
    /// What it takes, on hovering it.
    #[props(default)]
    hint: Option<String>,
    #[props(default)] weakest_level: Option<Signal<LogLevel>>,
) -> Element {
    let mut value = value;
    let locked = use_map_enabled();

    rsx! {
        label {
            class: "field",
            "data-locked": locked,
            title: hint,
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
            if let Some(weakest_level) = weakest_level {
                LevelFilter { weakest_level, locked }
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

/// A level as a filter down to it shows it: as a tag in its color; ALL for None,
/// the weakest.
#[component]
fn LevelChoice(level: LogLevel) -> Element {
    rsx! {
        if level == LogLevel::None {
            span { class: "field-level-all", "ALL" }
        } else {
            LogLevelTag { level }
        }
    }
}

/// The weakest level a map takes of an input: its choices side by side, as the
/// pointer is over it (or by a click).
#[component]
fn LevelFilter(weakest_level: Signal<LogLevel>, locked: bool) -> Element {
    let mut weakest_level = weakest_level;
    let mut open = use_signal(|| false);

    rsx! {
        span {
            class: "field-level",
            onmouseenter: move |_| open.set(!locked),
            onmouseleave: move |_| open.set(false),
            PopoverRoot {
                class: "field-level-popover",
                is_modal: false,
                open: Some(open()),
                on_open_change: move |value| open.set(value && !locked),
                PopoverTrigger {
                    class: "field-level-trigger",
                    aria_label: "Weakest level",
                    title: "The weakest level taken: those weaker are left out",
                    disabled: locked,
                    span { class: "field-label", "Filter by" }
                    LevelChoice { level: weakest_level() }
                }
                PopoverContent {
                    class: "field-level-content",
                    side: ContentSide::Bottom,
                    // To the right, over the form: to the left, it would go past the map
                    // list, which clips it
                    align: ContentAlign::Start,
                    div {
                        class: "field-level-choices",
                        role: "listbox",
                        aria_label: "Weakest level",
                        for level in LogLevel::ALL {
                            button {
                                class: "field-level-choice",
                                r#type: "button",
                                role: "option",
                                aria_selected: level == weakest_level(),
                                onclick: move |_| {
                                    weakest_level.set(level);
                                    open.set(false);
                                },
                                LevelChoice { level }
                            }
                        }
                    }
                }
            }
        }
    }
}
