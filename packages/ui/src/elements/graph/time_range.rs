//! The time range setting of the graphs over time: a time series's, a box
//! plot's and a scatter plot's.

use dioxus::prelude::*;

use super::{TimeRange, TimeRangeWidth};
use crate::components::{slider::Slider, toggle::Toggle};

/// A slider of the width of `range`, and a toggle of all the time, which
/// turns the slider off; `on_change` is given each range picked.
#[component]
pub(super) fn TimeRangeSetting(range: TimeRange, on_change: EventHandler<TimeRange>) -> Element {
    // Of all the time, those it had: so turned off, it has them again
    let mut ranged = use_signal(|| match range {
        TimeRange::Ranged(width) => width,
        TimeRange::All => TimeRangeWidth::default(),
    });
    let all = range == TimeRange::All;
    let width = match range {
        TimeRange::Ranged(width) => width,
        TimeRange::All => ranged(),
    };

    rsx! {
        div {
            class: "graph-setting",
            span { class: "graph-setting-label", "Time range" }
            div {
                class: "graph-time-range",
                Slider {
                    value: Some(f64::from(width.get())),
                    min: f64::from(TimeRangeWidth::MIN),
                    max: f64::from(TimeRangeWidth::MAX),
                    step: f64::from(TimeRangeWidth::STEP),
                    disabled: all,
                    // The primitive marks its thumb as `data-disabled` alone
                    aria_disabled: all,
                    label: Some("Time range".to_string()),
                    on_value_change: move |picked: f64| {
                        let picked = TimeRangeWidth::new(picked.round() as u32);
                        if picked != width {
                            on_change.call(TimeRange::Ranged(picked));
                        }
                    },
                }
                span {
                    class: "graph-time-range-value",
                    "data-off": all,
                    "{width.get()} s"
                }
                div {
                    class: "graph-scale",
                    role: "group",
                    aria_label: "All the time",
                    Toggle {
                        pressed: Some(all),
                        on_pressed_change: move |pressed: bool| {
                            if pressed {
                                ranged.set(width);
                                on_change.call(TimeRange::All);
                            } else {
                                on_change.call(TimeRange::Ranged(width));
                            }
                        },
                        "All"
                    }
                }
            }
        }
    }
}
