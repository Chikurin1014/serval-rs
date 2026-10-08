use dioxus::prelude::*;
use dioxus_primitives::toast::{ToastOptions, ToastType, Toasts, use_toast};

use crate::time::TimeContext;

/// As long as a toast stays up.
const INTERVAL_MS: i64 = 5_000;

/// Shows toasts, skipping a repeat of the last title for `INTERVAL_MS`.
#[derive(Clone, Copy)]
pub struct Toaster {
    toasts: Toasts,
    time: CopyValue<TimeContext>,
    last: CopyValue<Option<(String, i64)>>,
}

impl Toaster {
    pub fn success(&self, title: &str, description: &str) {
        self.show(ToastType::Success, title, description);
    }

    pub fn info(&self, title: &str, description: &str) {
        self.show(ToastType::Info, title, description);
    }

    pub fn error(&self, title: &str, description: &str) {
        self.show(ToastType::Error, title, description);
    }

    fn show(&self, kind: ToastType, title: &str, description: &str) {
        let now = self.time.read().current();
        let mut last = self.last;
        if !should_show(last.peek().as_ref(), title, now) {
            return;
        }
        last.set(Some((title.to_string(), now)));
        self.toasts.show(
            title.to_string(),
            kind,
            ToastOptions::new().description(description),
        );
    }
}

pub fn use_toaster() -> Toaster {
    let toasts = use_toast();
    let time = use_context::<TimeContext>();
    use_hook(|| Toaster {
        toasts,
        time: CopyValue::new(time),
        last: CopyValue::new(None),
    })
}

fn should_show(last: Option<&(String, i64)>, title: &str, now: i64) -> bool {
    match last {
        Some((last_title, shown_at)) => last_title != title || now - shown_at >= INTERVAL_MS,
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::{INTERVAL_MS, should_show};

    #[test]
    fn the_same_title_waits_for_the_interval() {
        let last = ("Failed to send".to_string(), 1_000);
        assert!(!should_show(
            Some(&last),
            "Failed to send",
            1_000 + INTERVAL_MS - 1
        ));
        assert!(should_show(
            Some(&last),
            "Failed to send",
            1_000 + INTERVAL_MS
        ));
    }

    #[test]
    fn another_title_shows_at_once() {
        let last = ("Port opened".to_string(), 1_000);
        assert!(should_show(Some(&last), "Port closed", 1_001));
        assert!(should_show(None, "Port opened", 0));
    }
}
