use dioxus::prelude::*;

use ui::{
    ThemeProvider,
    components::{
        navbar::{Navbar, NavbarItem},
        toast::ToastProvider,
    },
    data::{DataProvider, MapProvider},
    elements::{
        graph::{GraphProvider, builtin_graph_kinds, initial_graphs},
        map::{builtin_map_kinds, initial_maps},
    },
    theme::ThemeSwitch,
    views::Home,
};

mod serial;
mod time;
mod unsupported;

use serial::SerialProvider;
use time::TimeProvider;
use unsupported::UnsupportedBrowserDialog;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(WebNavbar)]
    #[route("/")]
    Home {},
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const WEB_NAVBAR_CSS: Asset = asset!("/assets/styling/web-navbar.css");
/// The third-party licenses, copied from `public/` as it is
const LICENSES_PAGE: &str = "/third-party-licenses.html";

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
            // Outermost after the theme, so any provider below can show toasts
            ToastProvider {
                TimeProvider {
                    DataProvider {
                        SerialProvider {
                            MapProvider {
                                kinds: builtin_map_kinds(),
                                initial: initial_maps(),
                                GraphProvider {
                                    kinds: builtin_graph_kinds(),
                                    initial: initial_graphs(),
                                    Router::<Route> {}
                                    UnsupportedBrowserDialog {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The page: the navbar with the routes' links and the theme switch, then the
/// route's view.
#[component]
fn WebNavbar() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: WEB_NAVBAR_CSS }

        div {
            class: "app-shell",
            div {
                class: "navbar-shell",
                Navbar {
                    aria_label: "Pages",
                    NavbarItem {
                        index: 0usize,
                        value: "home".to_string(),
                        to: Route::Home {},
                        "Home"
                    }
                    // A static page beside the app (see `about.toml`); in a new
                    // tab, so the open port and its data stay
                    NavbarItem {
                        index: 1usize,
                        value: "licenses".to_string(),
                        to: NavigationTarget::<Route>::External(LICENSES_PAGE.to_string()),
                        new_tab: true,
                        "Licenses"
                    }
                }
                ThemeSwitch {}
            }
            div {
                class: "route-shell",
                Outlet::<Route> {}
            }
        }
    }
}
