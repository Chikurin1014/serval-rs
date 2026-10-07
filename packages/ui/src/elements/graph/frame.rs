use dioxus::prelude::*;
use dioxus_icons::lucide;

use super::{GraphContext, board::GraphDrag};
use crate::components::{
    button::{Button, ButtonSize, ButtonVariant},
    card::{Card, CardContent, CardFooter},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    input::Input,
};

const GRAPH_FRAME_CSS: Asset = asset!("/assets/styling/graph-frame.css");

/// The part of a graph that does not depend on its kind, for the graph with
/// `id` in `GraphContext`: a remove button over the plot, a handle to move it
/// (on `GraphBoard`), and the title under it, which opens the settings (the
/// title, then `settings`).
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

    // Reordering, when on the board. The card is only draggable while its
    // handle is held, so dragging in the plot still zooms and text still selects
    let drag = try_use_context::<GraphDrag>();
    let mut handle_held = use_signal(|| false);
    let handle_id = format!("graph-handle-{id}");
    let is_dragging = drag.is_some_and(|drag| (drag.dragging)() == Some(id));
    let is_drop_target = drag.is_some_and(|drag| (drag.target)() == Some(id));
    let mut end_drag = move || {
        handle_held.set(false);
        if let Some(GraphDrag {
            mut dragging,
            mut target,
        }) = drag
        {
            dragging.set(None);
            target.set(None);
        }
    };

    rsx! {
        document::Link { rel: "stylesheet", href: GRAPH_FRAME_CSS }

        div {
            class: "graph reveals",
            draggable: handle_held(),
            "data-dragging": is_dragging,
            "data-drop-target": is_drop_target,
            ondragstart: move |event: DragEvent| {
                let Some(GraphDrag { mut dragging, .. }) = drag.filter(|_| handle_held()) else {
                    return;
                };
                // Firefox only starts a drag that carries some data
                let transfer = event.data_transfer();
                let _ = transfer.set_data("text/plain", &id.to_string());
                transfer.set_effect_allowed("move");
                dragging.set(Some(id));
            },
            ondragover: move |event: DragEvent| {
                let Some(GraphDrag { dragging, mut target }) = drag else {
                    return;
                };
                if dragging().is_some_and(|from| from != id) {
                    // Accept the drop here
                    event.prevent_default();
                    event.data_transfer().set_drop_effect("move");
                    if *target.peek() != Some(id) {
                        target.set(Some(id));
                    }
                }
            },
            ondragleave: move |_| {
                if let Some(GraphDrag { mut target, .. }) = drag {
                    if *target.peek() == Some(id) {
                        target.set(None);
                    }
                }
            },
            ondrop: move |event: DragEvent| {
                event.prevent_default();
                let from = drag.and_then(|drag| *drag.dragging.peek());
                // The dragged graph takes this one's place
                if let (Some(from), Some(position)) = (from, graph_context.position(id)) {
                    graph_context.move_to(from, position);
                }
                end_drag();
            },
            ondragend: move |_| end_drag(),
            if drag.is_some() {
                // Over the top left corner of the plot, shown while the graph is hovered
                Button {
                    class: "graph-handle reveal-on-hover",
                    id: "{handle_id}",
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::IconSm,
                    aria_label: "Move graph",
                    title: "Drag to move, or use the arrow keys",
                    onmousedown: move |_| handle_held.set(true),
                    onmouseup: move |_| handle_held.set(false),
                    onkeydown: {
                        let handle_id = handle_id.clone();
                        move |event: KeyboardEvent| {
                            let step: isize = match event.key() {
                                Key::ArrowLeft | Key::ArrowUp => -1,
                                Key::ArrowRight | Key::ArrowDown => 1,
                                _ => return,
                            };
                            event.prevent_default();
                            let Some(position) = graph_context.position(id) else {
                                return;
                            };
                            let Some(to) = position.checked_add_signed(step) else {
                                return;
                            };
                            graph_context.move_to(id, to);
                            // Moving the card in the page drops focus from the handle
                            document::eval(&format!(
                                "requestAnimationFrame(() => document.getElementById('{handle_id}')?.focus())"
                            ));
                        }
                    },
                    lucide::GripVertical {}
                }
            }
            // Over the top right corner of the plot, shown while the graph is hovered
            Button {
                class: "graph-remove reveal-on-hover",
                variant: ButtonVariant::Ghost,
                size: ButtonSize::IconSm,
                aria_label: "Remove graph",
                title: "Remove graph",
                onclick: move |_| graph_context.remove(id),
                lucide::X {}
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
