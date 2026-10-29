use dioxus::{
    desktop::{Config, WindowBuilder},
    prelude::*,
};

use ui::{AppPages, AppShell};

mod serial;
mod time;

/// The web app's, until they are in the desktop app.
const DOCUMENTS_PAGE: &str = "https://serval-5qv.pages.dev/docs/";
const LICENSES_PAGE: &str = "https://serval-5qv.pages.dev/third-party-licenses.html";

fn main() {
    let _ = dioxus::logger::init(dioxus::logger::tracing::Level::WARN);
    let window = WindowBuilder::new().with_title("Serval");
    dioxus::LaunchBuilder::desktop()
        .with_cfg(Config::new().with_window(window).with_menu(None))
        .launch(App);
}

#[component]
fn App() -> Element {
    use_context_provider(time::context);
    use_context_provider(serial::backend);

    rsx! {
        AppShell {
            pages: AppPages {
                documents: DOCUMENTS_PAGE.to_string(),
                licenses: LICENSES_PAGE.to_string(),
            },
        }
    }
}
