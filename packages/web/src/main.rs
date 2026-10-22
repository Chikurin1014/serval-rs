use dioxus::prelude::*;

use ui::{
    ThemeProvider,
    components::{
        navbar::{Navbar, NavbarItem},
        toast::ToastProvider,
    },
    data::{DataProvider, MapProvider},
    elements::{
        LayoutMenu, PluginButton,
        graph::{GraphProvider, builtin_graph_kinds},
        initial_layout,
        map::builtin_map_kinds,
        serial::RecordUnloadGuard,
    },
    plugin::TaskProvider,
    theme::ThemeSwitch,
    views::Home,
};

mod github_link;
mod record_file;
mod serial;
mod time;
mod unsupported;

use github_link::GithubLink;
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

const FAVICON_PNG: Asset = asset!("/assets/favicon.png");
const FAVICON: Asset = asset!("/assets/favicon.ico");
const WEB_NAVBAR_CSS: Asset = asset!("/assets/styling/web-navbar.css");
const LICENSES_PAGE: &str = "/third-party-licenses.html";

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let layout = use_hook(|| initial_layout(&builtin_map_kinds(), &builtin_graph_kinds()));

    rsx! {
        document::Link { rel: "icon", href: FAVICON, sizes: "16x16 32x32 48x48" }
        document::Link { rel: "icon", href: FAVICON_PNG, r#type: "image/png", sizes: "256x256" }

        ThemeProvider {
            ToastProvider {
                TimeProvider {
                    DataProvider {
                        SerialProvider {
                            MapProvider {
                                kinds: builtin_map_kinds(),
                                initial: layout.maps,
                                GraphProvider {
                                    kinds: builtin_graph_kinds(),
                                    initial: layout.graphs,
                                    TaskProvider {
                                        Router::<Route> {}
                                        UnsupportedBrowserDialog {}
                                        RecordUnloadGuard {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

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
                        // The page's title (in time, its logo)
                        span { class: "navbar-title", "Serval" }
                    }
                    // In a new tab, so the open port stays
                    NavbarItem {
                        index: 1usize,
                        value: "licenses".to_string(),
                        to: NavigationTarget::<Route>::External(LICENSES_PAGE.to_string()),
                        new_tab: true,
                        "Licenses"
                    }
                }
                GithubLink {}
                div {
                    class: "navbar-tools",
                    LayoutMenu {}
                    ThemeSwitch {}
                    PluginButton {}
                }
            }
            div {
                class: "route-shell",
                Outlet::<Route> {}
            }
        }
    }
}
