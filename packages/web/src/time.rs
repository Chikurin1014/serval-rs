use dioxus::prelude::*;
use js_sys::{Function, Promise, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

use ui::TimeContext;

/// Provides `TimeContext` with browser's clock and timer.
#[component]
pub fn TimeProvider(children: Element) -> Element {
    use_context_provider(|| {
        TimeContext::new(|| js_sys::Date::now() as i64)
            .with_timer(|ms| Box::pin(set_timeout(ms)))
            .with_format(|ms, millis| {
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

/// Resolves after `ms` milliseconds, by the browser's `setTimeout`.
async fn set_timeout(ms: u32) {
    let promise = Promise::new(&mut |resolve, _| {
        let global = js_sys::global();
        if let Ok(set_timeout) = Reflect::get(&global, &JsValue::from_str("setTimeout")) {
            let _ = Function::from(set_timeout).call2(&global, &resolve, &JsValue::from(ms));
        }
    });
    let _ = JsFuture::from(promise).await;
}
