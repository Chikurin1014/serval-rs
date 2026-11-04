//! Record files (`record_file.js`): through the File System Access API, or
//! another way's sink, each written in order.

use std::rc::Rc;

use wasm_bindgen::JsValue;

use serial_runtime::backend::web::message;
use ui::serial::{
    LocalFuture, SerialResult,
    record::{RecordFile, RecordFilePicker, RecordSink, TAIL_BYTES},
};

/// Record files the user picks through the browser.
pub fn record_picker() -> RecordFilePicker {
    RecordFilePicker::new(|extension, append| {
        Box::pin(async move { open_record(&extension, append).await })
    })
}

async fn open_record(extension: &str, append: bool) -> SerialResult<Option<RecordFile>> {
    if !js::is_supported() {
        return Err("Saving a record needs a browser that can save files".to_string());
    }
    let record = js::pick_record_file(extension, append, TAIL_BYTES as u32)
        .await
        .map_err(message)?;
    Ok((!record.is_null()).then(|| record_file(record)))
}

/// A new record's name, of the local time.
pub fn record_name(extension: &str) -> String {
    js::record_name(extension)
}

/// The record of `sink` (as `recording` in `record_file.js` takes it).
pub fn record_of(sink: JsValue) -> RecordFile {
    record_file(js::recording(sink))
}

fn record_file(record: JsValue) -> RecordFile {
    RecordFile {
        name: js::record_file_name(&record),
        tail: js::record_file_tail(&record),
        sink: Rc::new(JsRecordSink(record)),
    }
}

struct JsRecordSink(JsValue);

impl RecordSink for JsRecordSink {
    fn write(&self, bytes: Vec<u8>) {
        js::write_record_file(&self.0, bytes);
    }

    fn close(&self) -> LocalFuture<SerialResult<()>> {
        let record = self.0.clone();
        Box::pin(async move { js::close_record_file(&record).await.map_err(message) })
    }

    fn failure(&self) -> LocalFuture<Option<String>> {
        let record = self.0.clone();
        Box::pin(async move { js::record_file_failure(&record).await.as_string() })
    }
}

mod js {
    use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

    #[wasm_bindgen(module = "/src/record_file.js")]
    extern "C" {
        #[wasm_bindgen(js_name = isSupported)]
        pub fn is_supported() -> bool;

        #[wasm_bindgen(js_name = recordName)]
        pub fn record_name(extension: &str) -> String;

        /// Null if the user cancels.
        #[wasm_bindgen(catch, js_name = pickRecordFile)]
        pub async fn pick_record_file(
            extension: &str,
            append: bool,
            tail_bytes: u32,
        ) -> Result<JsValue, JsValue>;

        pub fn recording(sink: JsValue) -> JsValue;

        #[wasm_bindgen(js_name = recordFileName)]
        pub fn record_file_name(record: &JsValue) -> String;

        #[wasm_bindgen(js_name = recordFileTail)]
        pub fn record_file_tail(record: &JsValue) -> Vec<u8>;

        /// A string once a write fails; null once closed without that.
        #[wasm_bindgen(js_name = recordFileFailure)]
        pub async fn record_file_failure(record: &JsValue) -> JsValue;

        /// Ordered in JS: it returns at once.
        #[wasm_bindgen(js_name = writeRecordFile)]
        pub fn write_record_file(record: &JsValue, bytes: Vec<u8>);

        #[wasm_bindgen(catch, js_name = closeRecordFile)]
        pub async fn close_record_file(record: &JsValue) -> Result<(), JsValue>;
    }
}
