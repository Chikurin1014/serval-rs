use std::time::Duration;

use chrono::{Local, TimeZone, Utc};

use ui::TimeContext;

/// The OS's clock and local time, and tokio's timers.
pub fn context() -> TimeContext {
    TimeContext::new(|| Utc::now().timestamp_millis())
        .with_timer(|ms| Box::pin(tokio::time::sleep(Duration::from_millis(ms.into()))))
        .with_format(|ms, millis| {
            let Some(time) = Local.timestamp_millis_opt(ms).single() else {
                return String::new();
            };
            // With its milliseconds, as the web's: the same in every locale
            let format = if millis { "%H:%M:%S%.3f" } else { "%X" };
            time.format(format).to_string()
        })
}
