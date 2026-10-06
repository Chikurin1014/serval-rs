//! Graphs of the received data: their state ([`GraphContext`]), the kinds of
//! graph, and the board showing them.
//!
//! To add a kind of graph, pass a [`GraphKind`] with its view to
//! [`GraphProvider`] (see [`builtin_graph_kinds`] for the built-in ones). Views
//! wrap their plot in [`GraphFrame`] for the parts every kind shares.

mod board;
mod context;
mod frame;
mod time_series;

pub use board::GraphBoard;
pub use context::{
    AxisScale, DrawStyle, Graph, GraphContext, GraphKind, GraphProperty, GraphProvider,
};
pub use frame::GraphFrame;
pub use time_series::{TIME_SERIES, TimeSeriesGraph};

/// The built-in kinds, for `GraphProvider`'s `kinds`.
pub fn builtin_graph_kinds() -> Vec<GraphKind> {
    vec![TIME_SERIES]
}

/// For `GraphProvider`'s `initial`: one empty time series graph.
pub fn initial_graphs() -> Vec<(GraphKind, GraphProperty)> {
    vec![(TIME_SERIES, GraphProperty::default())]
}
