use std::{cell::RefCell, collections::HashMap, rc::Rc};

use dioxus::prelude::*;

use super::{GraphContext, GraphFrame, GraphKind};
use crate::components::tag_group::{Tag, TagGroupEmpty, TagGroupLabel, TagGroupMulti, TagList};
use crate::data::{DataContext, SourceCursor, TypedData};

const TIME_SERIES_CSS: Asset = asset!("/assets/styling/time-series.css");
const UPLOT_CSS: Asset = asset!("/assets/vendor/uplot/uPlot.min.css");
const UPLOT_JS: Asset = asset!(
    "/assets/vendor/uplot/uPlot.iife.min.js",
    AssetOptions::js().with_minify(false)
);
/// The data handling, then the plot, which uses it (see both files)
const TIME_SERIES_JS: &str = concat!(
    include_str!("time_series_data.js"),
    include_str!("time_series.js"),
);

pub const TIME_SERIES: GraphKind = GraphKind {
    name: "Time series",
    view: |id| rsx! { TimeSeriesGraph { id } },
};

/// One label's update for `time_series.js`: `(label, reset, [(timestamp_ms, value)])`.
type Update = (String, bool, Vec<(i64, f64)>);

/// Feeds the plot only the points it does not have yet.
#[derive(Default)]
struct PlotFeed {
    /// How far the plot has each selected label.
    cursors: HashMap<String, SourceCursor>,
}

impl PlotFeed {
    /// The next message for `time_series.js`: the selected labels that hold
    /// numbers, and an update for each one the plot is behind on. A label
    /// newly selected, or whose queue was cleared or replaced, is sent again in
    /// full, as a reset.
    fn next(
        &mut self,
        data: &HashMap<String, TypedData>,
        selected: &[String],
    ) -> (Vec<String>, Vec<Update>) {
        let mut labels = Vec::new();
        let mut updates = Vec::new();

        for (label, data) in data {
            let TypedData::Number(queue) = data else {
                continue;
            };
            if !selected.contains(label) {
                continue;
            }
            labels.push(label.clone());

            let read = self.cursors.entry(label.clone()).or_default().read(queue);
            if !read.restarted && read.entries.is_empty() {
                continue;
            }
            let points = read
                .entries
                .iter()
                .map(|data| (data.timestamp(), *data.value()))
                .collect();
            updates.push((label.clone(), read.restarted, points));
        }
        // Forget deselected labels, so selecting one again sends it in full
        self.cursors.retain(|label, _| labels.contains(label));

        (labels, updates)
    }
}

/// Plots the `Number` labels chosen in its settings against time, for the graph
/// with `id` in `GraphContext`.
#[component]
pub fn TimeSeriesGraph(id: usize) -> Element {
    let data_context = use_context::<DataContext>();
    let mut graph_context = use_context::<GraphContext>();
    // Graph ids are never reused, so this is unique on the page
    let container_id = format!("graph-plot-{id}");
    let plot = use_hook(|| document::eval(TIME_SERIES_JS));
    let feed = use_hook(|| Rc::new(RefCell::new(PlotFeed::default())));

    // Only this graph's labels, so edits to other graphs do not re-run the plot
    let selected = use_memo(move || {
        graph_context
            .get(id)
            .map(|graph| graph.property.labels)
            .unwrap_or_default()
    });
    // `TagGroupMulti` takes the selection as an optional list
    let selected_values = use_memo(move || Some(selected()));

    let number_labels = use_memo({
        let data_context = data_context.clone();
        move || {
            let mut labels = data_context.with_data(|data| {
                data.iter()
                    .filter(|(_, data)| matches!(data, TypedData::Number(_)))
                    .map(|(label, _)| label.clone())
                    .collect::<Vec<_>>()
            });
            labels.sort();
            labels
        }
    });
    let plotted = use_memo(move || {
        let selected = selected.read();
        number_labels
            .read()
            .iter()
            .filter(|label| selected.contains(label))
            .count()
    });

    use_effect(move || {
        let selected = selected.read();
        let message = data_context.with_data(|data| feed.borrow_mut().next(data, &selected));
        let _ = plot.send(message);
    });

    use_drop(move || {
        let _ = plot.send(());
    });

    rsx! {
        document::Link { rel: "stylesheet", href: UPLOT_CSS }
        document::Link { rel: "stylesheet", href: TIME_SERIES_CSS }
        document::Script { src: UPLOT_JS }

        GraphFrame {
            id,
            settings: rsx! {
                TagGroupMulti {
                    values: selected_values,
                    on_values_change: move |labels| {
                        graph_context.update(id, |property| property.labels = labels);
                    },
                    TagGroupLabel { "Data" }
                    TagList {
                        TagGroupEmpty { "No number data yet" }
                        for (index, label) in number_labels().into_iter().enumerate() {
                            Tag {
                                key: "{label}",
                                index,
                                value: label.clone(),
                                "{label}"
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
                    "No number data yet. Add a conversion to a number to plot it."
                }
            } else if plotted() == 0 {
                p {
                    class: "graph-empty",
                    "Open the settings under the title to choose what to plot."
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, VecDeque};

    use super::PlotFeed;
    use crate::data::{NumberData, TypedData};

    fn numbers(points: &[(i64, f64)]) -> TypedData {
        TypedData::Number(
            points
                .iter()
                .map(|&(timestamp, value)| NumberData::new(timestamp, value))
                .collect::<VecDeque<_>>(),
        )
    }

    fn selected(labels: &[&str]) -> Vec<String> {
        labels.iter().map(|label| label.to_string()).collect()
    }

    #[test]
    fn sends_everything_first_then_only_new_points() {
        let mut feed = PlotFeed::default();
        let mut data = HashMap::from([("temp".to_string(), numbers(&[(1, 1.0), (2, 2.0)]))]);

        let (labels, updates) = feed.next(&data, &selected(&["temp"]));
        assert_eq!(labels, ["temp"]);
        assert_eq!(
            updates,
            [("temp".to_string(), true, vec![(1, 1.0), (2, 2.0)])]
        );

        data.insert("temp".to_string(), numbers(&[(1, 1.0), (2, 2.0), (3, 3.0)]));
        let (_, updates) = feed.next(&data, &selected(&["temp"]));
        assert_eq!(updates, [("temp".to_string(), false, vec![(3, 3.0)])]);

        let (labels, updates) = feed.next(&data, &selected(&["temp"]));
        assert_eq!(labels, ["temp"]);
        assert!(updates.is_empty());
    }

    #[test]
    fn resends_a_cleared_queue_as_a_reset() {
        let mut feed = PlotFeed::default();
        let mut data = HashMap::from([("temp".to_string(), numbers(&[(1, 1.0), (2, 2.0)]))]);
        feed.next(&data, &selected(&["temp"]));

        // Cleared and refilled past its old length: only the first timestamp tells
        data.insert("temp".to_string(), numbers(&[(5, 5.0), (6, 6.0), (7, 7.0)]));
        let (_, updates) = feed.next(&data, &selected(&["temp"]));
        assert_eq!(
            updates,
            [("temp".to_string(), true, vec![(5, 5.0), (6, 6.0), (7, 7.0)])]
        );
    }

    #[test]
    fn plots_only_selected_number_labels() {
        let mut feed = PlotFeed::default();
        let data = HashMap::from([
            ("temp".to_string(), numbers(&[(1, 1.0)])),
            ("volt".to_string(), numbers(&[(1, 3.3)])),
            ("raw_str".to_string(), TypedData::String(VecDeque::new())),
        ]);

        let (labels, _) = feed.next(&data, &selected(&["volt", "raw_str"]));
        assert_eq!(labels, ["volt"]);

        // Deselected, then selected again: sent in full once more
        feed.next(&data, &selected(&[]));
        let (_, updates) = feed.next(&data, &selected(&["volt"]));
        assert_eq!(updates, [("volt".to_string(), true, vec![(1, 3.3)])]);
    }
}
