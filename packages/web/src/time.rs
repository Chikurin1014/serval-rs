use dioxus::prelude::*;

use ui::TimeContext;

/// Provides `TimeContext` with browser's clock.
#[component]
pub fn TimeProvider(children: Element) -> Element {
    use_context_provider(|| TimeContext::new(|| js_sys::Date::now() as i64));
    children
}
