use dioxus::prelude::*;

use ui::{AppPages, AppShell};

mod record_file;
mod serial;
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

#[component]
fn App() -> Element {
    use_context_provider(time::context);
    use_context_provider(serial::backend);

    rsx! {
        document::Link { rel: "icon", href: FAVICON, sizes: "16x16 32x32 48x48" }
        document::Link { rel: "icon", href: FAVICON_PNG, r#type: "image/png", sizes: "256x256" }

        AppShell {
            pages: AppPages {
                documents: Some(DOCUMENTS_PAGE.to_string()),
                licenses: LICENSES_PAGE.to_string(),
            },
            UnsupportedBrowserDialog {}
        }
    }
}
