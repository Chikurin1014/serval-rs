//! The graphs, kept in a context so they survive the board being unmounted.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use super::{
    ARROWS, ARROWS_3D, BOX_PLOT, LOG_TABLE, SCATTER, TIME_SERIES, arrows::ArrowsSettings,
    box_plot::BoxPlotSettings, log_table::LogTableSettings, scatter::ScatterSettings,
    time_series::TimeSeriesSettings,
};

/// A kind of graph: what it is named and shown as. Which kind a graph is, its
/// settings tell ([`GraphSettings::kind`]).
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
        GraphProperty::new((self.settings)())
    }
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

/// A graph's settings: those of every kind, and its kind's own. A layout file
/// keeps them all in one object, those of its kind alone.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GraphProperty {
    /// `None` for a numbered default.
    pub title: Option<String>,
    /// The labels turned off in the legend (a scatter plot has none).
    pub hidden: Vec<String>,
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
    pub fn new(settings: GraphSettings) -> Self {
        Self {
            title: None,
            hidden: Vec::new(),
            settings,
        }
    }

    pub fn kind(&self) -> GraphKind {
        self.settings.kind()
    }

    /// A graph of `kind` of its settings `saved` in a layout file, each missing
    /// as by default (and those of other kinds left out); else what is wrong.
    pub fn load(kind: GraphKind, saved: &Value) -> Result<Self, String> {
        let wrong = |error: serde_json::Error| format!("{}: {error}", kind.name);
        let CommonSettings { title, hidden } = CommonSettings::deserialize(saved).map_err(wrong)?;
        let settings = (kind.settings)().load(saved).map_err(wrong)?;
        Ok(Self {
            title,
            hidden,
            settings,
        })
    }
}

/// A graph's own settings, of its kind: which kind it is.
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
    /// The one place a kind is tied to its settings.
    pub fn kind(&self) -> GraphKind {
        match self {
            Self::TimeSeries(_) => TIME_SERIES,
            Self::BoxPlot(_) => BOX_PLOT,
            Self::Scatter(_) => SCATTER,
            Self::Arrows(_) => ARROWS,
            Self::Arrows3d(_) => ARROWS_3D,
            Self::LogTable(_) => LOG_TABLE,
        }
    }

    /// Of its kind, of `saved`: each missing as by default.
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
pub(super) trait VariantOfGraphSettings: 'static {
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

/// The newest values a graph over time is of: a time series's time axis fits
/// the data, up to that long; a box plot and a scatter plot take those back
/// from the newest they show (up to when the graphs were paused).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "TimeRangeFile", into = "TimeRangeFile")]
pub enum TimeRange {
    Ranged(TimeRangeWidth),
    /// All the values kept.
    All,
}

impl TimeRange {
    /// Its seconds; none for all the time.
    pub fn seconds(self) -> Option<u32> {
        match self {
            Self::Ranged(width) => Some(width.get()),
            Self::All => None,
        }
    }

    /// The time the values in it start from, it ending at `until`.
    pub fn since(self, until: i64) -> i64 {
        self.seconds()
            .map_or(i64::MIN, |seconds| until - i64::from(seconds) * 1000)
    }
}

impl Default for TimeRange {
    fn default() -> Self {
        Self::Ranged(TimeRangeWidth::default())
    }
}

/// A time range as a layout file has it: its width in seconds, or `"all"`.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum TimeRangeFile {
    Ranged(TimeRangeWidth),
    All(AllTime),
}

#[derive(Serialize, Deserialize)]
enum AllTime {
    #[serde(rename = "all")]
    All,
}

impl From<TimeRangeFile> for TimeRange {
    fn from(range: TimeRangeFile) -> Self {
        match range {
            TimeRangeFile::Ranged(width) => Self::Ranged(width),
            TimeRangeFile::All(AllTime::All) => Self::All,
        }
    }
}

impl From<TimeRange> for TimeRangeFile {
    fn from(range: TimeRange) -> Self {
        match range {
            TimeRange::Ranged(width) => Self::Ranged(width),
            TimeRange::All => Self::All(AllTime::All),
        }
    }
}

/// How many seconds a time range is wide: in its steps within its bounds; the
/// shortest at first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "u32", into = "u32")]
pub struct TimeRangeWidth(u32);

impl TimeRangeWidth {
    pub const MIN: u32 = 10;
    pub const MAX: u32 = 300;
    pub const STEP: u32 = 10;

    pub fn new(seconds: u32) -> Self {
        let steps = (seconds.clamp(Self::MIN, Self::MAX) + Self::STEP / 2) / Self::STEP;
        Self(steps * Self::STEP)
    }

    pub fn get(self) -> u32 {
        self.0
    }
}

impl From<u32> for TimeRangeWidth {
    fn from(seconds: u32) -> Self {
        Self::new(seconds)
    }
}

impl From<TimeRangeWidth> for u32 {
    fn from(width: TimeRangeWidth) -> Self {
        width.get()
    }
}

impl Default for TimeRangeWidth {
    fn default() -> Self {
        Self(Self::MIN)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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
#[serde(rename_all = "lowercase")]
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

    pub fn add(&mut self, property: GraphProperty) -> usize {
        let id = *self.next_id.peek();
        self.next_id.set(id + 1);
        self.graphs.write().push(Graph { id, property });
        id
    }

    pub fn update(&mut self, id: usize, f: impl FnOnce(&mut GraphProperty)) {
        if let Some(graph) = self.graphs.write().iter_mut().find(|g| g.id == id) {
            f(&mut graph.property);
        }
    }

    /// As [`Self::update`], of its settings' variant `S` (if they are of it).
    pub(super) fn update_variant<S: VariantOfGraphSettings>(
        &mut self,
        id: usize,
        f: impl FnOnce(&mut S),
    ) {
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
pub(super) fn use_variant_setting<
    S: VariantOfGraphSettings,
    T: Clone + Default + PartialEq + 'static,
>(
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        AxisScale, DrawStyle, GraphProperty, GraphSettings, LOG_TABLE, SCATTER, TimeRange,
        TimeRangeWidth, TimeSeriesSettings,
    };
    use crate::elements::graph::builtin_graph_kinds;

    #[test]
    fn widths_are_steps_within_their_range_the_shortest_at_first() {
        assert_eq!(TimeRangeWidth::default().get(), 10);
        assert_eq!(TimeRangeWidth::new(64).get(), 60);
        assert_eq!(TimeRangeWidth::new(65).get(), 70);
        assert_eq!(TimeRangeWidth::new(0).get(), 10);
        assert_eq!(TimeRangeWidth::new(1000).get(), 300);
    }

    #[test]
    fn a_time_range_starts_its_seconds_back_or_with_all_from_the_start() {
        assert_eq!(TimeRange::default().seconds(), Some(10));
        assert_eq!(TimeRange::default().since(60_000), 50_000);
        assert_eq!(TimeRange::All.seconds(), None);
        assert_eq!(TimeRange::All.since(60_000), i64::MIN);
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
        for kind in builtin_graph_kinds() {
            assert_eq!(kind.property().kind().name, kind.name);
            for preset in kind.presets {
                assert_eq!((preset.property)().kind().name, kind.name);
            }
        }
    }
}
