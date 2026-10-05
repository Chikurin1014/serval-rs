use dioxus::prelude::*;

use ui::TimeContext;

/// Provides `TimeContext` with browser's clock.
#[component]
pub fn TimeProvider(children: Element) -> Element {
    use_context_provider(|| {
        TimeContext::new(|| js_sys::Date::now() as i64).with_format(|ms| {
            // The time of day in the browser's time zone and locale
            js_sys::Date::new(&(ms as f64).into())
                .to_locale_time_string("default")
                .into()
        })
    });
    children
}
