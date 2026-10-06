use dioxus::prelude::*;
use js_sys::{Object, Reflect};

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
            // As above, with the milliseconds as the locale writes them
            let options = Object::new();
            for (key, value) in [
                ("hour", "numeric".into()),
                ("minute", "2-digit".into()),
                ("second", "2-digit".into()),
                ("fractionalSecondDigits", 3.into()),
            ] {
                let _ = Reflect::set(&options, &key.into(), &value);
            }
            date.to_locale_time_string_with_options("default", &options)
                .into()
        })
    });
    children
}
