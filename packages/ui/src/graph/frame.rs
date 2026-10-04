use dioxus::prelude::*;
use dioxus_free_icons::{icons::ld_icons::LdX, Icon};

use super::GraphContext;
use crate::components::{
    button::{Button, ButtonSize, ButtonVariant},
    card::{Card, CardContent, CardFooter},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    input::Input,
};

const GRAPH_FRAME_CSS: Asset = asset!("/assets/styling/graph-frame.css");

/// The part of a graph that does not depend on its kind, for the graph with
/// `id` in `GraphContext`: a remove button over the plot, and the title under
/// it, which opens the settings (the title, then `settings`).
///
/// `children` is the plot, filling the card above the title.
/// `settings` are label | control rows: a `label` holding a `.graph-setting-label`
/// span and a control, or anything that lays out its parts likewise.
#[component]
pub fn GraphFrame(
    id: usize,
    #[props(default = VNode::empty())] settings: Element,
    children: Element,
) -> Element {
    let mut graph_context = use_context::<GraphContext>();
    let custom_title = use_memo(move || {
        graph_context
            .get(id)
            .and_then(|graph| graph.property.title)
            .unwrap_or_default()
    });
    let default_title = use_memo(move || {
        let position = graph_context.position(id).unwrap_or_default();
        format!("Graph {}", position + 1)
    });
    let title = use_memo(move || {
        let custom = custom_title();
        if custom.trim().is_empty() {
            default_title()
        } else {
            custom
        }
    });

    rsx! {
        document::Link { rel: "stylesheet", href: GRAPH_FRAME_CSS }

        div {
            class: "graph",
            // Over the top right corner of the plot, shown while the graph is hovered
            Button {
                class: "graph-remove",
                variant: ButtonVariant::Ghost,
                size: ButtonSize::IconSm,
                aria_label: "Remove graph",
                title: "Remove graph",
                onclick: move |_| graph_context.remove(id),
                Icon { icon: LdX {} }
            }
            Card {
                CardContent {
                    {children}
                }
                CardFooter {
                    // The title opens the graph's settings below it
                    Collapsible {
                        class: "graph-settings",
                        CollapsibleTrigger {
                            class: "graph-title",
                            span { class: "graph-title-text", "{title}" }
                        }
                        CollapsibleContent {
                            // Label | control rows (see `.graph-settings-body`)
                            div {
                                class: "graph-settings-body",
                                label {
                                    span { class: "graph-setting-label", "Title" }
                                    Input {
                                        // Empty falls back to the numbered default
                                        placeholder: "{default_title}",
                                        value: "{custom_title}",
                                        oninput: move |event: FormEvent| {
                                            let value = event.value();
                                            graph_context.update(id, |property| {
                                                property.title = (!value.trim().is_empty()).then_some(value);
                                            });
                                        },
                                    }
                                }
                                {settings}
                            }
                        }
                    }
                }
            }
        }
    }
}
