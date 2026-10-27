//! Scatter plots of one Number label against another: the points the plot is
//! sent, and the line fit to them with its statistics.

use std::collections::VecDeque;

use dioxus::prelude::*;

use super::{GraphKind, GraphPreset, GraphProperty, TimeRange, context::until_paused};
use crate::data::{MAX_ENTRIES_PER_LABEL, NumberData, Queue, SourceCursor};

mod view;

pub use view::ScatterGraph;

pub const SCATTER: GraphKind = GraphKind {
    name: "Scatter",
    menu: "Statistics",
    view: |id| rsx! { ScatterGraph { id } },
    // Itself, its regression line on from the start
    presets: &[GraphPreset {
        name: "Scatter",
        property: || GraphProperty {
            regression: true,
            ..GraphProperty::default()
        },
    }],
};

/// The points of two Number labels a plot does not have yet: each the newest X
/// with the newest Y, once both have a new one (as a map of two inputs takes
/// them), at the time of the later. Those of one time (as of one chunk
/// received) go in turn: its first X and Y, then its second ones. Only those of
/// the time range back from the newest are shown: as older ones leave it, or
/// it changes, those of it are sent anew.
#[derive(Default)]
struct ScatterFeed {
    labels: (String, String),
    x: SourceCursor,
    y: SourceCursor,
    newest: (Option<f64>, Option<f64>),
    /// Whether the plot has any point.
    sent: bool,
    /// All the points paired, as many as it keeps, each with its time.
    kept: VecDeque<(i64, f64, f64)>,
    /// How many of the newest of them the plot has.
    shown: usize,
    /// The time range it has them of.
    range: TimeRange,
    /// When the graphs were paused, if they are: the entries after are left
    /// out, and all sent anew as it changes.
    paused_at: Option<i64>,
}

impl ScatterFeed {
    /// The new points, and whether they replace those sent before (else they
    /// follow them); those of `range`.
    fn next(
        &mut self,
        labels: (&str, &str),
        x: Option<&Queue<NumberData>>,
        y: Option<&Queue<NumberData>>,
        paused_at: Option<i64>,
        range: TimeRange,
    ) -> (bool, Vec<(f64, f64)>) {
        let mut reset = false;
        if (self.labels.0.as_str(), self.labels.1.as_str()) != labels || self.paused_at != paused_at
        {
            let sent = self.sent;
            *self = Self {
                labels: (labels.0.to_string(), labels.1.to_string()),
                paused_at,
                ..Self::default()
            };
            reset = sent;
        }
        let (Some(x), Some(y)) = (x, y) else {
            // As a label is cleared: none left to show
            let reset = reset || self.sent;
            self.x.reset();
            self.y.reset();
            self.newest = (None, None);
            self.sent = false;
            self.kept.clear();
            self.shown = 0;
            return (reset, Vec::new());
        };

        let mut new_x = self.x.read(x);
        let mut new_y = self.y.read(y);
        // What each pairs with is read again from the start
        if new_x.restarted != new_y.restarted {
            if new_x.restarted {
                self.y.reset();
                new_y = self.y.read(y);
            } else {
                self.x.reset();
                new_x = self.x.read(x);
            }
        }
        if new_x.restarted {
            reset = reset || self.sent;
            self.newest = (None, None);
            self.kept.clear();
            self.shown = 0;
        }

        let paused_at = self.paused_at;
        let mut entries: Vec<_> = in_turn(&new_x.entries, true)
            .chain(in_turn(&new_y.entries, false))
            .filter(|&(timestamp, ..)| until_paused(paused_at, timestamp))
            .collect();
        entries.sort_by_key(|&(timestamp, turn, is_x, _)| (timestamp, turn, !is_x));
        let mut points = Vec::new();
        for (timestamp, _, is_x, value) in entries {
            if is_x {
                self.newest.0 = Some(value);
            } else {
                self.newest.1 = Some(value);
            }
            if let (Some(x), Some(y)) = self.newest {
                // `Inf`, `-Inf` and `NaN` left out, as of no place on the plot
                if x.is_finite() && y.is_finite() {
                    points.push((timestamp, x, y));
                }
                self.newest = (None, None);
            }
        }
        self.kept.extend(&points);
        let excess = self.kept.len().saturating_sub(MAX_ENTRIES_PER_LABEL);
        self.kept.drain(..excess);
        let since = self
            .kept
            .back()
            .map_or(i64::MIN, |&(newest, ..)| range.since(newest));
        let shown = self
            .kept
            .iter()
            .rev()
            .take_while(|&&(timestamp, ..)| timestamp >= since)
            .count();
        // Some left the time range (the plot has them), or it changed (those
        // before it come back): those of it go anew. Else the plot keeps as
        // many, as it does
        let kept_as_many = shown == (self.shown + points.len()).min(MAX_ENTRIES_PER_LABEL);
        if range != self.range || !kept_as_many {
            reset = reset || self.sent;
            points = self.points_of(shown).copied().collect();
        }
        self.shown = shown;
        self.range = range;
        if reset {
            self.sent = false;
        }
        self.sent |= !points.is_empty();
        (reset, points.into_iter().map(|(_, x, y)| (x, y)).collect())
    }

    /// The line fit to the points the plot has, by least squares; none for
    /// fewer than two, or all of one X.
    fn line(&self) -> Option<Line> {
        let sums = self.sums()?;
        let slope = sums.xy / sums.xx;
        let line = Line {
            slope,
            intercept: sums.mean_y - slope * sums.mean_x,
        };
        (sums.xx > 0.0 && line.slope.is_finite() && line.intercept.is_finite()).then_some(line)
    }

    /// Of the points the plot has, their sample covariance (over n − 1), and
    /// correlation coefficient (Pearson's), if neither X nor Y is all of one
    /// value; none for fewer than two.
    fn statistics(&self) -> Option<(f64, Option<f64>)> {
        let sums = self.sums()?;
        let covariance = sums.xy / (self.shown - 1) as f64;
        let correlation = sums.xy / (sums.xx * sums.yy).sqrt();
        let correlation =
            (sums.xx > 0.0 && sums.yy > 0.0 && correlation.is_finite()).then_some(correlation);
        covariance.is_finite().then_some((covariance, correlation))
    }

    /// The newest `count` of the points kept.
    fn points_of(&self, count: usize) -> impl Iterator<Item = &(i64, f64, f64)> {
        self.kept.range(self.kept.len() - count..)
    }

    /// Of the points the plot has, if two or more.
    fn sums(&self) -> Option<Sums> {
        if self.shown < 2 {
            return None;
        }
        let count = self.shown as f64;
        let mean_x = self.points_of(self.shown).map(|&(_, x, _)| x).sum::<f64>() / count;
        let mean_y = self.points_of(self.shown).map(|&(_, _, y)| y).sum::<f64>() / count;
        let mut sums = Sums {
            mean_x,
            mean_y,
            xx: 0.0,
            yy: 0.0,
            xy: 0.0,
        };
        for &(_, x, y) in self.points_of(self.shown) {
            let (dx, dy) = (x - mean_x, y - mean_y);
            sums.xx += dx * dx;
            sums.yy += dy * dy;
            sums.xy += dx * dy;
        }
        Some(sums)
    }
}

/// The means of points, and the sums of the products of their deviations
/// from them.
struct Sums {
    mean_x: f64,
    mean_y: f64,
    xx: f64,
    yy: f64,
    xy: f64,
}

/// As the statistics box shows them: `[covariance, correlation]`, to two
/// decimal places; `—` for a correlation of none.
fn statistics_texts((covariance, correlation): (f64, Option<f64>)) -> [String; 2] {
    let correlation = correlation.map_or_else(|| "—".to_string(), signed);
    [signed(covariance), correlation]
}

/// To two decimal places, with `−` before it if it is below 0.
fn signed(value: f64) -> String {
    let value = rounded(value);
    let sign = if value < 0.0 { "−" } else { "" };
    format!("{sign}{:.2}", value.abs())
}

/// `Y = slope X + intercept`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Line {
    slope: f64,
    intercept: f64,
}

impl Line {
    /// Its slope (with its sign), whether its intercept is below 0, and the
    /// intercept's size, to two decimal places: as `Y = −1.52X − 0.20`.
    fn terms(self) -> (String, bool, String) {
        let intercept = rounded(self.intercept);
        (
            signed(self.slope),
            intercept < 0.0,
            format!("{:.2}", intercept.abs()),
        )
    }

    /// As `Y = 1.52X − 0.20`.
    fn equation(self) -> String {
        let (slope, below, intercept) = self.terms();
        let sign = if below { "−" } else { "+" };
        format!("Y = {slope}X {sign} {intercept}")
    }
}

/// To two decimal places, with no −0.00.
fn rounded(value: f64) -> f64 {
    let rounded = (value * 100.0).round() / 100.0;
    if rounded == 0.0 { 0.0 } else { rounded }
}

/// Each of `entries` as `(timestamp, turn, is_x, value)`: its turn, how many
/// of its time come before it.
fn in_turn(
    entries: &[NumberData],
    is_x: bool,
) -> impl Iterator<Item = (i64, usize, bool, f64)> + '_ {
    let mut last = None;
    let mut turn = 0;
    entries.iter().map(move |entry| {
        let timestamp = entry.timestamp();
        turn = if last == Some(timestamp) { turn + 1 } else { 0 };
        last = Some(timestamp);
        (timestamp, turn, is_x, *entry.value())
    })
}

#[cfg(test)]
mod tests {
    use super::{Line, ScatterFeed, statistics_texts};
    use crate::data::{NumberData, Queue};
    use crate::elements::graph::{TimeRange, TimeRangeWidth};

    fn numbers(points: &[(i64, f64)]) -> Queue<NumberData> {
        points
            .iter()
            .map(|&(timestamp, value)| NumberData::new(timestamp, value))
            .collect()
    }

    const LABELS: (&str, &str) = ("temp", "volt");
    /// The shortest time range
    fn ten_seconds() -> TimeRange {
        TimeRange::default()
    }

    #[test]
    fn pairs_the_newest_of_each_once_both_have_one() {
        let mut feed = ScatterFeed::default();
        // Two of X before a Y: the newer one is taken
        let x = numbers(&[(1, 1.0), (2, 2.0), (4, 4.0)]);
        let y = numbers(&[(3, 30.0), (4, 40.0)]);
        assert_eq!(
            feed.next(LABELS, Some(&x), Some(&y), None, TimeRange::All),
            (false, vec![(2.0, 30.0), (4.0, 40.0)])
        );
    }

    #[test]
    fn leaves_out_the_pairs_not_finite() {
        let mut feed = ScatterFeed::default();
        let x = numbers(&[(1, 1.0), (2, f64::INFINITY), (3, 3.0)]);
        let y = numbers(&[(1, 10.0), (2, 20.0), (3, f64::NAN)]);
        assert_eq!(
            feed.next(LABELS, Some(&x), Some(&y), None, TimeRange::All),
            (false, vec![(1.0, 10.0)])
        );
    }

    #[test]
    fn pairs_those_of_one_time_in_turn() {
        let mut feed = ScatterFeed::default();
        // As `a:1 b:10 a:2 b:20` in one chunk
        let x = numbers(&[(1, 1.0), (1, 2.0)]);
        let y = numbers(&[(1, 10.0), (1, 20.0)]);
        assert_eq!(
            feed.next(LABELS, Some(&x), Some(&y), None, TimeRange::All),
            (false, vec![(1.0, 10.0), (2.0, 20.0)])
        );
    }

    #[test]
    fn sends_only_new_points_after_the_first() {
        let mut feed = ScatterFeed::default();
        let mut x = numbers(&[(1, 1.0)]);
        let mut y = numbers(&[(1, 10.0)]);
        feed.next(LABELS, Some(&x), Some(&y), None, TimeRange::All);

        // One waits for the other, across reads
        x.push(NumberData::new(2, 2.0));
        assert_eq!(
            feed.next(LABELS, Some(&x), Some(&y), None, TimeRange::All),
            (false, vec![])
        );
        y.push(NumberData::new(3, 20.0));
        assert_eq!(
            feed.next(LABELS, Some(&x), Some(&y), None, TimeRange::All),
            (false, vec![(2.0, 20.0)])
        );
    }

    #[test]
    fn starts_again_as_a_label_is_cleared_or_another_chosen() {
        let mut feed = ScatterFeed::default();
        let x = numbers(&[(1, 1.0)]);
        let y = numbers(&[(1, 10.0)]);
        feed.next(LABELS, Some(&x), Some(&y), None, TimeRange::All);

        // Cleared: no points left, then those of the new queue
        assert_eq!(
            feed.next(LABELS, None, Some(&y), None, TimeRange::All),
            (true, vec![])
        );
        assert_eq!(
            feed.next(LABELS, None, Some(&y), None, TimeRange::All),
            (false, vec![])
        );
        let x = numbers(&[(5, 5.0)]);
        assert_eq!(
            feed.next(LABELS, Some(&x), Some(&y), None, TimeRange::All),
            (false, vec![(5.0, 10.0)])
        );

        // Another X: all of its points in place of those
        let other = numbers(&[(6, 6.0)]);
        assert_eq!(
            feed.next(
                ("other", "volt"),
                Some(&other),
                Some(&y),
                None,
                TimeRange::All
            ),
            (true, vec![(6.0, 10.0)])
        );
    }

    #[test]
    fn reads_both_again_as_one_queue_is_made_anew() {
        let mut feed = ScatterFeed::default();
        let x = numbers(&[(1, 1.0)]);
        let y = numbers(&[(1, 10.0), (2, 20.0)]);
        feed.next(LABELS, Some(&x), Some(&y), None, TimeRange::All);

        let x = numbers(&[(2, 2.0)]);
        assert_eq!(
            feed.next(LABELS, Some(&x), Some(&y), None, TimeRange::All),
            (true, vec![(2.0, 10.0)])
        );
    }

    #[test]
    fn the_line_is_fit_by_least_squares() {
        let mut feed = ScatterFeed::default();
        let x = numbers(&[(1, 1.0), (2, 2.0), (3, 3.0)]);
        let y = numbers(&[(1, 3.0), (2, 5.0), (3, 4.0)]);
        feed.next(LABELS, Some(&x), Some(&y), None, TimeRange::All);
        assert_eq!(
            feed.line(),
            Some(Line {
                slope: 0.5,
                intercept: 3.0
            })
        );
    }

    #[test]
    fn no_line_fits_one_point_or_all_of_one_x() {
        let mut feed = ScatterFeed::default();
        let x = numbers(&[(1, 1.0)]);
        let y = numbers(&[(1, 3.0)]);
        feed.next(LABELS, Some(&x), Some(&y), None, TimeRange::All);
        assert_eq!(feed.line(), None);

        let x = numbers(&[(1, 2.0), (2, 2.0)]);
        let y = numbers(&[(1, 3.0), (2, 5.0)]);
        feed.next(("other", "volt"), Some(&x), Some(&y), None, TimeRange::All);
        assert_eq!(feed.line(), None);
    }

    #[test]
    fn the_line_is_of_the_points_the_plot_has() {
        let mut feed = ScatterFeed::default();
        let x = numbers(&[(1, 1.0), (2, 2.0)]);
        let y = numbers(&[(1, 1.0), (2, 2.0)]);
        feed.next(LABELS, Some(&x), Some(&y), None, TimeRange::All);
        // Cleared and made anew: those before left out
        let x = numbers(&[(3, 1.0), (4, 2.0)]);
        let y = numbers(&[(3, 5.0), (4, 3.0)]);
        feed.next(LABELS, Some(&x), Some(&y), None, TimeRange::All);
        assert_eq!(
            feed.line(),
            Some(Line {
                slope: -2.0,
                intercept: 7.0
            })
        );
    }

    #[test]
    fn equations_are_to_two_decimal_places() {
        let line = |slope, intercept| Line { slope, intercept }.equation();
        assert_eq!(line(2.0, 1.0), "Y = 2.00X + 1.00");
        assert_eq!(line(-0.256, -3.044), "Y = −0.26X − 3.04");
        // Rounded to nothing: no sign
        assert_eq!(line(-0.004, -0.001), "Y = 0.00X + 0.00");
    }

    #[test]
    fn statistics_are_of_the_points_the_plot_has() {
        let mut feed = ScatterFeed::default();
        let x = numbers(&[(1, 1.0), (2, 2.0), (3, 3.0)]);
        let y = numbers(&[(1, 3.0), (2, 5.0), (3, 4.0)]);
        feed.next(LABELS, Some(&x), Some(&y), None, TimeRange::All);
        // Over n − 1: 1 / 2; and 1 / √(2 × 2)
        assert_eq!(feed.statistics(), Some((0.5, Some(0.5))));
        assert_eq!(statistics_texts((0.5, Some(0.5))), ["0.50", "0.50"]);
        // Covariance first
        assert_eq!(statistics_texts((2.0, Some(0.25))), ["2.00", "0.25"]);

        // Falling: below 0
        let x = numbers(&[(1, 1.0), (2, 2.0)]);
        let y = numbers(&[(1, 4.0), (2, 2.0)]);
        feed.next(("other", "volt"), Some(&x), Some(&y), None, TimeRange::All);
        assert_eq!(feed.statistics(), Some((-1.0, Some(-1.0))));
        assert_eq!(statistics_texts((-1.0, Some(-1.0))), ["−1.00", "−1.00"]);
    }

    #[test]
    fn no_correlation_is_of_one_value_and_no_statistics_of_one_point() {
        let mut feed = ScatterFeed::default();
        let x = numbers(&[(1, 1.0)]);
        let y = numbers(&[(1, 3.0)]);
        feed.next(LABELS, Some(&x), Some(&y), None, TimeRange::All);
        assert_eq!(feed.statistics(), None);

        // Y all of one value: covarying with nothing
        let x = numbers(&[(1, 1.0), (2, 2.0)]);
        let y = numbers(&[(1, 3.0), (2, 3.0)]);
        feed.next(("other", "volt"), Some(&x), Some(&y), None, TimeRange::All);
        assert_eq!(feed.statistics(), Some((0.0, None)));
        assert_eq!(statistics_texts((0.0, None)), ["0.00", "—"]);
    }

    #[test]
    fn paused_it_pairs_what_came_until_then_and_played_all_anew() {
        let mut feed = ScatterFeed::default();
        let x = numbers(&[(1, 1.0), (3, 3.0)]);
        let y = numbers(&[(1, 10.0), (3, 30.0)]);
        feed.next(LABELS, Some(&x), Some(&y), None, TimeRange::All);

        assert_eq!(
            feed.next(LABELS, Some(&x), Some(&y), Some(2), TimeRange::All),
            (true, vec![(1.0, 10.0)])
        );
        // Its line and statistics of those alone
        assert_eq!(feed.statistics(), None);
        assert_eq!(
            feed.next(LABELS, Some(&x), Some(&y), None, TimeRange::All),
            (true, vec![(1.0, 10.0), (3.0, 30.0)])
        );
    }

    #[test]
    fn keeps_the_points_of_the_time_range_back_from_the_newest() {
        let mut feed = ScatterFeed::default();
        let x = numbers(&[(0, 0.0), (5_000, 5.0), (20_000, 20.0)]);
        let y = numbers(&[(0, 0.0), (5_000, 50.0), (20_000, 200.0)]);
        // 10 s back from the newest (20 s): those at 10 s or after
        assert_eq!(
            feed.next(LABELS, Some(&x), Some(&y), None, ten_seconds()),
            (false, vec![(20.0, 200.0)])
        );
        // None leaving it: nothing to send
        assert_eq!(
            feed.next(LABELS, Some(&x), Some(&y), None, ten_seconds()),
            (false, vec![])
        );
    }

    #[test]
    fn sends_all_anew_as_points_leave_the_time_range() {
        let mut feed = ScatterFeed::default();
        let x = numbers(&[(0, 0.0), (5_000, 1.0), (8_000, 2.0)]);
        let y = numbers(&[(0, 0.0), (5_000, 10.0), (8_000, 20.0)]);
        feed.next(LABELS, Some(&x), Some(&y), None, ten_seconds());

        // A newer point puts the first out: all go anew
        let x = numbers(&[(0, 0.0), (5_000, 1.0), (8_000, 2.0), (12_000, 3.0)]);
        let y = numbers(&[(0, 0.0), (5_000, 10.0), (8_000, 20.0), (12_000, 30.0)]);
        assert_eq!(
            feed.next(LABELS, Some(&x), Some(&y), None, ten_seconds()),
            (true, vec![(1.0, 10.0), (2.0, 20.0), (3.0, 30.0)])
        );
        // Its line of those alone
        assert_eq!(
            feed.line(),
            Some(Line {
                slope: 10.0,
                intercept: 0.0
            })
        );
    }

    #[test]
    fn sends_those_of_it_anew_as_the_time_range_changes() {
        let mut feed = ScatterFeed::default();
        let x = numbers(&[(0, 0.0), (15_000, 1.0), (20_000, 2.0)]);
        let y = numbers(&[(0, 0.0), (15_000, 10.0), (20_000, 20.0)]);
        feed.next(LABELS, Some(&x), Some(&y), None, ten_seconds());

        // Longer: those before come back
        assert_eq!(
            feed.next(
                LABELS,
                Some(&x),
                Some(&y),
                None,
                TimeRange::Ranged(TimeRangeWidth::new(20))
            ),
            (true, vec![(0.0, 0.0), (1.0, 10.0), (2.0, 20.0)])
        );
        assert_eq!(
            feed.next(LABELS, Some(&x), Some(&y), None, ten_seconds()),
            (true, vec![(1.0, 10.0), (2.0, 20.0)])
        );
        // All the time: every one
        assert_eq!(
            feed.next(LABELS, Some(&x), Some(&y), None, TimeRange::All),
            (true, vec![(0.0, 0.0), (1.0, 10.0), (2.0, 20.0)])
        );
    }

    #[test]
    fn keeps_all_the_points_of_all_the_time() {
        let mut feed = ScatterFeed::default();
        let x = numbers(&[(0, 0.0), (60_000, 1.0)]);
        let y = numbers(&[(0, 0.0), (60_000, 10.0)]);
        assert_eq!(
            feed.next(LABELS, Some(&x), Some(&y), None, TimeRange::All),
            (false, vec![(0.0, 0.0), (1.0, 10.0)])
        );
    }
}
