//! Graphs shown on the board, kept in a context so they survive the board
//! being unmounted (e.g. while another tab is shown).

use dioxus::prelude::*;

/// A kind of graph that can be added to the board.
#[derive(Clone, Copy, Debug)]
pub struct GraphKind {
    pub name: &'static str,
    /// Draws the graph with the given id in [`GraphContext`].
    pub view: fn(usize) -> Element,
    /// The settings it can be added with, offered in its add menu.
    pub presets: &'static [GraphPreset],
}

/// A kind of graph with ready-made settings.
#[derive(Clone, Copy, Debug)]
pub struct GraphPreset {
    pub name: &'static str,
    pub property: fn() -> GraphProperty,
}

impl PartialEq for GraphKind {
    // Function pointers have no reliable identity, so compare by name
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

/// What a graph shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GraphProperty {
    /// Name given by the user; views fall back to a numbered default when `None`.
    pub title: Option<String>,
    /// How the value axis is scaled.
    pub value_scale: AxisScale,
    /// How each label's values are drawn.
    pub draw_style: DrawStyle,
    /// How much of the newest data the time axis shows.
    pub time_window: TimeWindow,
    /// The labels turned off in the legend, kept by label so they stay off when
    /// the data is cleared and comes back, or the graph is shown again.
    pub hidden: Vec<String>,
}

/// How many seconds of the newest data a graph's time axis shows: a multiple of
/// [`TimeWindow::STEP`] from [`TimeWindow::MIN`] to [`TimeWindow::MAX`].
///
/// At the smallest, the axis fits the data instead, up to that many seconds;
/// at any other, it is always that wide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeWindow(u32);

impl TimeWindow {
    pub const MIN: u32 = 10;
    pub const MAX: u32 = 300;
    pub const STEP: u32 = 10;

    /// `seconds` as the nearest window there is.
    pub fn new(seconds: u32) -> Self {
        let steps = (seconds.clamp(Self::MIN, Self::MAX) + Self::STEP / 2) / Self::STEP;
        Self(steps * Self::STEP)
    }

    pub fn seconds(self) -> u32 {
        self.0
    }

    /// Whether the axis fits the data (up to [`Self::seconds`]) rather than
    /// always being that wide.
    pub fn fits_data(self) -> bool {
        self.0 == Self::MIN
    }
}

impl Default for TimeWindow {
    fn default() -> Self {
        Self(Self::MIN)
    }
}

/// How a graph draws a label's values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DrawStyle {
    /// Each value on its own, unjoined.
    Points,
    /// Straight lines between the values, filled below.
    #[default]
    Linear,
    /// Steps, each value held until the next, filled below.
    Stepped,
}

impl DrawStyle {
    pub const ALL: [DrawStyle; 3] = [Self::Points, Self::Linear, Self::Stepped];

    /// Also what `time_series.js` is sent.
    pub fn name(self) -> &'static str {
        match self {
            Self::Points => "points",
            Self::Linear => "linear",
            Self::Stepped => "stepped",
        }
    }

    /// As the settings show it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Points => "Points",
            Self::Linear => "Linear",
            Self::Stepped => "Stepped",
        }
    }
}

/// How an axis spaces its values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AxisScale {
    #[default]
    Linear,
    /// Base 10; values of 0 or less are left out.
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

/// One graph in [`GraphContext`].
#[derive(Clone, Debug, PartialEq)]
pub struct Graph {
    pub id: usize,
    pub kind: GraphKind,
    pub property: GraphProperty,
}

impl Graph {
    /// The kind's view, keyed by id so each graph keeps its own state when
    /// others are added or removed (kinds need not key their views).
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
}

impl GraphContext {
    /// The kinds that can be added.
    pub fn kinds(&self) -> Vec<GraphKind> {
        self.kinds.read().clone()
    }

    pub fn list(&self) -> Vec<Graph> {
        self.graphs.read().clone()
    }

    pub fn get(&self, id: usize) -> Option<Graph> {
        self.graphs.read().iter().find(|g| g.id == id).cloned()
    }

    /// Position of the graph among all graphs, e.g. for numbering titles.
    pub fn position(&self, id: usize) -> Option<usize> {
        self.graphs.read().iter().position(|g| g.id == id)
    }

    /// Adds a graph of `kind` and returns its id.
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

    pub fn remove(&mut self, id: usize) {
        self.graphs.write().retain(|g| g.id != id);
    }

    /// Moves the graph to `position` among all graphs (clamped to the end),
    /// shifting the ones in between.
    pub fn move_to(&mut self, id: usize, position: usize) {
        let mut graphs = self.graphs.write();
        if let Some(from) = graphs.iter().position(|g| g.id == id) {
            let graph = graphs.remove(from);
            let to = position.min(graphs.len());
            graphs.insert(to, graph);
        }
    }
}

/// Provides [`GraphContext`].
#[component]
pub fn GraphProvider(
    /// The kinds that can be added.
    kinds: Vec<GraphKind>,
    /// Graphs present from the start.
    #[props(default)]
    initial: Vec<(GraphKind, GraphProperty)>,
    children: Element,
) -> Element {
    use_context_provider(|| {
        let mut context = GraphContext {
            kinds: Signal::new(kinds),
            graphs: Signal::new(Vec::new()),
            next_id: Signal::new(0),
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
