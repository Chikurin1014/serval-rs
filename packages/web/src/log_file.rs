//! Log files through the File System Access API (`log_file.js`).

use std::rc::Rc;

use wasm_bindgen::JsValue;

use ui::serial::{
    LocalFuture, SerialResult,
    log::{LogFile, LogSink, TAIL_BYTES},
};

use crate::serial::message;

/// A log file the user picks; see `SerialBackend::open_log_file`.
pub async fn open(extension: &str, append: bool) -> SerialResult<Option<LogFile>> {
    if !js::is_supported() {
        return Err("Saving a log needs a browser that can save files".to_string());
    }
    let log = js::pick_log_file(extension, append, TAIL_BYTES as u32)
        .await
        .map_err(message)?;
    if log.is_null() {
        return Ok(None);
    }
    Ok(Some(LogFile {
        name: js::log_file_name(&log),
        tail: js::log_file_tail(&log),
        sink: Rc::new(WebLogSink(log)),
    }))
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
        pub async fn pick_log_file(
            extension: &str,
            append: bool,
            tail_bytes: u32,
        ) -> Result<JsValue, JsValue>;

        #[wasm_bindgen(js_name = logFileName)]
        pub fn log_file_name(log: &JsValue) -> String;

        #[wasm_bindgen(js_name = logFileTail)]
        pub fn log_file_tail(log: &JsValue) -> Vec<u8>;

        /// Ordered in JS: it returns at once.
        #[wasm_bindgen(js_name = writeLogFile)]
        pub fn write_log_file(log: &JsValue, bytes: Vec<u8>);

        #[wasm_bindgen(catch, js_name = closeLogFile)]
        pub async fn close_log_file(log: &JsValue) -> Result<(), JsValue>;
    }
}
