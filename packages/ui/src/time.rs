use std::rc::Rc;

/// The clock used to timestamp data, in milliseconds since the Unix epoch.
///
/// Each platform provides one with its own clock, so the shared UI does not
/// depend on a platform API to tell the time.
#[derive(Clone)]
pub struct TimeContext {
    now_ms: Rc<dyn Fn() -> i64>,
    /// Given a time and whether to show its milliseconds.
    format_ms: Rc<dyn Fn(i64, bool) -> String>,
}

impl TimeContext {
    /// A context reading `now_ms`, showing times as UTC (see [`Self::with_format`]).
    pub fn new(now_ms: impl Fn() -> i64 + 'static) -> Self {
        Self {
            now_ms: Rc::new(now_ms),
            format_ms: Rc::new(utc_time_of_day),
        }
    }

    /// Shows times with `format_ms` instead, e.g. in the platform's local time.
    /// `format_ms` is given a time and whether to show its milliseconds.
    pub fn with_format(self, format_ms: impl Fn(i64, bool) -> String + 'static) -> Self {
        Self {
            format_ms: Rc::new(format_ms),
            ..self
        }
    }

    /// The current time, read from the clock when called.
    pub fn current(&self) -> i64 {
        (self.now_ms)()
    }

    /// The time of day at `ms` (a time from [`Self::current`]), to show.
    pub fn format(&self, ms: i64) -> String {
        (self.format_ms)(ms, false)
    }

    /// As [`Self::format`], with the milliseconds.
    pub fn format_millis(&self, ms: i64) -> String {
        (self.format_ms)(ms, true)
    }
}

/// `HH:MM:SS` (or `HH:MM:SS.mmm` with `millis`) in UTC: with no platform API,
/// the time zone is unknown.
fn utc_time_of_day(ms: i64, millis: bool) -> String {
    let seconds = ms.div_euclid(1000).rem_euclid(24 * 60 * 60);
    let time = format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds % 3600 / 60,
        seconds % 60
    );
    if millis {
        format!("{time}.{:03}", ms.rem_euclid(1000))
    } else {
        time
    }
}

impl PartialEq for TimeContext {
    // Closures cannot be compared; the same clock is the same context
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.now_ms, &other.now_ms)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use super::TimeContext;

    #[test]
    fn current_reads_the_clock_on_each_call() {
        let now = Rc::new(Cell::new(1_000));
        let time = TimeContext::new({
            let now = now.clone();
            move || now.get()
        });
        assert_eq!(time.current(), 1_000);
        now.set(1_250);
        assert_eq!(time.current(), 1_250);
    }

    #[test]
    fn formats_utc_by_default() {
        // 2026-10-05 01:02:03.456 UTC
        let utc = TimeContext::new(|| 0);
        assert_eq!(utc.format(1_791_162_123_456), "01:02:03");
        assert_eq!(utc.format_millis(1_791_162_123_456), "01:02:03.456");
        let local = TimeContext::new(|| 0).with_format(|_, millis| format!("local {millis}"));
        assert_eq!(local.format(0), "local false");
        assert_eq!(local.format_millis(0), "local true");
    }

    #[test]
    fn clones_share_the_clock() {
        let time = TimeContext::new(|| 0);
        assert!(time == time.clone());
        assert!(time != TimeContext::new(|| 0));
    }
}
