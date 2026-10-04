use dioxus::prelude::*;

use ui::{
    Navbar, ThemeProvider,
    data::{ConversionProvider, DataProvider},
    elements::{builtin_conversion_kinds, initial_conversions},
    graph::{GraphProvider, builtin_graph_kinds, initial_graphs},
    views::Home,
};

mod serial;
mod time;

use serial::SerialProvider;
use time::TimeProvider;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(WebNavbar)]
    #[route("/")]
    Home {},
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const WEB_NAVBAR_CSS: Asset = asset!("/assets/styling/web-navbar.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Build cool things ✌️

    rsx! {
        // Global app resources
        document::Link { rel: "icon", href: FAVICON }

        ThemeProvider {
            TimeProvider {
                SerialProvider {
                    DataProvider {
                        ConversionProvider {
                            kinds: builtin_conversion_kinds(),
                            initial: initial_conversions(),
                            GraphProvider {
                                kinds: builtin_graph_kinds(),
                                initial: initial_graphs(),
                                Router::<Route> {}
                            }
                        }
                    }
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
        document::Link { rel: "stylesheet", href: WEB_NAVBAR_CSS }

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
