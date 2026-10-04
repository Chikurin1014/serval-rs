use std::rc::Rc;

/// The clock used to timestamp data, in milliseconds since the Unix epoch.
///
/// Each platform provides one with its own clock, so the shared UI does not
/// depend on a platform API to tell the time.
#[derive(Clone)]
pub struct TimeContext {
    now_ms: Rc<dyn Fn() -> i64>,
}

impl TimeContext {
    pub fn new(now_ms: impl Fn() -> i64 + 'static) -> Self {
        Self {
            now_ms: Rc::new(now_ms),
        }
    }

    /// The current time, read from the clock when called.
    pub fn current(&self) -> i64 {
        (self.now_ms)()
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
    fn clones_share_the_clock() {
        let time = TimeContext::new(|| 0);
        assert!(time == time.clone());
        assert!(time != TimeContext::new(|| 0));
    }
}
