//! The app as each platform shows it: its providers, the navbar and its page.
//! The platform gives its time and its serial ports, as contexts, and where
//! its pages are.

use std::rc::Rc;

use dioxus::prelude::*;

use crate::{
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
    serial::{SerialBackend, use_serial_provider},
    theme::ThemeSwitch,
    views::Home,
};

mod github_link;

use github_link::GithubLink;

const APP_SHELL_CSS: Asset = asset!("/assets/styling/app-shell.css");

/// Where the pages beside the app are: each opened in a new tab (or out of the
/// app), so the open port stays.
#[derive(Clone, Debug, PartialEq)]
pub struct AppPages {
    /// None to leave them to the web app
    pub documents: Option<String>,
    pub licenses: String,
}

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(AppNavbar)]
    #[route("/")]
    Home {},
}

/// The app's pages, and whether the navbar is shown with them.
#[derive(Clone)]
struct ShownPages {
    pages: AppPages,
    navbar: bool,
}

/// Whether the app's tools (layouts, theme and plugins) and its licenses are
/// in the toolbar's menu: as there is no navbar for them.
pub(crate) fn tools_in_menu() -> bool {
    try_use_context::<ShownPages>().is_some_and(|shown| !shown.navbar)
}

/// Where the app's licenses are.
pub(crate) fn licenses_page() -> Option<String> {
    try_use_context::<ShownPages>().map(|shown| shown.pages.licenses)
}

/// The app; `children` (e.g. a dialog of its own) over it, with all its
/// contexts. Requires the platform's `TimeContext` and `Rc<dyn SerialBackend>`.
/// With no `navbar` (e.g. for room on a desktop), its tools and licenses are
/// in the toolbar's menu.
#[component]
pub fn AppShell(
    pages: AppPages,
    #[props(default = true)] navbar: bool,
    children: Element,
) -> Element {
    use_context_provider(|| ShownPages { pages, navbar });
    let layout = use_hook(|| initial_layout(&builtin_map_kinds(), &builtin_graph_kinds()));

    rsx! {
        ThemeProvider {
            ToastProvider {
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
                                    RecordUnloadGuard {}
                                    {children}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Of the platform's backend. Requires `DataContext` and `TimeContext`.
#[component]
fn SerialProvider(children: Element) -> Element {
    let backend = use_context::<Rc<dyn SerialBackend>>();
    use_serial_provider(|| backend);
    children
}

/// The page, under the navbar if it is shown.
#[component]
fn AppNavbar() -> Element {
    let ShownPages { pages, navbar } = use_context::<ShownPages>();
    let licenses_index = 1 + usize::from(pages.documents.is_some());

    rsx! {
        document::Link { rel: "stylesheet", href: APP_SHELL_CSS }

        div {
            class: "app-shell",
            if navbar {
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
                        if let Some(documents) = pages.documents {
                            NavbarItem {
                                index: 1usize,
                                value: "documents".to_string(),
                                to: NavigationTarget::<Route>::External(documents),
                                new_tab: true,
                                "Documents"
                            }
                        }
                        NavbarItem {
                            index: licenses_index,
                            value: "licenses".to_string(),
                            to: NavigationTarget::<Route>::External(pages.licenses),
                            new_tab: true,
                            "Licenses"
                        }
                    }
                    GithubLink {}
                    div {
                        class: "navbar-tools",
                        AppTools {}
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

/// Layouts, the theme and plugins: in the navbar, or else in the toolbar's
/// menu.
#[component]
pub(crate) fn AppTools() -> Element {
    rsx! {
        LayoutMenu {}
        ThemeSwitch {}
        PluginButton {}
    }
}
