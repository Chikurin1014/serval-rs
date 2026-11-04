use std::rc::Rc;

use dioxus::prelude::*;

use serial_runtime::{
    SerialBackend,
    backend::{bridge, web},
};
use ui::{AppPages, AppShell, Licenses};

mod files;
mod time;
mod unsupported;
mod vscode_files;

use unsupported::UnsupportedBrowserDialog;

const FAVICON_PNG: Asset = asset!("/assets/favicon.png");
const FAVICON: Asset = asset!("/assets/favicon.ico");
const LICENSES_PAGE: &str = "/third-party-licenses.html";
const DOCUMENTS_PAGE: &str = "/docs/";

fn main() {
    // Warnings and errors alone: on the web, each span and event logged is
    // also a `performance` mark and measure, kept until the page is closed,
    // and Dioxus's own (as each memo computes) would pile up as data comes
    let _ = dioxus::logger::init(dioxus::logger::tracing::Level::WARN);
    dioxus::launch(App);
}

/// The bridge's ports in a host that has one (as the VS Code extension), else
/// Web Serial's.
fn serial_backend() -> Rc<dyn SerialBackend> {
    if bridge::is_active() {
        bridge::backend()
    } else {
        web::backend()
    }
}

#[component]
fn App() -> Element {
    use_context_provider(time::context);
    use_context_provider(serial_backend);
    // The same each render, as the hooks must be
    // Only the VS Code extension hosts the app, so far
    let in_extension = bridge::is_active();
    if in_extension {
        // Files through VS Code's dialogs, not the browser's nor downloads
        use_context_provider(vscode_files::record_picker);
        use_context_provider(vscode_files::saver);
        use_context_provider(vscode_files::opener);
    } else {
        use_context_provider(files::record_picker);
    }

    rsx! {
        document::Link { rel: "icon", href: FAVICON, sizes: "16x16 32x32 48x48" }
        document::Link { rel: "icon", href: FAVICON_PNG, r#type: "image/png", sizes: "256x256" }

        AppShell {
            pages: AppPages {
                // Not in VS Code's panel: the documents are the web app's
                documents: (!in_extension).then(|| DOCUMENTS_PAGE.to_string()),
                // In VS Code, in a tab of its own (as `frame.js` asks)
                licenses: Licenses::Link(LICENSES_PAGE.to_string()),
            },
            // VS Code's panel has its own title: its tools in the toolbar's
            // menu, as on the desktop, though the toolbar is shown
            navbar: !in_extension,
            UnsupportedBrowserDialog {}
        }
    }
}
