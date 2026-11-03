//! The settings some kinds share, as a layout file keeps them.

use serde::{Deserialize, Serialize};

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
    use super::{TimeRange, TimeRangeWidth};

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
}
