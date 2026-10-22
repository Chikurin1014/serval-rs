use dioxus::{document::Eval, prelude::*};

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

/// Runs `script`, told `()` as the component is dropped, so it ends.
pub(crate) fn use_script(script: &'static str) -> Eval {
    let eval = use_hook(|| document::eval(script));
    use_drop(move || {
        let _ = eval.send(());
    });
    eval
}

/// Saves `text` as a file named `name`, of the media type `mime`.
pub(crate) fn download_text(name: &str, mime: &str, text: String) {
    let download = document::eval(DOWNLOAD_JS);
    let _ = download.send((name, mime, text));
}

/// Opens the file picker of the file input of `id`.
pub(crate) fn pick_file(id: &str) {
    document::eval(&format!("document.getElementById({id:?})?.click()"));
}

/// Empties the file input of `id`, so the same file picked again is a change.
pub(crate) fn clear_file(id: &str) {
    document::eval(&format!(
        "const input = document.getElementById({id:?}); if (input) input.value = ''"
    ));
}

/// The scripts `$files` (of `src/`) as one, in their order: a script after those
/// it uses (as `helper/script/`'s), from wherever it is used.
macro_rules! concat_scripts {
    ($($file:literal),+ $(,)?) => {
        concat!($(
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/", $file)),
            "\n",
        )+)
    };
}
pub(crate) use concat_scripts;
