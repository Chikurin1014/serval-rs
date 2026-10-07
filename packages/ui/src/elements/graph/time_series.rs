use std::{cell::RefCell, collections::HashMap, rc::Rc};

use dioxus::prelude::*;

use super::{
    AxisScale, DrawStyle, GraphContext, GraphFrame, GraphKind, GraphPreset, GraphProperty,
    TimeWindow,
};
use crate::components::{slider::Slider, toggle::Toggle};
use crate::data::{DataContext, DataType, SourceCursor, TypedData};
use crate::elements::FilterContext;

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

/// Line colors, as CSS custom properties. A label takes the one at its place
/// among the number labels, which its tag shows too, so tags and lines match.
const SERIES_COLORS: [&str; 5] = [
    "--dc-accent",
    "--focused-border-color",
    "--secondary-warning-color",
    "--dc-danger",
    "--secondary-color-5",
];

fn series_color(index: usize) -> &'static str {
    SERIES_COLORS[index % SERIES_COLORS.len()]
}

pub const TIME_SERIES: GraphKind = GraphKind {
    name: "Time series",
    view: |id| rsx! { TimeSeriesGraph { id } },
    // One for each way of drawing the values
    presets: &[
        GraphPreset {
            name: "Points",
            property: || with_draw_style(DrawStyle::Points),
        },
        GraphPreset {
            name: "Linear",
            property: || with_draw_style(DrawStyle::Linear),
        },
        GraphPreset {
            name: "Stepped",
            property: || with_draw_style(DrawStyle::Stepped),
        },
    ],
};

fn with_draw_style(draw_style: DrawStyle) -> GraphProperty {
    GraphProperty {
        draw_style,
        ..GraphProperty::default()
    }
}

/// One label's update for `time_series.js`: `(label, reset, [(timestamp_ms, value)])`.
type Update = (String, bool, Vec<(i64, f64)>);

/// Feeds the plot only the points it does not have yet.
#[derive(Default)]
struct PlotFeed {
    /// How far the plot has each label it shows.
    cursors: HashMap<String, SourceCursor>,
}

impl PlotFeed {
    /// The next message for `time_series.js`: the labels that hold numbers and
    /// `shows` lets through, and an update for each one the plot is behind on. A
    /// label newly shown, or whose queue was cleared or replaced, is sent again
    /// in full, as a reset.
    fn next(
        &mut self,
        data: &[(&str, &TypedData)],
        shows: impl Fn(&str) -> bool,
    ) -> (Vec<String>, Vec<Update>) {
        let mut labels = Vec::new();
        let mut updates = Vec::new();

        for &(label, data) in data {
            let TypedData::Number(queue) = data else {
                continue;
            };
            if !shows(label) {
                continue;
            }
            labels.push(label.to_string());

            let read = self
                .cursors
                .entry(label.to_string())
                .or_default()
                .read(queue);
            if !read.restarted && read.entries.is_empty() {
                continue;
            }
            let points = read
                .entries
                .iter()
                .map(|data| (data.timestamp(), *data.value()))
                .collect();
            updates.push((label.to_string(), read.restarted, points));
        }
        // Forget the labels no longer shown, so showing one again sends it in full
        self.cursors.retain(|label, _| labels.contains(label));

        (labels, updates)
    }
}

/// Plots against time the `Number` labels the data list shows (by
/// `FilterContext`), for the graph with `id` in `GraphContext`.
#[component]
pub fn TimeSeriesGraph(id: usize) -> Element {
    let data_context = use_context::<DataContext>();
    let filter_context = use_context::<FilterContext>();
    let mut graph_context = use_context::<GraphContext>();
    // Only this graph's, so edits to other graphs do not re-run the plot
    let value_scale = use_memo(move || {
        graph_context
            .get(id)
            .map(|graph| graph.property.value_scale)
            .unwrap_or_default()
    });
    let draw_style = use_memo(move || {
        graph_context
            .get(id)
            .map(|graph| graph.property.draw_style)
            .unwrap_or_default()
    });
    let time_window = use_memo(move || {
        graph_context
            .get(id)
            .map(|graph| graph.property.time_window)
            .unwrap_or_default()
    });
    let hidden = use_memo(move || {
        graph_context
            .get(id)
            .map(|graph| graph.property.hidden)
            .unwrap_or_default()
    });
    // Graph ids are never reused, so this is unique on the page
    let container_id = format!("graph-plot-{id}");
    let plot = use_hook(|| document::eval(TIME_SERIES_JS));
    let feed = use_hook(|| Rc::new(RefCell::new(PlotFeed::default())));

    let number_labels = use_memo(move || data_context.labels_of(DataType::Number));
    let plotted = use_memo(move || {
        number_labels
            .read()
            .iter()
            .filter(|label| filter_context.shows(label))
            .count()
    });

    use_effect(move || {
        // Only the Number labels' writes run this again
        let (labels, updates) = data_context.with_each(Some(DataType::Number), |data| {
            feed.borrow_mut()
                .next(data, |label| filter_context.shows(label))
        });
        let colors = number_labels
            .read()
            .iter()
            .enumerate()
            .map(|(index, label)| (label.clone(), series_color(index)))
            .collect::<HashMap<_, _>>();
        let log = value_scale() == AxisScale::Log;
        let window = (time_window().seconds(), time_window().fits_data());
        let _ = plot.send((
            labels,
            updates,
            colors,
            log,
            draw_style().name(),
            hidden(),
            window,
        ));
    });

    // The labels turned off or on in the legend, kept in `GraphContext`
    use_future(move || async move {
        let mut plot = plot;
        while let Ok(labels) = plot.recv::<Vec<String>>().await {
            if graph_context
                .get(id)
                .is_some_and(|graph| graph.property.hidden != labels)
            {
                graph_context.update(id, |property| property.hidden = labels);
            }
        }
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
                div {
                    class: "graph-setting",
                    span { class: "graph-setting-label", "Draw" }
                    div {
                        class: "graph-scale",
                        role: "group",
                        aria_label: "Draw",
                        for style in DrawStyle::ALL {
                            Toggle {
                                pressed: Some(draw_style() == style),
                                on_pressed_change: move |_| {
                                    graph_context.update(id, |property| property.draw_style = style);
                                },
                                "{style.label()}"
                            }
                        }
                    }
                }
                div {
                    class: "graph-setting",
                    span { class: "graph-setting-label", "Time axis" }
                    div {
                        class: "graph-time-window",
                        Slider {
                            value: Some(f64::from(time_window().seconds())),
                            min: f64::from(TimeWindow::MIN),
                            max: f64::from(TimeWindow::MAX),
                            step: f64::from(TimeWindow::STEP),
                            label: Some("Time axis".to_string()),
                            on_value_change: move |seconds: f64| {
                                let window = TimeWindow::new(seconds.round() as u32);
                                if time_window() != window {
                                    graph_context.update(id, |property| property.time_window = window);
                                }
                            },
                        }
                        span {
                            class: "graph-time-window-value",
                            "{time_window().seconds()} s"
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
    use std::collections::HashMap;

    use super::PlotFeed;
    use crate::data::{DataEntry, NumberData, Queue, TypedData};

    fn numbers(points: &[(i64, f64)]) -> TypedData {
        TypedData::Number(
            points
                .iter()
                .map(|&(timestamp, value)| NumberData::new(timestamp, value))
                .collect::<Queue<_>>(),
        )
    }

    /// `data` as `DataContext::with_each` gives it.
    fn each(data: &HashMap<String, TypedData>) -> Vec<(&str, &TypedData)> {
        data.iter()
            .map(|(label, data)| (label.as_str(), data))
            .collect()
    }

    /// Shows `labels`, as `FilterContext::shows` would.
    fn shown<'a>(labels: &'a [&'a str]) -> impl Fn(&str) -> bool + 'a {
        move |label| labels.contains(&label)
    }

    #[test]
    fn sends_everything_first_then_only_new_points() {
        let mut feed = PlotFeed::default();
        let mut data = HashMap::from([("temp".to_string(), numbers(&[(1, 1.0), (2, 2.0)]))]);

        let (labels, updates) = feed.next(&each(&data), shown(&["temp"]));
        assert_eq!(labels, ["temp"]);
        assert_eq!(
            updates,
            [("temp".to_string(), true, vec![(1, 1.0), (2, 2.0)])]
        );

        let temp = data
            .get_mut("temp")
            .and_then(NumberData::queue_mut)
            .unwrap();
        temp.push(NumberData::new(3, 3.0));
        let (_, updates) = feed.next(&each(&data), shown(&["temp"]));
        assert_eq!(updates, [("temp".to_string(), false, vec![(3, 3.0)])]);

        let (labels, updates) = feed.next(&each(&data), shown(&["temp"]));
        assert_eq!(labels, ["temp"]);
        assert!(updates.is_empty());
    }

    #[test]
    fn resends_a_cleared_queue_as_a_reset() {
        let mut feed = PlotFeed::default();
        let mut data = HashMap::from([("temp".to_string(), numbers(&[(1, 1.0), (2, 2.0)]))]);
        feed.next(&each(&data), shown(&["temp"]));

        // Cleared and refilled: another queue, even if as long
        data.insert("temp".to_string(), numbers(&[(5, 5.0), (6, 6.0), (7, 7.0)]));
        let (_, updates) = feed.next(&each(&data), shown(&["temp"]));
        assert_eq!(
            updates,
            [("temp".to_string(), true, vec![(5, 5.0), (6, 6.0), (7, 7.0)])]
        );
    }

    #[test]
    fn plots_only_shown_number_labels() {
        let mut feed = PlotFeed::default();
        let data = HashMap::from([
            ("temp".to_string(), numbers(&[(1, 1.0)])),
            ("volt".to_string(), numbers(&[(1, 3.3)])),
            ("message".to_string(), TypedData::String(Queue::new())),
        ]);

        let (labels, _) = feed.next(&each(&data), shown(&["volt", "message"]));
        assert_eq!(labels, ["volt"]);

        // Hidden, then shown again: sent in full once more
        feed.next(&each(&data), shown(&[]));
        let (_, updates) = feed.next(&each(&data), shown(&["volt"]));
        assert_eq!(updates, [("volt".to_string(), true, vec![(1, 3.3)])]);
    }
}
