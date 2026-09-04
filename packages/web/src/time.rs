use dioxus::prelude::*;
use wasm_bindgen::JsCast;

use ui::TimeContext;

#[component]
pub fn TimeProvider(children: Element) -> Element {
    let now = use_signal(|| js_sys::Date::now() as i64);

    let mut now_for_interval = now.clone();
    use_effect(move || {
        let window = web_sys::window().expect("window");
        let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
            now_for_interval.set(js_sys::Date::now() as i64);
        }) as Box<dyn FnMut()>);
        let _handle = window
            .set_interval_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                1,
            )
            .expect("interval handle");
        closure.forget();
    });

    use_context_provider(|| TimeContext { timestamp_ms: now });
    children
}
