use std::rc::Rc;

use dioxus::prelude::*;

use serial_runtime::{
    SerialBackend,
    backend::{extension, web},
};
use ui::{AppPages, AppShell, Licenses};

mod files;
mod time;
mod unsupported;

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

/// The VS Code extension's ports in it, else Web Serial's.
fn serial_backend() -> Rc<dyn SerialBackend> {
    if extension::is_active() {
        extension::backend()
    } else {
        web::backend()
    }
}

#[component]
fn App() -> Element {
    use_context_provider(time::context);
    use_context_provider(serial_backend);
    use_context_provider(files::record_picker);

    rsx! {
        document::Link { rel: "icon", href: FAVICON, sizes: "16x16 32x32 48x48" }
        document::Link { rel: "icon", href: FAVICON_PNG, r#type: "image/png", sizes: "256x256" }

        AppShell {
            pages: AppPages {
                documents: Some(DOCUMENTS_PAGE.to_string()),
                licenses: Licenses::Link(LICENSES_PAGE.to_string()),
            },
            UnsupportedBrowserDialog {}
        }
    }
}
