use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;

use ui::TimeContext;

struct IntervalHandle {
    window: web_sys::Window,
    id: i32,
    _closure: wasm_bindgen::closure::Closure<dyn FnMut()>,
}

impl Drop for IntervalHandle {
    fn drop(&mut self) {
        self.window.clear_interval_with_handle(self.id);
    }
}

#[component]
pub fn TimeProvider(children: Element) -> Element {
    let now = use_signal(|| js_sys::Date::now() as i64);
    let interval_handle = use_hook(|| Rc::new(RefCell::new(None::<IntervalHandle>)));

    let mut now_for_interval = now.clone();
    let interval_handle_for_effect = interval_handle.clone();
    use_effect(move || {
        if interval_handle_for_effect.borrow().is_some() {
            return;
        }

        let window = web_sys::window().expect("window");
        let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
            now_for_interval.set(js_sys::Date::now() as i64);
        }) as Box<dyn FnMut()>);
        let id = window
            .set_interval_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                1,
            )
            .expect("interval handle");
        *interval_handle_for_effect.borrow_mut() = Some(IntervalHandle {
            window,
            id,
            _closure: closure,
        });
    });

    use_context_provider(|| TimeContext { timestamp_ms: now });
    children
}
