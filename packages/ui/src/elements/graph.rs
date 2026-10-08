//! Graphs of the received data. To add a kind, pass a [`GraphKind`] to
//! [`GraphProvider`]; views wrap their plot in [`GraphFrame`].

mod board;
mod context;
mod frame;
mod time_series;

pub use board::GraphBoard;
pub use context::{
    AxisScale, DrawStyle, Graph, GraphContext, GraphKind, GraphPreset, GraphProperty,
    GraphProvider, TimeWindow,
};
pub use frame::GraphFrame;
pub use time_series::{TIME_SERIES, TimeSeriesGraph};

pub fn builtin_graph_kinds() -> Vec<GraphKind> {
    vec![TIME_SERIES]
}

pub fn initial_graphs() -> Vec<(GraphKind, GraphProperty)> {
    vec![(TIME_SERIES, GraphProperty::default())]
}
