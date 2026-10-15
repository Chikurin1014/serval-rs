use dioxus::prelude::*;

use super::{
    GraphContext, GraphFrame, GraphKind, GraphPreset, GraphProperty, context::use_graph_setting,
    time_series::series_color,
};
use crate::components::toggle::Toggle;
use crate::data::{DataContext, DataType, Queue, TypedData, Vector2dData, format_number};
use crate::elements::FilterContext;

const TIME_SERIES_CSS: Asset = asset!("/assets/styling/time-series.css");
const UPLOT_CSS: Asset = asset!("/assets/vendor/uplot/uPlot.min.css");
const UPLOT_JS: Asset = asset!(
    "/assets/vendor/uplot/uPlot.iife.min.js",
    AssetOptions::js().with_minify(false)
);
const ARROWS_JS: &str = include_str!("arrows.js");

pub const ARROWS: GraphKind = GraphKind {
    name: "Arrows (2D)",
    menu: "Vector",
    view: |id| rsx! { ArrowsGraph { id } },
    // Itself, its trails on from the start
    presets: &[GraphPreset {
        name: "Arrows (2D)",
        property: || GraphProperty {
            trails: true,
            ..GraphProperty::default()
        },
    }],
};

/// How many values of a label before its newest the trail of its tip shows.
const TRAIL: usize = 50;

/// A 2D vector as `[x, y]`.
type Point = (f64, f64);

/// `(label, color, newest [x, y], trail [[x, y]] oldest first and up to the
/// newest, hover rows)`, as `arrows.js` takes each arrow.
type PlotArrow = (
    String,
    &'static str,
    Point,
    Vec<Point>,
    [(&'static str, String); 4],
);

/// The newest of `queue` (finite), and the [`TRAIL`] before it with it.
fn newest_with_trail(queue: &Queue<Vector2dData>) -> Option<(Point, Vec<Point>)> {
    let values: Vec<Point> = queue
        .iter()
        .skip(queue.len().saturating_sub(TRAIL + 1))
        .map(|entry| (entry.value().x(), entry.value().y()))
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .collect();
    Some((*values.last()?, values))
}

/// A vector as its hover tells it: its components, its length, and its angle
/// from X towards Y.
fn rows((x, y): Point) -> [(&'static str, String); 4] {
    [
        ("X", format_number(x)),
        ("Y", format_number(y)),
        ("Magnitude", format_number(x.hypot(y))),
        (
            "Angle",
            format!("{}°", format_number(y.atan2(x).to_degrees())),
        ),
    ]
}

/// An arrow from the origin for the newest value of each Vector2d label the
/// data list shows, the trail of its tip behind it; each turned on or off in
/// the legend.
#[component]
pub fn ArrowsGraph(id: usize) -> Element {
    let data_context = use_context::<DataContext>();
    let filter_context = use_context::<FilterContext>();
    let mut graph_context = use_context::<GraphContext>();
    let hidden = use_graph_setting(graph_context, id, |property| property.hidden.clone());
    let trails = use_graph_setting(graph_context, id, |property| property.trails);
    let container_id = format!("graph-plot-{id}");
    let plot = use_hook(|| document::eval(ARROWS_JS));

    let vector_labels = use_memo(move || data_context.labels_of(DataType::Vector2d));
    let plotted = use_memo(move || {
        vector_labels
            .read()
            .iter()
            .filter(|label| filter_context.shows(label))
            .count()
    });

    use_effect(move || {
        // Colored by their place among the 2D vector labels
        let colors = vector_labels.read().clone();
        let arrows: Vec<PlotArrow> = data_context.with_each(Some(DataType::Vector2d), |data| {
            data.iter()
                .filter(|(label, _)| filter_context.shows(label))
                .filter_map(|&(label, data)| {
                    let TypedData::Vector2d(queue) = data else {
                        return None;
                    };
                    let (newest, trail) = newest_with_trail(queue)?;
                    let index = colors.iter().position(|each| each == label).unwrap_or(0);
                    Some((
                        label.to_string(),
                        series_color(index),
                        newest,
                        trail,
                        rows(newest),
                    ))
                })
                .collect()
        });
        let _ = plot.send((arrows, hidden(), trails()));
    });

    // The labels turned off in the legend
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
                    span { class: "graph-setting-label", "Trails" }
                    div {
                        class: "graph-scale",
                        role: "group",
                        aria_label: "Trails",
                        Toggle {
                            pressed: Some(trails()),
                            on_pressed_change: move |pressed: bool| {
                                graph_context.update(id, |property| property.trails = pressed);
                            },
                            "Show"
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
            if vector_labels.read().is_empty() {
                p {
                    class: "graph-empty",
                    "No 2D vector data yet. Add a map to a Vector2d to plot it."
                }
            } else if plotted() == 0 {
                p {
                    class: "graph-empty",
                    "No 2D vector data shown. Change the Data list's filters to show some."
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TRAIL, newest_with_trail, rows};
    use crate::data::{Queue, Vector2d, Vector2dData};

    fn vectors(values: impl IntoIterator<Item = (f64, f64)>) -> Queue<Vector2dData> {
        values
            .into_iter()
            .enumerate()
            .map(|(index, (x, y))| Vector2dData::new(index as i64, Vector2d::new(x, y)))
            .collect()
    }

    #[test]
    fn the_newest_comes_with_the_trail_before_it() {
        let (newest, trail) = newest_with_trail(&vectors([(1.0, 2.0), (3.0, 4.0)])).unwrap();
        assert_eq!(newest, (3.0, 4.0));
        assert_eq!(trail, [(1.0, 2.0), (3.0, 4.0)]);

        // As many as it keeps, and the newest
        let many = vectors((0..100).map(|index| (f64::from(index), 0.0)));
        let (newest, trail) = newest_with_trail(&many).unwrap();
        assert_eq!(newest, (99.0, 0.0));
        assert_eq!(trail.len(), TRAIL + 1);
        assert_eq!(trail[0], (f64::from(99 - TRAIL as i32), 0.0));

        assert_eq!(newest_with_trail(&vectors([])), None);
    }

    #[test]
    fn rows_tell_the_components_length_and_angle() {
        assert_eq!(
            rows((3.0, 4.0)).map(|(_, text)| text),
            ["3.0000", "4.0000", "5.0000", "53.130°"]
        );
        assert_eq!(rows((-1.0, 0.0))[3].1, "180.00°");
        assert_eq!(rows((0.0, -2.0))[3].1, "-90.000°");
    }
}
