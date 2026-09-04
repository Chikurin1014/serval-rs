use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct TimeContext {
    pub timestamp_ms: Signal<i64>,
}

impl TimeContext {
    pub fn current(&self) -> i64 {
        *self.timestamp_ms.read()
    }
}
