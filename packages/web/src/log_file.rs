//! Log files through the File System Access API (`log_file.js`).

use std::rc::Rc;

use wasm_bindgen::JsValue;

use ui::serial::{LocalFuture, SerialResult, log::LogSink};

use crate::serial::message;

/// A log file the user picks; see `SerialBackend::open_log_file`.
pub async fn open(extension: &str, append: bool) -> SerialResult<Option<Rc<dyn LogSink>>> {
    if !js::is_supported() {
        return Err("Saving a log needs a browser that can save files".to_string());
    }
    let log = js::pick_log_file(extension, append)
        .await
        .map_err(message)?;
    Ok((!log.is_null()).then(|| Rc::new(WebLogSink(log)) as Rc<dyn LogSink>))
}

struct WebLogSink(JsValue);

impl LogSink for WebLogSink {
    fn write(&self, bytes: Vec<u8>) {
        js::write_log_file(&self.0, bytes);
    }

    fn close(&self) -> LocalFuture<SerialResult<()>> {
        let log = self.0.clone();
        Box::pin(async move { js::close_log_file(&log).await.map_err(message) })
    }
}

mod js {
    use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

    #[wasm_bindgen(module = "/src/log_file.js")]
    extern "C" {
        #[wasm_bindgen(js_name = isSupported)]
        pub fn is_supported() -> bool;

        /// Null if the user cancels.
        #[wasm_bindgen(catch, js_name = pickLogFile)]
        pub async fn pick_log_file(extension: &str, append: bool) -> Result<JsValue, JsValue>;

        /// Ordered in JS: it returns at once.
        #[wasm_bindgen(js_name = writeLogFile)]
        pub fn write_log_file(log: &JsValue, bytes: Vec<u8>);

        #[wasm_bindgen(catch, js_name = closeLogFile)]
        pub async fn close_log_file(log: &JsValue) -> Result<(), JsValue>;
    }
}
