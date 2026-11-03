//! Graphs of the received data. To add a kind, give its settings a variant of
//! [`GraphSettings`], make its [`GraphKind`] (its settings and presets all of
//! that variant) and pass it to [`GraphProvider`]; views wrap their plot in
//! [`GraphFrame`].

mod arrows;
mod board;
mod box_plot;
mod frame;
mod log_table;
mod plot;
mod scatter;
mod time_range;
mod time_series;

pub use arrows::{ARROWS, ARROWS_3D, ArrowsGraph, Dimension};
pub use board::GraphBoard;
pub use box_plot::{BOX_PLOT, BoxPlotGraph};
pub use frame::GraphFrame;
pub use log_table::{LOG_TABLE, LogTable};
pub use logic::graph::{
    ArrowsSettings, AxisScale, BoxPlotSettings, DrawStyle, Graph, GraphContext, GraphKind,
    GraphPreset, GraphProperty, GraphProvider, GraphSettings, LogTableSettings, ScatterSettings,
    TimeRange, TimeRangeWidth, TimeSeriesSettings,
};
pub use scatter::{SCATTER, ScatterGraph};
pub use time_series::{TIME_SERIES, TimeSeriesGraph};

use logic::graph::{until_paused, use_graph_setting, use_variant_setting};

pub fn builtin_graph_kinds() -> Vec<GraphKind> {
    vec![TIME_SERIES, BOX_PLOT, SCATTER, ARROWS, ARROWS_3D, LOG_TABLE]
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use std::mem::discriminant;

    use super::{GraphProperty, GraphSettings, LOG_TABLE, SCATTER, builtin_graph_kinds};

    #[test]
    fn a_graph_has_its_own_settings_and_those_of_every_kind() {
        let saved = json!({
            "title": "Pairs",
            "x_label": "temp",
            "regression": true,
            // Of other kinds: left out
            "draw_style": "stepped",
            "weakest_level": "LOUD",
        });
        let property = GraphProperty::load(SCATTER, &saved).unwrap();
        assert_eq!(property.kind().name, "Scatter");
        assert_eq!(property.title.as_deref(), Some("Pairs"));
        let GraphSettings::Scatter(scatter) = &property.settings else {
            panic!("a scatter plot's");
        };
        assert_eq!(scatter.x_label, "temp");
        assert!(scatter.regression);

        let written = serde_json::to_value(&property).unwrap();
        let mut keys: Vec<_> = written.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        assert_eq!(
            keys,
            [
                "hidden",
                "regression",
                "time_range",
                "title",
                "value_scale",
                "x_label",
                "x_scale",
                "y_label"
            ]
        );
        assert_eq!(GraphProperty::load(SCATTER, &written).unwrap(), property);

        let wrong = GraphProperty::load(LOG_TABLE, &saved).unwrap_err();
        assert!(wrong.starts_with("Log: "), "{wrong}");
    }

    #[test]
    fn each_kind_makes_settings_of_its_own() {
        let variants: Vec<_> = builtin_graph_kinds()
            .iter()
            .map(|kind| discriminant(&(kind.settings)()))
            .collect();
        for (kind, variant) in builtin_graph_kinds().iter().zip(&variants) {
            assert_eq!(kind.property().kind().name, kind.name);
            assert_eq!(
                variants.iter().filter(|other| *other == variant).count(),
                1,
                "{}: a variant of its own",
                kind.name
            );
            for preset in kind.presets {
                let property = kind.preset(preset);
                assert_eq!(property.kind().name, kind.name);
                assert_eq!(
                    discriminant(&property.settings),
                    *variant,
                    "{}",
                    preset.name
                );
            }
        }
    }
}
