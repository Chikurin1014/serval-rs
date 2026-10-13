use dioxus::prelude::*;

use crate::data::LogLevel;

const LOG_LEVEL_TAG_CSS: Asset = asset!("/assets/styling/log-level-tag.css");

/// A log level as a tag in its color, as `env_logger` colors it; none for None.
#[component]
pub fn LogLevelTag(level: LogLevel) -> Element {
    if level == LogLevel::None {
        return rsx! {};
    }
    rsx! {
        document::Link { rel: "stylesheet", href: LOG_LEVEL_TAG_CSS }
        span {
            class: "log-level-tag",
            "data-level": level.as_str(),
            "{level.as_str()}"
        }
    }
}
