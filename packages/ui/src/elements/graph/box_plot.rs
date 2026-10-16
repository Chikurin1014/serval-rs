use dioxus::prelude::*;

use super::{
    AxisScale, GraphContext, GraphFrame, GraphKind,
    context::{until_paused, use_graph_setting},
    plot::{PlotAssets, label_color, use_plot, use_shown_count},
};
use crate::components::toggle::Toggle;
use crate::data::{DataContext, DataType, TypedData, format_number};
use crate::elements::FilterContext;

const BOX_PLOT_JS: &str = concat!(include_str!("paced_draws.js"), include_str!("box_plot.js"));

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

/// A box for each Number label the data list shows, of all its values; each
/// turned on or off in the legend.
#[component]
pub fn BoxPlotGraph(id: usize) -> Element {
    let data_context = use_context::<DataContext>();
    let filter_context = use_context::<FilterContext>();
    let mut graph_context = use_context::<GraphContext>();
    let value_scale = use_graph_setting(graph_context, id, |property| property.value_scale);
    let hidden = use_graph_setting(graph_context, id, |property| property.hidden.clone());
    let container_id = format!("graph-plot-{id}");
    let plot = use_plot(id, BOX_PLOT_JS);

    let number_labels = use_memo(move || data_context.labels_of(DataType::Number));
    let plotted = use_shown_count(number_labels);

    use_effect(move || {
        let paused_at = graph_context.paused_at();
        // Colored by their place among the number labels, as the lines of a time series
        let colors = number_labels.read().clone();
        let boxes: Vec<PlotBox> = data_context.with_each(Some(DataType::Number), |data| {
            data.iter()
                .filter(|(label, _)| filter_context.shows(label))
                .filter_map(|&(label, data)| {
                    let TypedData::Number(queue) = data else {
                        return None;
                    };
                    let summary = Summary::of(
                        queue
                            .iter()
                            .filter(|entry| until_paused(paused_at, entry.timestamp()))
                            .map(|entry| *entry.value()),
                    )?;
                    let (low, high) = summary.whiskers;
                    Some((
                        label.to_string(),
                        label_color(&colors, label),
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
                .collect()
        });
        let _ = plot.send((boxes, value_scale() == AxisScale::Log, hidden()));
    });

    rsx! {
        PlotAssets { uplot: true }

        GraphFrame {
            id,
            settings: rsx! {
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
            if number_labels.read().is_empty() {
                p {
                    class: "graph-empty",
                    "No number data yet. Add a map to a number to plot it."
                }
            } else if plotted() == 0 {
                p {
                    class: "graph-empty",
                    "No number data shown. Change the Data list's filters to show some."
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Summary, quantile};

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
}
