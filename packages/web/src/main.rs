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
const COMPONENTS_CSS: Asset = asset!("/assets/components.css");
const DX_COMPONENTS_THEME_CSS: Asset = asset!("/assets/dx-components-theme.css");

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
        document::Link { rel: "stylesheet", href: COMPONENTS_CSS }
        document::Link { rel: "stylesheet", href: DX_COMPONENTS_THEME_CSS }

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
            class: "app-shell",
            div {
                class: "navbar-shell",
                Navbar {
                    Link {
                        to: Route::Home {},
                        "Home"
                    }
                }
            }
            div {
                class: "route-shell",
                Outlet::<Route> {}
            }
        }
    }
}
