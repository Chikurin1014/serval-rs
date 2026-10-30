use dioxus::prelude::*;

use crate::data::LogLevel;

const LOG_LEVEL_TAG_CSS: Asset = asset!("/assets/styling/log-level-tag.css");

/// A log level as a tag in its color, as `env_logger` colors it; none for None.
/// Its name past the initial apart, so a narrow place can show only the
/// initial (by CSS, hiding `.log-level-tag-rest`).
#[component]
pub fn LogLevelTag(level: LogLevel) -> Element {
    if level == LogLevel::None {
        return rsx! {};
    }
    let (initial, rest) = level.as_str().split_at(1);
    rsx! {
        document::Link { rel: "stylesheet", href: LOG_LEVEL_TAG_CSS }
        span {
            class: "log-level-tag",
            "data-level": level.as_str(),
            "{initial}"
            span { class: "log-level-tag-rest", "{rest}" }
        }
    }
}
