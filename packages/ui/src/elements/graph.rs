//! Graphs of the received data. To add a kind, give its settings a variant of
//! [`GraphSettings`] (tied to its [`GraphKind`] in [`GraphSettings::kind`]) and
//! pass the kind to [`GraphProvider`]; views wrap their plot in [`GraphFrame`].

mod arrows;
mod board;
mod box_plot;
mod context;
mod frame;
mod log_table;
mod plot;
mod scatter;
mod time_range;
mod time_series;

pub use arrows::{ARROWS, ARROWS_3D, ArrowsGraph, ArrowsSettings, Dimension};
pub use board::GraphBoard;
pub use box_plot::{BOX_PLOT, BoxPlotGraph, BoxPlotSettings};
pub use context::{
    AxisScale, DrawStyle, Graph, GraphContext, GraphKind, GraphPreset, GraphProperty,
    GraphProvider, GraphSettings, TimeRange, TimeRangeWidth,
};
pub use frame::GraphFrame;
pub use log_table::{LOG_TABLE, LogTable, LogTableSettings};
pub use scatter::{SCATTER, ScatterGraph, ScatterSettings};
pub use time_series::{TIME_SERIES, TimeSeriesGraph, TimeSeriesSettings};

pub fn builtin_graph_kinds() -> Vec<GraphKind> {
    vec![TIME_SERIES, BOX_PLOT, SCATTER, ARROWS, ARROWS_3D, LOG_TABLE]
}
