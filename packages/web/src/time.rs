use dioxus::prelude::*;

use ui::TimeContext;

/// Provides `TimeContext` with browser's clock.
#[component]
pub fn TimeProvider(children: Element) -> Element {
    use_context_provider(|| {
        TimeContext::new(|| js_sys::Date::now() as i64).with_format(|ms, millis| {
            // The time of day in the browser's time zone and locale
            let date = js_sys::Date::new(&(ms as f64).into());
            if !millis {
                return date.to_locale_time_string("default").into();
            }
            // `HH:MM:SS.SSS` in the browser's time zone, the same in every locale
            format!(
                "{:02}:{:02}:{:02}.{:03}",
                date.get_hours(),
                date.get_minutes(),
                date.get_seconds(),
                date.get_milliseconds()
            )
        })
    });
    children
}
