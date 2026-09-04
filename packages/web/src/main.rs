use dioxus::prelude::*;

use ui::{data::DataProvider, Navbar};
use views::Home;

mod time;
mod views;
mod web_serial_api;

use time::TimeProvider;
use web_serial_api::SerialProvider;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(WebNavbar)]
    #[route("/")]
    Home {},
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const DAISYUI_CSS: &str = "https://cdn.jsdelivr.net/npm/daisyui@5";
const TAILWIND_CSS: &str = "https://cdn.jsdelivr.net/npm/@tailwindcss/browser@4";

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Build cool things ✌️

    rsx! {
        // Global app resources
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", type: "text/css", href: DAISYUI_CSS }
        document::Script { src: TAILWIND_CSS }

        TimeProvider {
            SerialProvider {
                DataProvider {
                    Router::<Route> {}
                }
            }
        }
    }
}

/// A web-specific Router around the shared `Navbar` component
/// which allows us to use the web-specific `Route` enum.
#[component]
fn WebNavbar() -> Element {
    rsx! {
        div {
            class: "flex h-screen flex-col overflow-hidden",
            div {
                class: "shrink-0",
                Navbar {
                    Link {
                        to: Route::Home {},
                        "Home"
                    }
                }
            }
            div {
                class: "min-h-0 flex-1 overflow-hidden",
                Outlet::<Route> {}
            }
        }
    }
}
