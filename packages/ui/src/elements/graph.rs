//! Graphs of the received data. To add a kind, make its [`GraphKind`] of a type
//! of settings of its own (any a layout file can keep: see [`GraphSettings`]),
//! its presets of that type too, and pass it to [`GraphProvider`]; views wrap
//! their plot in [`GraphFrame`].

mod arrows;
mod board;
mod box_plot;
mod frame;
mod log_table;
mod plot;
mod scatter;
mod time_range;
mod time_series;

pub use arrows::{ARROWS, ARROWS_3D, ArrowsGraph, ArrowsSettings, Dimension};
pub use board::GraphBoard;
pub use box_plot::{BOX_PLOT, BoxPlotGraph, BoxPlotSettings};
pub use frame::GraphFrame;
pub use log_table::{LOG_TABLE, LogTable, LogTableSettings};
pub use logic::graph::{
    AxisScale, DrawStyle, Graph, GraphContext, GraphKind, GraphPreset, GraphProperty,
    GraphProvider, GraphSettings, SettingsType, TimeRange, TimeRangeWidth,
};
pub use scatter::{SCATTER, ScatterGraph, ScatterSettings};
pub use time_series::{TIME_SERIES, TimeSeriesGraph, TimeSeriesSettings};

use logic::graph::{until_paused, use_graph_setting, use_own_setting};

pub fn builtin_graph_kinds() -> Vec<GraphKind> {
    vec![TIME_SERIES, BOX_PLOT, SCATTER, ARROWS, ARROWS_3D, LOG_TABLE]
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        AxisScale, DrawStyle, GraphProperty, LOG_TABLE, SCATTER, ScatterSettings, TimeRange,
        TimeSeriesSettings, builtin_graph_kinds,
    };

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
        let scatter = property
            .settings::<ScatterSettings>()
            .expect("a scatter plot's");
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
        // Its presets checked as they are made, too
        for kind in builtin_graph_kinds() {
            assert_eq!(kind.property().kind().name, kind.name);
            for preset in kind.presets {
                assert_eq!(kind.preset(preset).kind().name, kind.name);
            }
        }
    }

    fn series(text: &str) -> Result<TimeSeriesSettings, serde_json::Error> {
        serde_json::from_str(text)
    }

    #[test]
    fn a_layout_has_a_time_range_as_its_seconds_or_all() {
        assert_eq!(series("{}").unwrap().time_range, TimeRange::default());
        // Its name before: of a layout of v0.4.1, upgraded as it is imported, not here
        let settings = series(r#"{"time_window": 30}"#).unwrap();
        assert_eq!(settings.time_range, TimeRange::default());

        let settings = series(r#"{"time_range": 64}"#).unwrap();
        assert_eq!(settings.time_range.seconds(), Some(60));
        assert_eq!(serde_json::to_value(&settings).unwrap()["time_range"], 60);
        let settings = series(r#"{"time_range": "all"}"#).unwrap();
        assert_eq!(settings.time_range, TimeRange::All);
        assert_eq!(
            serde_json::to_value(&settings).unwrap()["time_range"],
            "all"
        );
        assert!(series(r#"{"time_range": "All"}"#).is_err());
    }

    #[test]
    fn a_layout_has_scales_and_draw_styles_in_lower_case() {
        let settings = series(r#"{"value_scale": "log", "draw_style": "stepped"}"#).unwrap();
        assert_eq!(settings.value_scale, AxisScale::Log);
        assert_eq!(settings.draw_style, DrawStyle::Stepped);
        let written = serde_json::to_value(&settings).unwrap();
        assert_eq!(written["draw_style"], "stepped");
        assert!(series(r#"{"value_scale": "Log"}"#).is_err());
    }
}
