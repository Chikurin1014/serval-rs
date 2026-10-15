use dioxus::prelude::*;
use dioxus_icons::lucide;

use super::{
    AxisScale, GraphContext, GraphFrame, GraphKind, context::use_graph_setting,
    time_series::series_color,
};
use crate::components::{
    button::{Button, ButtonSize, ButtonVariant},
    input::Input,
    tag_group::{Tag, TagGroup, TagList},
    toggle::Toggle,
};
use crate::data::{DataContext, DataType, TypedData, format_number};

const TIME_SERIES_CSS: Asset = asset!("/assets/styling/time-series.css");
const UPLOT_CSS: Asset = asset!("/assets/vendor/uplot/uPlot.min.css");
const UPLOT_JS: Asset = asset!(
    "/assets/vendor/uplot/uPlot.iife.min.js",
    AssetOptions::js().with_minify(false)
);
const BOX_PLOT_JS: &str = include_str!("box_plot.js");

pub const BOX_PLOT: GraphKind = GraphKind {
    name: "Box plot",
    menu: "Statistics",
    view: |id| rsx! { BoxPlotGraph { id } },
    presets: &[],
};

/// How far past a quartile a whisker reaches, in interquartile ranges (as
/// Tukey's): the values further are outliers.
const WHISKER_IQRS: f64 = 1.5;

/// The values of a label summed up, as its box shows them.
#[derive(Clone, Debug, PartialEq)]
struct Summary {
    mean: f64,
    /// Over n − 1: none of one value.
    variance: Option<f64>,
    median: f64,
    max: f64,
    min: f64,
    q3: f64,
    q1: f64,
    /// The furthest values within [`WHISKER_IQRS`] of the box, below and above.
    whiskers: (f64, f64),
    /// The values past them, in order.
    outliers: Vec<f64>,
}

impl Summary {
    /// Of `values`, but those not finite; none if none are.
    fn of(values: impl IntoIterator<Item = f64>) -> Option<Self> {
        let mut sorted: Vec<f64> = values
            .into_iter()
            .filter(|value| value.is_finite())
            .collect();
        if sorted.is_empty() {
            return None;
        }
        sorted.sort_by(f64::total_cmp);
        let count = sorted.len() as f64;
        let mean = sorted.iter().sum::<f64>() / count;
        let variance = (sorted.len() > 1).then(|| {
            sorted
                .iter()
                .map(|value| (value - mean).powi(2))
                .sum::<f64>()
                / (count - 1.0)
        });
        let (q1, median, q3) = (
            quantile(&sorted, 0.25),
            quantile(&sorted, 0.5),
            quantile(&sorted, 0.75),
        );
        let reach = WHISKER_IQRS * (q3 - q1);
        let (low, high) = (q1 - reach, q3 + reach);
        let within = |value: &&f64| (low..=high).contains(*value);
        let whiskers = (
            *sorted.iter().find(within).unwrap_or(&q1),
            *sorted.iter().rev().find(within).unwrap_or(&q3),
        );
        let outliers = sorted
            .iter()
            .copied()
            .filter(|value| !(low..=high).contains(value))
            .collect();
        Some(Self {
            mean,
            variance,
            median,
            max: sorted[sorted.len() - 1],
            min: sorted[0],
            q3,
            q1,
            whiskers,
            outliers,
        })
    }

    /// As its hover shows them, in this order.
    fn rows(&self) -> [(&'static str, String); 7] {
        [
            ("Mean", format_number(self.mean)),
            (
                "Variance",
                self.variance.map_or_else(|| "—".to_string(), format_number),
            ),
            ("Median", format_number(self.median)),
            ("Max", format_number(self.max)),
            ("Min", format_number(self.min)),
            ("Q3", format_number(self.q3)),
            ("Q1", format_number(self.q1)),
        ]
    }
}

/// The `p` quantile of `sorted` (not empty), between its values by linear
/// interpolation: as NumPy's by default, and spreadsheets' `QUARTILE.INC`.
fn quantile(sorted: &[f64], p: f64) -> f64 {
    let position = p * (sorted.len() - 1) as f64;
    let below = position.floor() as usize;
    let above = (below + 1).min(sorted.len() - 1);
    sorted[below] + (position - below as f64) * (sorted[above] - sorted[below])
}

/// `(label, color, [min, whisker below, q1, median, q3, whisker above, max],
/// outliers, hover rows)`, as `box_plot.js` takes each box.
type PlotBox = (
    String,
    &'static str,
    [f64; 7],
    Vec<f64>,
    [(&'static str, String); 7],
);

/// `labels` with `text` (trimmed) after them, unless it is empty or one of them.
fn with_label(labels: &[String], text: &str) -> Option<Vec<String>> {
    let label = text.trim();
    if label.is_empty() || labels.iter().any(|each| each == label) {
        return None;
    }
    let mut labels = labels.to_vec();
    labels.push(label.to_string());
    Some(labels)
}

/// A box for each Number label chosen in its settings, of all its values.
#[component]
pub fn BoxPlotGraph(id: usize) -> Element {
    let data_context = use_context::<DataContext>();
    let mut graph_context = use_context::<GraphContext>();
    let value_scale = use_graph_setting(graph_context, id, |property| property.value_scale);
    let labels = use_graph_setting(graph_context, id, |property| property.labels.clone());
    let mut typed = use_signal(String::new);
    let container_id = format!("graph-plot-{id}");
    let labels_list_id = format!("graph-number-labels-{id}");
    let plot = use_hook(|| document::eval(BOX_PLOT_JS));

    let number_labels = use_memo(move || data_context.labels_of(DataType::Number));
    let mut add = move || {
        if let Some(next) = with_label(&labels.read(), &typed.read()) {
            graph_context.update(id, |property| property.labels = next);
        }
        typed.set(String::new());
    };

    use_effect(move || {
        // Colored by their place among the number labels, as the lines of a time series
        let colors = number_labels.read().clone();
        let boxes: Vec<PlotBox> = labels
            .read()
            .iter()
            .filter_map(|label| {
                let summary = data_context
                    .with_label(label, |data| match data {
                        TypedData::Number(queue) => {
                            Summary::of(queue.iter().map(|entry| *entry.value()))
                        }
                        _ => None,
                    })
                    .flatten()?;
                let index = colors.iter().position(|each| each == label).unwrap_or(0);
                let (low, high) = summary.whiskers;
                Some((
                    label.clone(),
                    series_color(index),
                    [
                        summary.min,
                        low,
                        summary.q1,
                        summary.median,
                        summary.q3,
                        high,
                        summary.max,
                    ],
                    summary.outliers.clone(),
                    summary.rows(),
                ))
            })
            .collect();
        let _ = plot.send((boxes, value_scale() == AxisScale::Log));
    });

    use_drop(move || {
        let _ = plot.send(());
    });

    let empty = if number_labels.read().is_empty() {
        Some("No number data yet. Add a map to a number to plot it.")
    } else if labels.read().is_empty() {
        Some("Add the Number labels to plot in its settings.")
    } else if !labels
        .read()
        .iter()
        .any(|label| number_labels.read().contains(label))
    {
        Some("No number data of these labels yet.")
    } else {
        None
    };

    rsx! {
        document::Link { rel: "stylesheet", href: UPLOT_CSS }
        document::Link { rel: "stylesheet", href: TIME_SERIES_CSS }
        document::Script { src: UPLOT_JS }

        GraphFrame {
            id,
            settings: rsx! {
                div {
                    class: "graph-setting",
                    span { class: "graph-setting-label", "Labels" }
                    div {
                        class: "graph-labels",
                        div {
                            class: "graph-labels-input",
                            Input {
                                placeholder: "Number label",
                                aria_label: "Label to plot",
                                list: "{labels_list_id}",
                                value: "{typed}",
                                oninput: move |event: FormEvent| typed.set(event.value()),
                                onkeydown: move |event: KeyboardEvent| {
                                    if event.key() == Key::Enter {
                                        add();
                                    }
                                },
                            }
                            Button {
                                variant: ButtonVariant::Outline,
                                size: ButtonSize::IconSm,
                                aria_label: "Add label",
                                title: "Add label",
                                onclick: move |_| add(),
                                lucide::Plus {}
                            }
                        }
                        // The tag group takes no class
                        div {
                            class: "graph-labels-tags",
                            TagGroup {
                                selectable: false,
                                aria_label: "Labels plotted",
                                TagList {
                                    for (index, label) in labels().into_iter().enumerate() {
                                        Tag {
                                            key: "{label}",
                                            index,
                                            value: label.clone(),
                                            span { "{label}" }
                                            Button {
                                                class: "graph-labels-remove",
                                                variant: ButtonVariant::Ghost,
                                                size: ButtonSize::IconXs,
                                                aria_label: "Remove label {label}",
                                                title: "Remove label",
                                                onclick: move |event: MouseEvent| {
                                                    event.stop_propagation();
                                                    graph_context.update(id, |property| {
                                                        property.labels.retain(|each| *each != label);
                                                    });
                                                },
                                                lucide::X {}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div {
                    class: "graph-setting",
                    span { class: "graph-setting-label", "Value axis" }
                    div {
                        class: "graph-scale",
                        role: "group",
                        aria_label: "Value axis",
                        for scale in AxisScale::ALL {
                            Toggle {
                                pressed: Some(value_scale() == scale),
                                on_pressed_change: move |_| {
                                    graph_context.update(id, |property| property.value_scale = scale);
                                },
                                "{scale.name()}"
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

#[cfg(test)]
mod tests {
    use super::{Summary, quantile, with_label};

    #[test]
    fn quantiles_go_between_values_linearly() {
        let sorted = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(quantile(&sorted, 0.0), 1.0);
        assert_eq!(quantile(&sorted, 0.25), 1.75);
        assert_eq!(quantile(&sorted, 0.5), 2.5);
        assert_eq!(quantile(&sorted, 0.75), 3.25);
        assert_eq!(quantile(&sorted, 1.0), 4.0);
        assert_eq!(quantile(&[5.0], 0.25), 5.0);
    }

    #[test]
    fn values_past_the_whiskers_are_outliers() {
        // In any order
        let summary = Summary::of([4.0, 100.0, 2.0, -50.0, 3.0, 1.0, 2.5]).unwrap();
        assert_eq!((summary.q1, summary.median, summary.q3), (1.5, 2.5, 3.5));
        // Within 1.5 × 2 of the box: −1.5 to 6.5
        assert_eq!(summary.whiskers, (1.0, 4.0));
        assert_eq!(summary.outliers, [-50.0, 100.0]);
        assert_eq!((summary.min, summary.max), (-50.0, 100.0));
    }

    #[test]
    fn rows_are_in_order_with_the_sample_variance() {
        let summary = Summary::of([1.0, 2.0, 3.0, 4.0, 100.0]).unwrap();
        let rows = summary.rows();
        assert_eq!(
            rows.clone().map(|(name, _)| name),
            ["Mean", "Variance", "Median", "Max", "Min", "Q3", "Q1"]
        );
        // Over n − 1: (21² + 20² + 19² + 18² + 78²) / 4
        assert_eq!(
            rows.map(|(_, value)| value),
            [
                "22.000", "1902.5", "3.0000", "100.00", "1.0000", "4.0000", "2.0000"
            ]
        );
    }

    #[test]
    fn one_value_has_no_variance_and_none_no_summary() {
        let summary = Summary::of([7.0]).unwrap();
        assert_eq!(summary.variance, None);
        assert_eq!(summary.rows()[1].1, "—");
        assert_eq!((summary.whiskers, summary.outliers.len()), ((7.0, 7.0), 0));
        assert_eq!(Summary::of([f64::NAN]), None);
        assert_eq!(Summary::of([]), None);
    }

    #[test]
    fn labels_are_added_once_each_in_order() {
        let labels = with_label(&[], " temp ").unwrap();
        assert_eq!(labels, ["temp"]);
        let labels = with_label(&labels, "volt").unwrap();
        assert_eq!(labels, ["temp", "volt"]);
        assert_eq!(with_label(&labels, "temp"), None);
        assert_eq!(with_label(&labels, "  "), None);
    }
}
