//! The graphs, kept in a context so they survive the board being unmounted.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::data::LogLevel;

#[derive(Clone, Copy, Debug)]
pub struct GraphKind {
    pub name: &'static str,
    /// The add menu it is in: with its presets, or itself if it has none.
    pub menu: &'static str,
    pub view: fn(usize) -> Element,
    pub presets: &'static [GraphPreset],
}

#[derive(Clone, Copy, Debug)]
pub struct GraphPreset {
    pub name: &'static str,
    pub property: fn() -> GraphProperty,
}

impl PartialEq for GraphKind {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

/// As a layout file keeps it, each setting missing there as by default.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphProperty {
    /// `None` for a numbered default.
    pub title: Option<String>,
    /// The value axis's: a scatter plot's Y axis.
    pub value_scale: AxisScale,
    /// A scatter plot's X axis.
    pub x_scale: AxisScale,
    /// The Number labels a scatter plot pairs, as X and Y.
    pub x_label: String,
    pub y_label: String,
    /// Whether a scatter plot shows the line fit to its points.
    pub regression: bool,
    /// Whether the arrows of 2D vectors trail the tips they had.
    pub trails: bool,
    /// The weakest level a log table shows: those weaker left out.
    pub weakest_level: LogLevel,
    pub draw_style: DrawStyle,
    pub time_window: TimeWindow,
    /// The labels turned off in the legend.
    pub hidden: Vec<String>,
}

/// How many seconds of the newest data the time axis shows. At the smallest, it
/// fits the data instead, up to that long.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "u32", into = "u32")]
pub struct TimeWindow(u32);

impl TimeWindow {
    pub const MIN: u32 = 10;
    pub const MAX: u32 = 300;
    pub const STEP: u32 = 10;

    pub fn new(seconds: u32) -> Self {
        let steps = (seconds.clamp(Self::MIN, Self::MAX) + Self::STEP / 2) / Self::STEP;
        Self(steps * Self::STEP)
    }

    pub fn seconds(self) -> u32 {
        self.0
    }

    pub fn fits_data(self) -> bool {
        self.0 == Self::MIN
    }
}

impl From<u32> for TimeWindow {
    fn from(seconds: u32) -> Self {
        Self::new(seconds)
    }
}

impl From<TimeWindow> for u32 {
    fn from(window: TimeWindow) -> Self {
        window.seconds()
    }
}

impl Default for TimeWindow {
    fn default() -> Self {
        Self(Self::MIN)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrawStyle {
    Points,
    #[default]
    Linear,
    Stepped,
}

impl DrawStyle {
    pub const ALL: [DrawStyle; 3] = [Self::Points, Self::Linear, Self::Stepped];

    /// As `time_series.js` takes it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Points => "points",
            Self::Linear => "linear",
            Self::Stepped => "stepped",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Points => "Points",
            Self::Linear => "Linear",
            Self::Stepped => "Stepped",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AxisScale {
    #[default]
    Linear,
    /// Values of 0 or less are left out.
    Log,
}

impl AxisScale {
    pub const ALL: [AxisScale; 2] = [Self::Linear, Self::Log];

    pub fn name(self) -> &'static str {
        match self {
            Self::Linear => "Linear",
            Self::Log => "Log",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Graph {
    pub id: usize,
    pub kind: GraphKind,
    pub property: GraphProperty,
}

impl Graph {
    /// Keyed by id, so each graph keeps its state as others come and go.
    pub fn view(&self) -> Element {
        let id = self.id;
        rsx! {
            Fragment { key: "{id}", {(self.kind.view)(id)} }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct GraphContext {
    kinds: Signal<Vec<GraphKind>>,
    graphs: Signal<Vec<Graph>>,
    next_id: Signal<usize>,
    /// When the graphs were paused: they show the data up to then (in their
    /// settings as they are changed), the port read on all the same.
    paused_at: Signal<Option<i64>>,
}

impl GraphContext {
    pub fn kinds(&self) -> Vec<GraphKind> {
        self.kinds.read().clone()
    }

    pub fn list(&self) -> Vec<Graph> {
        self.graphs.read().clone()
    }

    pub fn get(&self, id: usize) -> Option<Graph> {
        self.graphs.read().iter().find(|g| g.id == id).cloned()
    }

    pub fn position(&self, id: usize) -> Option<usize> {
        self.graphs.read().iter().position(|g| g.id == id)
    }

    pub fn add(&mut self, kind: GraphKind, property: GraphProperty) -> usize {
        let id = *self.next_id.peek();
        self.next_id.set(id + 1);
        self.graphs.write().push(Graph { id, kind, property });
        id
    }

    pub fn update(&mut self, id: usize, f: impl FnOnce(&mut GraphProperty)) {
        if let Some(graph) = self.graphs.write().iter_mut().find(|g| g.id == id) {
            f(&mut graph.property);
        }
    }

    /// Turns off `labels` in its legend, the others on.
    pub fn set_hidden(&mut self, id: usize, labels: Vec<String>) {
        if self
            .get(id)
            .is_some_and(|graph| graph.property.hidden != labels)
        {
            self.update(id, |property| property.hidden = labels);
        }
    }

    pub fn remove(&mut self, id: usize) {
        self.graphs.write().retain(|g| g.id != id);
    }

    /// Puts `graphs` in place of all there are.
    pub fn replace_all(&mut self, graphs: Vec<(GraphKind, GraphProperty)>) {
        let first = *self.next_id.peek();
        self.next_id.set(first + graphs.len());
        let graphs = graphs
            .into_iter()
            .zip(first..)
            .map(|((kind, property), id)| Graph { id, kind, property })
            .collect();
        self.graphs.set(graphs);
    }

    /// When the graphs were paused, if they are: they show the data up to
    /// then until played again, and then all that came meanwhile.
    pub fn paused_at(&self) -> Option<i64> {
        (self.paused_at)()
    }

    /// Pauses the graphs at `at` (as `TimeContext` has it), or plays them with
    /// none.
    pub fn set_paused_at(&mut self, at: Option<i64>) {
        self.paused_at.set(at);
    }

    /// Pauses the graphs at `at`, unless they are already.
    pub fn pause(&mut self, at: i64) {
        if self.paused_at.peek().is_none() {
            self.paused_at.set(Some(at));
        }
    }

    pub fn move_to(&mut self, id: usize, position: usize) {
        let mut graphs = self.graphs.write();
        if let Some(from) = graphs.iter().position(|g| g.id == id) {
            let graph = graphs.remove(from);
            let to = position.min(graphs.len());
            graphs.insert(to, graph);
        }
    }
}

/// Whether an entry of `timestamp` is shown, with the graphs paused at
/// `paused_at` (if they are).
pub(super) fn until_paused(paused_at: Option<i64>, timestamp: i64) -> bool {
    paused_at.is_none_or(|paused_at| timestamp <= paused_at)
}

/// One setting of a graph, as a memo of its own.
pub(super) fn use_graph_setting<T: Clone + Default + PartialEq + 'static>(
    graph_context: GraphContext,
    id: usize,
    setting: fn(&GraphProperty) -> T,
) -> Memo<T> {
    use_memo(move || {
        graph_context
            .get(id)
            .map(|graph| setting(&graph.property))
            .unwrap_or_default()
    })
}

#[component]
pub fn GraphProvider(
    kinds: Vec<GraphKind>,
    #[props(default)] initial: Vec<(GraphKind, GraphProperty)>,
    children: Element,
) -> Element {
    use_context_provider(|| {
        let mut context = GraphContext {
            kinds: Signal::new(kinds),
            graphs: Signal::new(Vec::new()),
            next_id: Signal::new(0),
            paused_at: Signal::new(None),
        };
        for (kind, property) in initial {
            context.add(kind, property);
        }
        context
    });

    rsx! {
        {children}
    }
}

#[cfg(test)]
mod tests {
    use super::TimeWindow;

    #[test]
    fn time_windows_are_steps_within_their_range() {
        assert_eq!(TimeWindow::default().seconds(), 10);
        assert_eq!(TimeWindow::new(64).seconds(), 60);
        assert_eq!(TimeWindow::new(65).seconds(), 70);
        assert_eq!(TimeWindow::new(0).seconds(), 10);
        assert_eq!(TimeWindow::new(1000).seconds(), 300);
    }

    #[test]
    fn only_the_smallest_window_fits_the_data() {
        assert!(TimeWindow::default().fits_data());
        assert!(!TimeWindow::new(20).fits_data());
    }
}
