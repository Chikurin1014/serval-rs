//! The time range setting of the graphs over time: a time series's, a box
//! plot's and a scatter plot's.

use dioxus::prelude::*;

use super::TimeWindow;
use crate::components::{slider::Slider, toggle::Toggle};

/// A slider of `seconds`, in the steps of a [`TimeWindow`], and a toggle of
/// `all` the time, which turns the slider off; `on_change` is given each step
/// picked, `on_all_change` the toggle's state.
#[component]
pub(super) fn TimeRangeSetting(
    seconds: u32,
    all: bool,
    on_change: EventHandler<u32>,
    on_all_change: EventHandler<bool>,
) -> Element {
    rsx! {
        div {
            class: "graph-setting",
            span { class: "graph-setting-label", "Time range" }
            div {
                class: "graph-time-window",
                Slider {
                    value: Some(f64::from(seconds)),
                    min: f64::from(TimeWindow::MIN),
                    max: f64::from(TimeWindow::MAX),
                    step: f64::from(TimeWindow::STEP),
                    disabled: all,
                    // The primitive marks its thumb as `data-disabled` alone
                    aria_disabled: all,
                    label: Some("Time range".to_string()),
                    on_value_change: move |picked: f64| {
                        let picked = TimeWindow::new(picked.round() as u32).seconds();
                        if picked != seconds {
                            on_change.call(picked);
                        }
                    },
                }
                span {
                    class: "graph-time-window-value",
                    "data-off": all,
                    "{seconds} s"
                }
                div {
                    class: "graph-scale",
                    role: "group",
                    aria_label: "All the time",
                    Toggle {
                        pressed: Some(all),
                        on_pressed_change: move |pressed: bool| on_all_change.call(pressed),
                        "All"
                    }
                }
            }
        }
    }
}
