use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct TimeContext {
    timestamp_ms: Signal<i64>,
}

impl TimeContext {
    pub fn new(timestamp_ms: Signal<i64>) -> Self {
        Self { timestamp_ms }
    }

    pub fn update(&mut self, timestamp: i64) {
        *self.timestamp_ms.write() = timestamp;
    }

    pub fn current(&self) -> i64 {
        *self.timestamp_ms.peek()
    }
}
