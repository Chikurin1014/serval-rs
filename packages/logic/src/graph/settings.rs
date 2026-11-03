//! Each kind's own settings, as a layout file keeps them, and those some kinds
//! share.

use serde::{Deserialize, Serialize};

use crate::data::LogLevel;

/// A time series's own settings, each missing in a layout file as by default.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TimeSeriesSettings {
    pub value_scale: AxisScale,
    pub draw_style: DrawStyle,
    pub time_range: TimeRange,
}

/// A box plot's own settings, each missing in a layout file as by default.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BoxPlotSettings {
    pub value_scale: AxisScale,
    pub time_range: TimeRange,
}

/// A scatter plot's own settings, each missing in a layout file as by default.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScatterSettings {
    /// The Number labels it pairs, as X and Y.
    pub x_label: String,
    pub y_label: String,
    pub x_scale: AxisScale,
    /// Its Y axis's (as the value axis of the others).
    pub value_scale: AxisScale,
    /// Whether it shows the line fit to its points.
    pub regression: bool,
    pub time_range: TimeRange,
}

/// The arrows' own settings (of 2D and 3D vectors alike), each missing in a
/// layout file as by default.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ArrowsSettings {
    /// Whether the arrows trail the tips they had.
    pub trails: bool,
}

/// A log table's own settings, each missing in a layout file as by default.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LogTableSettings {
    /// The weakest level it shows: those weaker left out.
    pub weakest_level: LogLevel,
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

#[cfg(test)]
mod tests {
    use super::{AxisScale, DrawStyle, TimeRange, TimeRangeWidth, TimeSeriesSettings};

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
}
