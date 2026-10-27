use std::{cell::RefCell, collections::HashMap, rc::Rc};

use dioxus::prelude::*;

use super::{
    AxisScale, DrawStyle, GraphContext, GraphFrame, GraphKind, GraphPreset, GraphProperty,
    TimeWindow,
    context::{until_paused, use_graph_setting},
    plot::{PlotAssets, plot_script, series_color, use_plot, use_shown_count},
    time_range::TimeRangeSetting,
};
use crate::components::toggle::Toggle;
use crate::data::{DataContext, DataType, SourceCursor, TypedData};
use crate::elements::FilterContext;

const TIME_SERIES_JS: &str = plot_script!("time_series_data.js", "time_series.js");

pub const TIME_SERIES: GraphKind = GraphKind {
    name: "Time series",
    menu: "Time series",
    view: |id| rsx! { TimeSeriesGraph { id } },
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

/// `(label, reset, [(timestamp_ms, value)])`
type Update = (String, bool, Vec<(i64, f64)>);

#[derive(Default)]
struct PlotFeed {
    cursors: HashMap<String, SourceCursor>,
    /// When the graphs were paused, if they are: the entries after are left
    /// out, and all sent anew as it changes.
    paused_at: Option<i64>,
}

impl PlotFeed {
    /// The shown number labels, and their points the plot does not have yet
    /// (up to `paused_at`, if paused).
    fn next(
        &mut self,
        data: &[(&str, &TypedData)],
        shows: impl Fn(&str) -> bool,
        paused_at: Option<i64>,
    ) -> (Vec<String>, Vec<Update>) {
        if self.paused_at != paused_at {
            *self = Self {
                paused_at,
                ..Self::default()
            };
        }
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
            let points: Vec<_> = read
                .entries
                .iter()
                .filter(|data| until_paused(paused_at, data.timestamp()))
                .map(|data| (data.timestamp(), *data.value()))
                .collect();
            // Nothing new (as what came after the time paused at)
            if !read.restarted && points.is_empty() {
                continue;
            }
            updates.push((label.to_string(), read.restarted, points));
        }
        // So a label shown again is sent in full
        self.cursors.retain(|label, _| labels.contains(label));

        (labels, updates)
    }
}

/// Plots the number labels the data list shows against time.
#[component]
pub fn TimeSeriesGraph(id: usize) -> Element {
    let data_context = use_context::<DataContext>();
    let filter_context = use_context::<FilterContext>();
    let mut graph_context = use_context::<GraphContext>();
    let value_scale = use_graph_setting(graph_context, id, |property| property.value_scale);
    let draw_style = use_graph_setting(graph_context, id, |property| property.draw_style);
    let time_window = use_graph_setting(graph_context, id, |property| property.time_window);
    let all_time = use_graph_setting(graph_context, id, |property| property.all_time);
    let hidden = use_graph_setting(graph_context, id, |property| property.hidden.clone());
    let container_id = format!("graph-plot-{id}");
    let plot = use_plot(id, TIME_SERIES_JS);
    let feed = use_hook(|| Rc::new(RefCell::new(PlotFeed::default())));

    let number_labels = use_memo(move || data_context.labels_of(DataType::Number));
    let plotted = use_shown_count(number_labels);

    use_effect(move || {
        let paused_at = graph_context.paused_at();
        let (labels, updates) = data_context.with_each(Some(DataType::Number), |data| {
            feed.borrow_mut()
                .next(data, |label| filter_context.shows(label), paused_at)
        });
        let colors = number_labels
            .read()
            .iter()
            .enumerate()
            .map(|(index, label)| (label.clone(), series_color(index)))
            .collect::<HashMap<_, _>>();
        let log = value_scale() == AxisScale::Log;
        // No window, of all the time
        let window = (!all_time()).then(|| time_window().seconds());
        let _ = plot.send((
            labels,
            updates,
            colors,
            log,
            draw_style().name(),
            hidden(),
            window,
            paused_at.is_some(),
        ));
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
                TimeRangeSetting {
                    seconds: time_window().seconds(),
                    on_change: move |seconds| {
                        let window = TimeWindow::new(seconds);
                        graph_context.update(id, |property| property.time_window = window);
                    },
                    all: all_time(),
                    on_all_change: move |all| {
                        graph_context.update(id, |property| property.all_time = all);
                    },
                }
            },
            div {
                class: "graph-plot",
                id: "{container_id}",
                onmounted: {
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

    fn each(data: &HashMap<String, TypedData>) -> Vec<(&str, &TypedData)> {
        data.iter()
            .map(|(label, data)| (label.as_str(), data))
            .collect()
    }

    fn shown<'a>(labels: &'a [&'a str]) -> impl Fn(&str) -> bool + 'a {
        move |label| labels.contains(&label)
    }

    #[test]
    fn sends_everything_first_then_only_new_points() {
        let mut feed = PlotFeed::default();
        let mut data = HashMap::from([("temp".to_string(), numbers(&[(1, 1.0), (2, 2.0)]))]);

        let (labels, updates) = feed.next(&each(&data), shown(&["temp"]), None);
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
        let (_, updates) = feed.next(&each(&data), shown(&["temp"]), None);
        assert_eq!(updates, [("temp".to_string(), false, vec![(3, 3.0)])]);

        let (labels, updates) = feed.next(&each(&data), shown(&["temp"]), None);
        assert_eq!(labels, ["temp"]);
        assert!(updates.is_empty());
    }

    #[test]
    fn resends_a_cleared_queue_as_a_reset() {
        let mut feed = PlotFeed::default();
        let mut data = HashMap::from([("temp".to_string(), numbers(&[(1, 1.0), (2, 2.0)]))]);
        feed.next(&each(&data), shown(&["temp"]), None);

        data.insert("temp".to_string(), numbers(&[(5, 5.0), (6, 6.0), (7, 7.0)]));
        let (_, updates) = feed.next(&each(&data), shown(&["temp"]), None);
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
            ("message".to_string(), TypedData::Log(Queue::new())),
        ]);

        let (labels, _) = feed.next(&each(&data), shown(&["volt", "message"]), None);
        assert_eq!(labels, ["volt"]);

        feed.next(&each(&data), shown(&[]), None);
        let (_, updates) = feed.next(&each(&data), shown(&["volt"]), None);
        assert_eq!(updates, [("volt".to_string(), true, vec![(1, 3.3)])]);
    }

    #[test]
    fn paused_it_sends_what_came_until_then_and_played_all_anew() {
        let mut feed = PlotFeed::default();
        let mut data =
            HashMap::from([("temp".to_string(), numbers(&[(1, 1.0), (2, 2.0), (3, 3.0)]))]);
        feed.next(&each(&data), shown(&["temp"]), None);

        // In place of what it had: up to the time paused at
        let (_, updates) = feed.next(&each(&data), shown(&["temp"]), Some(2));
        assert_eq!(
            updates,
            [("temp".to_string(), true, vec![(1, 1.0), (2, 2.0)])]
        );
        // What comes meanwhile, left out
        let temp = data
            .get_mut("temp")
            .and_then(NumberData::queue_mut)
            .unwrap();
        temp.push(NumberData::new(4, 4.0));
        let (_, updates) = feed.next(&each(&data), shown(&["temp"]), Some(2));
        assert!(updates.is_empty());

        // Played: all of it
        let (_, updates) = feed.next(&each(&data), shown(&["temp"]), None);
        assert_eq!(
            updates,
            [(
                "temp".to_string(),
                true,
                vec![(1, 1.0), (2, 2.0), (3, 3.0), (4, 4.0)]
            )]
        );
    }
}
