//! Files through VS Code's dialogs (`vscode_files.js`), where the app is in its
//! panel: those saved, those opened, and record files.

use std::rc::Rc;

use wasm_bindgen::{JsCast, JsValue};

use ui::{
    save::{FileOpener, FileSaver, PickedFile},
    serial::{
        LocalFuture, SerialResult,
        record::{RecordFile, RecordFilePicker, RecordSink, TAIL_BYTES},
    },
};

/// Files saved where the user picks through VS Code.
pub fn saver() -> FileSaver {
    FileSaver::new(|name, text| {
        Box::pin(async move { js::save_file(&name, &text).await.map_err(message) })
    })
}

/// Files the user picks through VS Code, in place of the page's file inputs.
pub fn opener() -> FileOpener {
    FileOpener::new(|extensions| {
        Box::pin(async move {
            let file = js::open_file(extensions).await.map_err(message)?;
            if file.is_null() {
                return Ok(None);
            }
            Ok(Some(PickedFile {
                name: js::file_name(&file),
                bytes: js::file_bytes(&file),
            }))
        })
    })
}

/// Record files the user picks through VS Code.
pub fn record_picker() -> RecordFilePicker {
    RecordFilePicker::new(|extension, append| {
        Box::pin(async move { open_record(&extension, append).await })
    })
}

async fn open_record(extension: &str, append: bool) -> SerialResult<Option<RecordFile>> {
    let record = js::pick_record_file(extension, append, TAIL_BYTES as u32)
        .await
        .map_err(message)?;
    if record.is_null() {
        return Ok(None);
    }
    Ok(Some(RecordFile {
        name: js::record_file_name(&record),
        tail: js::record_file_tail(&record),
        sink: Rc::new(VscodeRecordSink(record)),
    }))
}

struct VscodeRecordSink(JsValue);

impl RecordSink for VscodeRecordSink {
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

fn message(error: JsValue) -> String {
    match error.dyn_ref::<js_sys::Error>() {
        Some(error) => error.message().into(),
        None => format!("{error:?}"),
    }
}

mod js {
    use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

    #[wasm_bindgen(module = "/src/vscode_files.js")]
    extern "C" {
        #[wasm_bindgen(catch, js_name = saveFile)]
        pub async fn save_file(name: &str, text: &str) -> Result<(), JsValue>;

        /// Null if the user cancels.
        #[wasm_bindgen(catch, js_name = openFile)]
        pub async fn open_file(extensions: Vec<String>) -> Result<JsValue, JsValue>;

        #[wasm_bindgen(js_name = fileName)]
        pub fn file_name(file: &JsValue) -> String;

        #[wasm_bindgen(js_name = fileBytes)]
        pub fn file_bytes(file: &JsValue) -> Vec<u8>;

        /// Null if the user cancels.
        #[wasm_bindgen(catch, js_name = pickRecordFile)]
        pub async fn pick_record_file(
            extension: &str,
            append: bool,
            tail_bytes: u32,
        ) -> Result<JsValue, JsValue>;

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
