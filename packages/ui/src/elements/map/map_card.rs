//! A map's card in the list: its kind, switch and latest conversion, opened to
//! its form.

use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::components::{
    button::{Button, ButtonSize, ButtonVariant},
    card::{Card, CardContent, CardHeader},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    switch::Switch,
};
use crate::data::{Conversion, DataType, Map, MapContext, MapKind, Segment};
use crate::elements::LogLevelTag;
use crate::helper::single_line;

use super::field::MapEnabled;

#[component]
pub(super) fn MapCard(map: Map) -> Element {
    let mut context = use_context::<MapContext>();
    let Map {
        id,
        kind,
        mut enabled,
        mut open,
        latest,
        output_error,
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
                            Latest { latest, output_error }
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
pub(super) fn heading_width(kind: &MapKind) -> f64 {
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

pub(super) fn type_names(types: &[DataType]) -> Vec<&'static str> {
    types.iter().map(DataType::name).collect()
}

/// `from -> to` of a kind of map.
#[component]
pub(super) fn MapTypes(from: Vec<&'static str>, to: &'static str) -> Element {
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

/// The latest conversion of a map, if any; why its output was refused over it.
#[component]
fn Latest(
    latest: ReadSignal<Option<Conversion>>,
    output_error: ReadSignal<Option<String>>,
) -> Element {
    rsx! {
        if let Some(error) = output_error() {
            span {
                class: "map-latest",
                "data-failed": true,
                title: "{error}",
                lucide::CircleAlert {}
                span { class: "map-latest-value", {single_line(&error)} }
            }
        } else if let Some(conversion) = latest() {
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
            if let Some(level) = segment.level {
                LogLevelTag { level }
            } else {
                span {
                    class: "map-segment",
                    "data-from-input": segment.from_input,
                    {single_line(&segment.text)}
                }
            }
        }
    }
}
