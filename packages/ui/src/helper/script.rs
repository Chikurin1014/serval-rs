use dioxus::{core::spawn_forever, document::Eval, prelude::*};

use crate::{
    save::{FileOpener, PickedFile},
    toast::{consume_toaster, use_toaster},
};

/// Runs `script`, told `()` as the component is dropped, so it ends.
pub(crate) fn use_script(script: &'static str) -> Eval {
    let eval = use_hook(|| document::eval(script));
    use_drop(move || {
        let _ = eval.send(());
    });
    eval
}

/// Picks a file: through the platform's [`FileOpener`] if it has one,
/// `on_pick` then given it, else through the file input of `id` (whose change
/// takes it). `extensions` are those it may have (any if none).
pub(crate) fn pick_file(
    id: &str,
    extensions: &'static [&'static str],
    on_pick: impl FnOnce(PickedFile) + 'static,
) {
    if !open_picked_file(extensions, on_pick) {
        document::eval(&format!("document.getElementById({id:?})?.click()"));
    }
}

/// Picks a file through the platform's [`FileOpener`], `on_pick` then given
/// it; false if it has none (for the page's file inputs).
pub(crate) fn open_picked_file(
    extensions: &'static [&'static str],
    on_pick: impl FnOnce(PickedFile) + 'static,
) -> bool {
    let Some(opener) = try_consume_context::<FileOpener>() else {
        return false;
    };
    let toaster = consume_toaster();
    let picked = opener.open(extensions);
    // Past the component asking (e.g. in a menu that closes)
    spawn_forever(async move {
        match picked.await {
            Ok(Some(file)) => on_pick(file),
            Ok(None) => {}
            Err(error) => toaster.error("Failed to open the file", &error),
        }
    });
    true
}

/// A file input of `id`, as [`pick_file`] opens it (or a label of it): the file
/// picked is read and given to `on_pick`.
#[component]
pub(crate) fn FileInput(
    id: &'static str,
    class: &'static str,
    accept: Option<&'static str>,
    aria_label: Option<&'static str>,
    #[props(default)] disabled: bool,
    on_pick: EventHandler<PickedFile>,
) -> Element {
    let toaster = use_toaster();
    rsx! {
        input {
            id,
            class,
            r#type: "file",
            accept,
            aria_label,
            disabled,
            onchange: move |event: FormEvent| async move {
                let Some(file) = event.files().into_iter().next() else {
                    return;
                };
                let bytes = file.read_bytes().await;
                // So the same file picked again is a change
                clear_file(id);
                match bytes {
                    Ok(bytes) => on_pick.call(PickedFile { name: file.name(), bytes: bytes.to_vec() }),
                    Err(error) => toaster.error("Failed to open the file", &error.to_string()),
                }
            },
        }
    }
}

/// Empties the file input of `id`, so the same file picked again is a change.
fn clear_file(id: &str) {
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
