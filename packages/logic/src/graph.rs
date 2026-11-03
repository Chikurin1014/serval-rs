//! The graphs, kept in a context so they survive the board being unmounted.
//! Their kinds, and how each shows, are `ui`'s.

mod settings;

use dioxus::prelude::*;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

pub use settings::{
    ArrowsSettings, AxisScale, BoxPlotSettings, DrawStyle, LogTableSettings, ScatterSettings,
    TimeRange, TimeRangeWidth, TimeSeriesSettings,
};

/// A kind of graph: what it is named and shown as. Its settings are all of one
/// variant of [`GraphSettings`], its own.
#[derive(Clone, Copy, Debug)]
pub struct GraphKind {
    pub name: &'static str,
    /// The add menu it is in: with its presets, or itself if it has none.
    pub menu: &'static str,
    pub view: fn(usize) -> Element,
    /// Its settings by default: of a graph added as it is, and of those a
    /// layout file leaves out.
    pub settings: fn() -> GraphSettings,
    pub presets: &'static [GraphPreset],
}

impl GraphKind {
    /// A graph of it as it is, its settings by default.
    pub fn property(&self) -> GraphProperty {
        self.property_of((self.settings)())
    }

    /// A graph of it, of one of its presets.
    pub fn preset(&self, preset: &GraphPreset) -> GraphProperty {
        self.property_of((preset.settings)())
    }

    fn property_of(&self, settings: GraphSettings) -> GraphProperty {
        GraphProperty {
            kind: *self,
            title: None,
            hidden: Vec::new(),
            settings,
        }
    }
}

/// A kind of graph with ready-made settings.
#[derive(Clone, Copy, Debug)]
pub struct GraphPreset {
    pub name: &'static str,
    /// Of its kind's variant.
    pub settings: fn() -> GraphSettings,
}

impl PartialEq for GraphKind {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

/// A graph's kind and settings: those of every kind, and its kind's own. A
/// layout file keeps the settings all in one object, those of its kind alone
/// (and its kind beside them). Made of its kind, as its own settings are.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GraphProperty {
    #[serde(skip)]
    kind: GraphKind,
    /// `None` for a numbered default.
    pub title: Option<String>,
    /// The labels turned off in the legend (a scatter plot has none).
    pub hidden: Vec<String>,
    /// Of its kind's variant: changed within it.
    #[serde(flatten)]
    pub settings: GraphSettings,
}

/// The settings of every kind, as a layout file keeps them.
#[derive(Default, Deserialize)]
#[serde(default)]
struct CommonSettings {
    title: Option<String>,
    hidden: Vec<String>,
}

impl GraphProperty {
    pub fn kind(&self) -> GraphKind {
        self.kind
    }

    /// A graph of `kind` of its settings `saved` in a layout file, each missing
    /// as by default (and those of other kinds left out); else what is wrong.
    pub fn load(kind: GraphKind, saved: &Value) -> Result<Self, String> {
        let wrong = |error: serde_json::Error| format!("{}: {error}", kind.name);
        let CommonSettings { title, hidden } = CommonSettings::deserialize(saved).map_err(wrong)?;
        let settings = (kind.settings)().load(saved).map_err(wrong)?;
        Ok(Self {
            kind,
            title,
            hidden,
            settings,
        })
    }
}

/// A graph's own settings: a variant for each kind (or a few alike).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum GraphSettings {
    TimeSeries(TimeSeriesSettings),
    BoxPlot(BoxPlotSettings),
    Scatter(ScatterSettings),
    Arrows(ArrowsSettings),
    Arrows3d(ArrowsSettings),
    LogTable(LogTableSettings),
}

impl GraphSettings {
    /// Of its variant, of `saved`: each missing as by default.
    fn load(self, saved: &Value) -> Result<Self, serde_json::Error> {
        fn of<T: DeserializeOwned>(saved: &Value) -> Result<T, serde_json::Error> {
            T::deserialize(saved)
        }
        Ok(match self {
            Self::TimeSeries(_) => Self::TimeSeries(of(saved)?),
            Self::BoxPlot(_) => Self::BoxPlot(of(saved)?),
            Self::Scatter(_) => Self::Scatter(of(saved)?),
            Self::Arrows(_) => Self::Arrows(of(saved)?),
            Self::Arrows3d(_) => Self::Arrows3d(of(saved)?),
            Self::LogTable(_) => Self::LogTable(of(saved)?),
        })
    }
}

/// A kind's own settings (or some kinds'): one variant of [`GraphSettings`],
/// taken out of it if it is of that variant.
pub trait VariantOfGraphSettings: 'static {
    fn of(settings: &GraphSettings) -> Option<&Self>;
    fn of_mut(settings: &mut GraphSettings) -> Option<&mut Self>;
}

macro_rules! variant_of_graph_settings {
    ($($settings:ty: $($variant:ident)|+;)+) => {
        $(
            impl VariantOfGraphSettings for $settings {
                fn of(settings: &GraphSettings) -> Option<&Self> {
                    match settings {
                        $(GraphSettings::$variant(own))|+ => Some(own),
                        _ => None,
                    }
                }

                fn of_mut(settings: &mut GraphSettings) -> Option<&mut Self> {
                    match settings {
                        $(GraphSettings::$variant(own))|+ => Some(own),
                        _ => None,
                    }
                }
            }
        )+
    };
}

variant_of_graph_settings! {
    TimeSeriesSettings: TimeSeries;
    BoxPlotSettings: BoxPlot;
    ScatterSettings: Scatter;
    ArrowsSettings: Arrows | Arrows3d;
    LogTableSettings: LogTable;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Graph {
    pub id: usize,
    pub property: GraphProperty,
}

impl Graph {
    pub fn kind(&self) -> GraphKind {
        self.property.kind()
    }

    /// Keyed by id, so each graph keeps its state as others come and go.
    pub fn view(&self) -> Element {
        let id = self.id;
        rsx! {
            Fragment { key: "{id}", {(self.kind().view)(id)} }
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
    /// The graph shown alone, if one is.
    maximized: Signal<Option<usize>>,
}

impl GraphContext {
    pub fn kinds(&self) -> Vec<GraphKind> {
        self.kinds.read().clone()
    }

    pub fn list(&self) -> Vec<Graph> {
        self.graphs.read().clone()
    }

    pub fn count(&self) -> usize {
        self.graphs.read().len()
    }

    /// The graphs to show: the maximized one alone, if there is one.
    pub fn shown(&self) -> Vec<Graph> {
        match (self.maximized)() {
            Some(id) => self.get(id).into_iter().collect(),
            None => self.list(),
        }
    }

    pub fn maximized(&self) -> Option<usize> {
        (self.maximized)()
    }

    pub fn set_maximized(&mut self, id: Option<usize>) {
        self.maximized.set(id);
    }

    pub fn get(&self, id: usize) -> Option<Graph> {
        self.graphs.read().iter().find(|g| g.id == id).cloned()
    }

    pub fn position(&self, id: usize) -> Option<usize> {
        self.graphs.read().iter().position(|g| g.id == id)
    }

    pub fn add(&mut self, property: GraphProperty) -> usize {
        let id = *self.next_id.peek();
        self.next_id.set(id + 1);
        self.graphs.write().push(Graph { id, property });
        // So the new graph is seen
        self.maximized.set(None);
        id
    }

    pub fn update(&mut self, id: usize, f: impl FnOnce(&mut GraphProperty)) {
        if let Some(graph) = self.graphs.write().iter_mut().find(|g| g.id == id) {
            f(&mut graph.property);
        }
    }

    /// As [`Self::update`], of its settings' variant `S` (if they are of it).
    pub fn update_variant<S: VariantOfGraphSettings>(&mut self, id: usize, f: impl FnOnce(&mut S)) {
        self.update(id, |property| {
            if let Some(settings) = S::of_mut(&mut property.settings) {
                f(settings);
            }
        });
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
        if *self.maximized.peek() == Some(id) {
            self.maximized.set(None);
        }
    }

    /// Puts `graphs` in place of all there are.
    pub fn replace_all(&mut self, graphs: Vec<GraphProperty>) {
        let first = *self.next_id.peek();
        self.next_id.set(first + graphs.len());
        let graphs = graphs
            .into_iter()
            .zip(first..)
            .map(|(property, id)| Graph { id, property })
            .collect();
        self.graphs.set(graphs);
        self.maximized.set(None);
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
pub fn until_paused(paused_at: Option<i64>, timestamp: i64) -> bool {
    paused_at.is_none_or(|paused_at| timestamp <= paused_at)
}

/// One setting of a graph, as a memo of its own.
pub fn use_graph_setting<T: Clone + Default + PartialEq + 'static>(
    graph_context: GraphContext,
    id: usize,
    setting: impl Fn(&GraphProperty) -> T + 'static,
) -> Memo<T> {
    use_memo(move || {
        graph_context
            .get(id)
            .map(|graph| setting(&graph.property))
            .unwrap_or_default()
    })
}

/// As [`use_graph_setting`], of its settings' variant `S` (if they are of it).
pub fn use_variant_setting<S: VariantOfGraphSettings, T: Clone + Default + PartialEq + 'static>(
    graph_context: GraphContext,
    id: usize,
    setting: fn(&S) -> T,
) -> Memo<T> {
    use_graph_setting(graph_context, id, move |property| {
        S::of(&property.settings).map(setting).unwrap_or_default()
    })
}

#[component]
pub fn GraphProvider(
    kinds: Vec<GraphKind>,
    #[props(default)] initial: Vec<GraphProperty>,
    children: Element,
) -> Element {
    use_context_provider(|| {
        let mut context = GraphContext {
            kinds: Signal::new(kinds),
            graphs: Signal::new(Vec::new()),
            next_id: Signal::new(0),
            paused_at: Signal::new(None),
            maximized: Signal::new(None),
        };
        for property in initial {
            context.add(property);
        }
        context
    });

    rsx! {
        {children}
    }
}
