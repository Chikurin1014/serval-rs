use dioxus::prelude::*;

use super::{
    GraphContext, GraphFrame, GraphKind, GraphPreset, GraphProperty,
    context::{until_paused, use_graph_setting},
    plot::{PlotAssets, label_color, use_plot, use_shown_count},
};
use crate::components::toggle::Toggle;
use crate::data::{Data, DataContext, DataType, Queue, TypedData, Vector, format_number};
use crate::elements::FilterContext;

const ZDOG_JS: Asset = asset!(
    "/assets/vendor/zdog/zdog.dist.min.js",
    AssetOptions::js().with_minify(false)
);
const ARROWS_JS: &str = concat!(include_str!("paced_draws.js"), include_str!("arrows.js"));
const ARROWS_3D_JS: &str = concat!(include_str!("paced_draws.js"), include_str!("arrows_3d.js"));

/// Itself, its trails on from the start.
fn with_trails() -> GraphProperty {
    GraphProperty {
        trails: true,
        ..GraphProperty::default()
    }
}

pub const ARROWS: GraphKind = GraphKind {
    name: "Arrows (2D)",
    menu: "Vector",
    view: |id| rsx! { ArrowsGraph { id, dimension: Dimension::Two } },
    presets: &[GraphPreset {
        name: "Arrows (2D)",
        property: with_trails,
    }],
};

pub const ARROWS_3D: GraphKind = GraphKind {
    name: "Arrows (3D)",
    menu: "Vector",
    view: |id| rsx! { ArrowsGraph { id, dimension: Dimension::Three } },
    presets: &[GraphPreset {
        name: "Arrows (3D)",
        property: with_trails,
    }],
};

/// Of which vectors arrows are drawn: on a plane with uPlot, or in space with
/// Zdog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dimension {
    Two,
    Three,
}

impl Dimension {
    fn data_type(self) -> DataType {
        match self {
            Dimension::Two => DataType::Vector2d,
            Dimension::Three => DataType::Vector3d,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Dimension::Two => "2D",
            Dimension::Three => "3D",
        }
    }

    fn script(self) -> &'static str {
        match self {
            Dimension::Two => ARROWS_JS,
            Dimension::Three => ARROWS_3D_JS,
        }
    }

    /// A vector as its hover tells it: its components and its length; on a
    /// plane, its angle from X towards Y too.
    fn rows(self, components: &[f64]) -> Vec<(&'static str, String)> {
        let mut rows: Vec<_> = ["X", "Y", "Z"]
            .into_iter()
            .zip(components)
            .map(|(name, &value)| (name, format_number(value)))
            .collect();
        let length = components.iter().map(|value| value * value).sum::<f64>();
        rows.push(("Magnitude", format_number(length.sqrt())));
        if let (Dimension::Two, &[x, y]) = (self, components) {
            let angle = format_number(y.atan2(x).to_degrees());
            rows.push(("Angle", format!("{angle}°")));
        }
        rows
    }
}

/// How many values of a label before its newest the trail of its tip shows.
const TRAIL: usize = 50;

/// `(label, color, newest, trail oldest first and up to the newest, hover
/// rows)`, each vector as its components, as the scripts take each arrow.
type PlotArrow = (
    String,
    &'static str,
    Vec<f64>,
    Vec<Vec<f64>>,
    Vec<(&'static str, String)>,
);

/// The newest of `queue` (finite, and up to `paused_at` if paused), and the
/// [`TRAIL`] before it with it, each as its components.
fn newest_with_trail<V: Vector>(
    queue: &Queue<Data<V>>,
    paused_at: Option<i64>,
) -> Option<(Vec<f64>, Vec<Vec<f64>>)> {
    let mut values: Vec<Vec<f64>> = queue
        .iter()
        .filter(|entry| until_paused(paused_at, entry.timestamp()))
        .map(|entry| entry.value().components().to_vec())
        .filter(|components| components.iter().all(|value| value.is_finite()))
        .collect();
    values.drain(..values.len().saturating_sub(TRAIL + 1));
    Some((values.last()?.clone(), values))
}

/// An arrow from the origin for the newest value of each vector label (of
/// `dimension`) the data list shows, the trail of its tip behind it; each
/// turned on or off in the legend.
#[component]
pub fn ArrowsGraph(id: usize, dimension: Dimension) -> Element {
    let data_context = use_context::<DataContext>();
    let filter_context = use_context::<FilterContext>();
    let mut graph_context = use_context::<GraphContext>();
    let hidden = use_graph_setting(graph_context, id, |property| property.hidden.clone());
    let trails = use_graph_setting(graph_context, id, |property| property.trails);
    let container_id = format!("graph-plot-{id}");
    let plot = use_plot(id, dimension.script());
    let data_type = dimension.data_type();

    let vector_labels = use_memo(move || data_context.labels_of(data_type));
    let plotted = use_shown_count(vector_labels);

    use_effect(move || {
        let paused_at = graph_context.paused_at();
        // Colored by their place among the vector labels
        let colors = vector_labels.read().clone();
        let arrows: Vec<PlotArrow> = data_context.with_each(Some(data_type), |data| {
            data.iter()
                .filter(|(label, _)| filter_context.shows(label))
                .filter_map(|&(label, data)| {
                    let (newest, trail) = match data {
                        TypedData::Vector2d(queue) => newest_with_trail(queue, paused_at),
                        TypedData::Vector3d(queue) => newest_with_trail(queue, paused_at),
                        _ => None,
                    }?;
                    let rows = dimension.rows(&newest);
                    Some((
                        label.to_string(),
                        label_color(&colors, label),
                        newest,
                        trail,
                        rows,
                    ))
                })
                .collect()
        });
        let _ = plot.send((arrows, hidden(), trails()));
    });

    let name = dimension.name();

    rsx! {
        // uPlot's styles for the legend of the 3D arrows too, as the others'
        PlotAssets { uplot: dimension == Dimension::Two }
        if dimension == Dimension::Three {
            document::Script { src: ZDOG_JS }
        }

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
                    "No {name} vector data yet. Add a map to a {data_type.name()} to plot it."
                }
            } else if plotted() == 0 {
                p {
                    class: "graph-empty",
                    "No {name} vector data shown. Change the Data list's filters to show some."
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Dimension, TRAIL, newest_with_trail};
    use crate::data::{Queue, Vector2d, Vector2dData, Vector3d, Vector3dData};

    fn vectors(values: impl IntoIterator<Item = (f64, f64)>) -> Queue<Vector2dData> {
        values
            .into_iter()
            .enumerate()
            .map(|(index, (x, y))| Vector2dData::new(index as i64, Vector2d::new(x, y)))
            .collect()
    }

    #[test]
    fn the_newest_comes_with_the_trail_before_it() {
        let (newest, trail) = newest_with_trail(&vectors([(1.0, 2.0), (3.0, 4.0)]), None).unwrap();
        assert_eq!(newest, [3.0, 4.0]);
        assert_eq!(trail, [[1.0, 2.0], [3.0, 4.0]]);

        // As many as it keeps, and the newest
        let many = vectors((0..100).map(|index| (f64::from(index), 0.0)));
        let (newest, trail) = newest_with_trail(&many, None).unwrap();
        assert_eq!(newest, [99.0, 0.0]);
        assert_eq!(trail.len(), TRAIL + 1);
        assert_eq!(trail[0], [f64::from(99 - TRAIL as i32), 0.0]);

        assert_eq!(newest_with_trail(&vectors([]), None), None);

        // Of three components too
        let space: Queue<Vector3dData> = [Vector3dData::new(1, Vector3d::new(1.0, 2.0, 3.0))]
            .into_iter()
            .collect();
        assert_eq!(newest_with_trail(&space, None).unwrap().0, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn rows_tell_the_components_length_and_on_a_plane_angle() {
        let texts = |dimension: Dimension, components: &[f64]| {
            dimension
                .rows(components)
                .into_iter()
                .map(|(_, text)| text)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            texts(Dimension::Two, &[3.0, 4.0]),
            ["3.0000", "4.0000", "5.0000", "53.130°"]
        );
        assert_eq!(texts(Dimension::Two, &[-1.0, 0.0])[3], "180.00°");
        assert_eq!(texts(Dimension::Two, &[0.0, -2.0])[3], "-90.000°");
        assert_eq!(
            Dimension::Three.rows(&[2.0, 3.0, 6.0]),
            [
                ("X", "2.0000".to_string()),
                ("Y", "3.0000".to_string()),
                ("Z", "6.0000".to_string()),
                ("Magnitude", "7.0000".to_string()),
            ]
        );
    }

    #[test]
    fn paused_the_newest_is_that_until_then() {
        let queue = vectors([(1.0, 1.0), (2.0, 2.0), (3.0, 3.0)]);
        let (newest, trail) = newest_with_trail(&queue, Some(1)).unwrap();
        assert_eq!(newest, [2.0, 2.0]);
        assert_eq!(trail, [[1.0, 1.0], [2.0, 2.0]]);
        assert_eq!(newest_with_trail(&queue, Some(-1)), None);
    }
}
