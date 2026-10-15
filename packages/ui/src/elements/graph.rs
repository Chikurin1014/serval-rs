//! Graphs of the received data. To add a kind, pass a [`GraphKind`] to
//! [`GraphProvider`]; views wrap their plot in [`GraphFrame`].

mod arrows;
mod board;
mod box_plot;
mod context;
mod frame;
mod scatter;
mod time_series;

pub use arrows::{ARROWS, ARROWS_3D, ArrowsGraph, Dimension};
pub use board::GraphBoard;
pub use box_plot::{BOX_PLOT, BoxPlotGraph};
pub use context::{
    AxisScale, DrawStyle, Graph, GraphContext, GraphKind, GraphPreset, GraphProperty,
    GraphProvider, TimeWindow,
};
pub use frame::GraphFrame;
pub use scatter::{SCATTER, ScatterGraph};
pub use time_series::{TIME_SERIES, TimeSeriesGraph};

pub fn builtin_graph_kinds() -> Vec<GraphKind> {
    vec![TIME_SERIES, BOX_PLOT, SCATTER, ARROWS, ARROWS_3D]
}

pub fn initial_graphs() -> Vec<(GraphKind, GraphProperty)> {
    vec![(TIME_SERIES, GraphProperty::default())]
}
