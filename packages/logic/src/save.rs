//! Saving files the app makes (e.g. a layout or a CSV): downloaded, or as the
//! platform saves them. And opening those it takes, as the platform opens them
//! if it has its own way.

use std::rc::Rc;

use dioxus::{core::spawn_forever, prelude::*};

use crate::{
    serial::{LocalFuture, SerialResult},
    toast::consume_toaster,
};

const DOWNLOAD_JS: &str = r#"
const [name, type, text] = await dioxus.recv();
const url = URL.createObjectURL(new Blob([text], { type }));
const link = document.createElement("a");
link.href = url;
link.download = name;
document.body.append(link);
link.click();
link.remove();
// Once the download has taken it
setTimeout(() => URL.revokeObjectURL(url), 0);
"#;

/// Saves a file in the platform's own way (e.g. through a save dialog), in
/// place of a download: given the name it suggests and its text. Provided as a
/// context by the platform; with none, files are downloaded.
#[derive(Clone)]
pub struct FileSaver(Rc<dyn Fn(String, String) -> LocalFuture<SerialResult<()>>>);

impl FileSaver {
    pub fn new(save: impl Fn(String, String) -> LocalFuture<SerialResult<()>> + 'static) -> Self {
        Self(Rc::new(save))
    }
}

/// Saves `text` as a file named `name`, of the media type `mime`; a failure
/// shown as a toast.
pub fn save_text(name: &str, mime: &str, text: String) {
    let Some(FileSaver(save)) = try_consume_context::<FileSaver>() else {
        let download = document::eval(DOWNLOAD_JS);
        let _ = download.send((name, mime, text));
        return;
    };
    let toaster = consume_toaster();
    let saved = save(name.to_string(), text);
    // Past the component asking (e.g. in a menu that closes)
    spawn_forever(async move {
        if let Err(error) = saved.await {
            toaster.error("Failed to save the file", &error);
        }
    });
}

/// A file the user picked: its name and bytes.
pub struct PickedFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// Opens a file the user picks in the platform's own way (e.g. through its
/// open dialog), in place of the page's file inputs: given the extensions it
/// may have (e.g. `json`; any if none). `None` if they cancel. Provided as a
/// context by the platform; with none, the file inputs are used.
#[derive(Clone)]
pub struct FileOpener(Rc<dyn Fn(Vec<String>) -> OpenedFile>);

type OpenedFile = LocalFuture<SerialResult<Option<PickedFile>>>;

impl FileOpener {
    pub fn new(open: impl Fn(Vec<String>) -> OpenedFile + 'static) -> Self {
        Self(Rc::new(open))
    }

    pub fn open(&self, extensions: &[&str]) -> OpenedFile {
        (self.0)(
            extensions
                .iter()
                .map(|extension| extension.to_string())
                .collect(),
        )
    }
}
