//! Files through VS Code's dialogs (`vscode_files.js`).

use wasm_bindgen::JsValue;

use serial_runtime::backend::web::message;
use ui::{
    save::{FileOpener, FileSaver, PickedFile},
    serial::{
        SerialResult,
        record::{RecordFile, RecordFilePicker, TAIL_BYTES},
    },
};

use crate::files;

pub fn saver() -> FileSaver {
    FileSaver::new(|name, text| {
        Box::pin(async move { js::save_file(&name, &text).await.map_err(message) })
    })
}

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

pub fn record_picker() -> RecordFilePicker {
    RecordFilePicker::new(|extension, append| {
        Box::pin(async move { open_record(&extension, append).await })
    })
}

async fn open_record(extension: &str, append: bool) -> SerialResult<Option<RecordFile>> {
    let name = files::record_name(extension);
    let sink: JsValue = js::pick_record_file(&name, append, TAIL_BYTES as u32)
        .await
        .map_err(message)?;
    Ok((!sink.is_null()).then(|| files::record_of(sink)))
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

        /// A sink for `recording`, or null if the user cancels.
        #[wasm_bindgen(catch, js_name = pickRecordFile)]
        pub async fn pick_record_file(
            name: &str,
            append: bool,
            tail_bytes: u32,
        ) -> Result<JsValue, JsValue>;
    }
}
