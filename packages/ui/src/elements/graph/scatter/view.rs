//! The scatter plot itself: its settings, and the points, line and statistics
//! sent to `scatter.js`.

use std::{cell::RefCell, rc::Rc};

use dioxus::prelude::*;

use super::{Line, ScatterFeed, statistics_texts};
use crate::components::{input::Input, toggle::Toggle};
use crate::data::{DataContext, DataEntry, DataType, NumberData};
use crate::elements::Formula;
use crate::elements::graph::{
    AxisScale, GraphContext, GraphFrame,
    context::use_graph_setting,
    plot::{PlotAssets, plot_script, use_plot},
};
use crate::helper::set_if_changed;

const SCATTER_JS: &str = plot_script!("scatter.js");

/// Its points' color, as the first line of a time series.
const POINT_COLOR: &str = "--dc-accent";
/// Its regression line's, as the third.
const LINE_COLOR: &str = "--secondary-warning-color";

/// Plots the values of one Number label against those of another.
#[component]
pub fn ScatterGraph(id: usize) -> Element {
    let data_context = use_context::<DataContext>();
    let mut graph_context = use_context::<GraphContext>();
    // As typed, for the fields
    let x_text = use_graph_setting(graph_context, id, |property| property.x_label.clone());
    let y_text = use_graph_setting(graph_context, id, |property| property.y_label.clone());
    let x_label = use_memo(move || x_text.read().trim().to_string());
    let y_label = use_memo(move || y_text.read().trim().to_string());
    let x_scale = use_graph_setting(graph_context, id, |property| property.x_scale);
    let y_scale = use_graph_setting(graph_context, id, |property| property.value_scale);
    let regression = use_graph_setting(graph_context, id, |property| property.regression);
    let mut line = use_signal(|| None::<Line>);
    let container_id = format!("graph-plot-{id}");
    let labels_list_id = format!("graph-number-labels-{id}");
    let plot = use_plot(id, SCATTER_JS);
    let feed = use_hook(|| Rc::new(RefCell::new(ScatterFeed::default())));

    let number_labels = use_memo(move || data_context.labels_of(DataType::Number));

    use_effect(move || {
        let paused_at = graph_context.paused_at();
        let (x, y) = (x_label(), y_label());
        let labels = (x.as_str(), y.as_str());
        let mut feed = feed.borrow_mut();
        let update = data_context
            .with_label(&x, |x_data| {
                data_context.with_label(&y, |y_data| {
                    let (x, y) = (NumberData::queue(x_data), NumberData::queue(y_data));
                    feed.next(labels, x, y, paused_at)
                })
            })
            .flatten();
        let (reset, points) = update.unwrap_or_else(|| feed.next(labels, None, None, paused_at));
        let fitted = feed.line();
        set_if_changed(&mut line, fitted);
        let statistics = feed.statistics().map(statistics_texts);
        let shown = fitted
            .filter(|_| regression())
            .map(|line| (line.slope, line.intercept));
        let logs = (x_scale() == AxisScale::Log, y_scale() == AxisScale::Log);
        let colors = (POINT_COLOR, LINE_COLOR);
        let paused = paused_at.is_some();
        let _ = plot.send((
            reset,
            points,
            (&x, &y),
            logs,
            colors,
            shown,
            statistics,
            paused,
        ));
    });

    let empty = if number_labels.read().is_empty() {
        Some("No number data yet. Add a map to a number to plot it.")
    } else if x_label.read().is_empty() || y_label.read().is_empty() {
        Some("Choose the Number labels for X and Y in its settings.")
    } else if [x_label, y_label]
        .iter()
        .any(|label| !number_labels.read().contains(&label.read()))
    {
        Some("No number data of these labels yet.")
    } else {
        None
    };

    rsx! {
        PlotAssets { uplot: true }

        GraphFrame {
            id,
            settings: rsx! {
                for (axis, chosen) in [("X", x_text), ("Y", y_text)] {
                    label {
                        span { class: "graph-setting-label", "{axis}" }
                        Input {
                            placeholder: "Number label",
                            aria_label: "{axis} label",
                            list: "{labels_list_id}",
                            value: "{chosen}",
                            oninput: move |event: FormEvent| {
                                let value = event.value();
                                graph_context.update(id, |property| {
                                    if axis == "X" {
                                        property.x_label = value;
                                    } else {
                                        property.y_label = value;
                                    }
                                });
                            },
                        }
                    }
                }
                for (axis, scale) in [("X", x_scale), ("Y", y_scale)] {
                    div {
                        class: "graph-setting",
                        span { class: "graph-setting-label", "{axis} axis" }
                        div {
                            class: "graph-scale",
                            role: "group",
                            aria_label: "{axis} axis",
                            for choice in AxisScale::ALL {
                                Toggle {
                                    pressed: Some(scale() == choice),
                                    on_pressed_change: move |_| {
                                        graph_context.update(id, |property| {
                                            if axis == "X" {
                                                property.x_scale = choice;
                                            } else {
                                                property.value_scale = choice;
                                            }
                                        });
                                    },
                                    "{choice.name()}"
                                }
                            }
                        }
                    }
                }
                div {
                    class: "graph-setting",
                    span { class: "graph-setting-label", "Regression" }
                    div {
                        class: "graph-regression",
                        div {
                            class: "graph-scale",
                            role: "group",
                            aria_label: "Regression",
                            Toggle {
                                pressed: Some(regression()),
                                on_pressed_change: move |pressed: bool| {
                                    graph_context.update(id, |property| property.regression = pressed);
                                },
                                "Line"
                            }
                        }
                        if regression() {
                            if let Some(fitted) = line() {
                                Equation { line: fitted }
                            } else {
                                span { class: "graph-equation", "Not enough points" }
                            }
                        }
                    }
                }
                datalist {
                    id: "{labels_list_id}",
                    for label in number_labels() {
                        option { value: "{label}" }
                    }
                }
            },
            div {
                class: "graph-plot",
                id: "{container_id}",
                onmounted: {
                    let container_id = container_id.clone();
                    move |_| {
                        let _ = plot.send(container_id.clone());
                    }
                },
            }
            if let Some(empty) = empty {
                p { class: "graph-empty", "{empty}" }
            }
        }
    }
}

/// `line`'s equation: what stays as it is by KaTeX, its numbers as text (so
/// they change without it).
#[component]
fn Equation(line: Line) -> Element {
    let (slope, below, intercept) = line.terms();
    // KaTeX's for the operator, and the term before it: with `{}` after, so
    // as a binary one (or relation), spaced on both sides
    let (latex, fallback) = if below {
        ("X -{}", "X −")
    } else {
        ("X +{}", "X +")
    };

    rsx! {
        span {
            class: "graph-equation",
            role: "math",
            aria_label: "{line.equation()}",
            Formula {
                aria_hidden: "true",
                latex: "Y ={}".to_string(),
                fallback: "Y =".to_string(),
            }
            span { aria_hidden: "true", "{slope}" }
            Formula {
                aria_hidden: "true",
                latex: latex.to_string(),
                fallback: fallback.to_string(),
            }
            span { aria_hidden: "true", "{intercept}" }
        }
    }
}
