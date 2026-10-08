//! The graphs, kept in a context so they survive the board being unmounted.

use dioxus::prelude::*;

#[derive(Clone, Copy, Debug)]
pub struct GraphKind {
    pub name: &'static str,
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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GraphProperty {
    /// `None` for a numbered default.
    pub title: Option<String>,
    pub value_scale: AxisScale,
    pub draw_style: DrawStyle,
    pub time_window: TimeWindow,
    /// The labels turned off in the legend.
    pub hidden: Vec<String>,
}

/// How many seconds of the newest data the time axis shows. At the smallest, it
/// fits the data instead, up to that long.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

impl Default for TimeWindow {
    fn default() -> Self {
        Self(Self::MIN)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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

    pub fn remove(&mut self, id: usize) {
        self.graphs.write().retain(|g| g.id != id);
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
