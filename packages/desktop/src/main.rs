use dioxus::prelude::*;

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

/// Serval runs in the browser for now (it uses Web Serial), so the desktop app
/// only says so.
#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        p { "Serval is not available on desktop yet." }
    }
}
