//! The app as each platform shows it: its providers, the navbar and its page.
//! The platform gives its time and its serial ports, as contexts, and where
//! its pages are.

use std::rc::Rc;

use dioxus::{core::spawn_forever, prelude::*};
use dioxus_icons::lucide;

use crate::{
    ThemeProvider,
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        dialog::Dialog,
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
    time::TimeContext,
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
    pub licenses: Licenses,
}

/// The app's licenses page.
#[derive(Clone, Debug, PartialEq)]
pub enum Licenses {
    /// Opened in a new tab, of its URL
    Link(String),
    /// Shown over the app, of its HTML: so it is there offline
    Inline(&'static str),
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
    /// Whether inline licenses are shown
    licenses_shown: Signal<bool>,
}

/// Whether the app's tools (layouts, theme and plugins) and its licenses are
/// in the toolbar's menu: as there is no navbar for them.
pub(crate) fn tools_in_menu() -> bool {
    try_use_context::<ShownPages>().is_some_and(|shown| !shown.navbar)
}

/// As the toolbar's menu closes (its animation, with some to spare).
const MENU_CLOSED_MS: u32 = 250;

/// A link to the app's licenses: opening them in a new tab, or over the app.
/// Requires the app's pages.
#[component]
pub(crate) fn LicensesLink(class: String) -> Element {
    let ShownPages {
        pages,
        mut licenses_shown,
        ..
    } = use_context::<ShownPages>();
    let time = use_context::<TimeContext>();

    match pages.licenses {
        Licenses::Link(url) => rsx! {
            a {
                class,
                href: url,
                target: "_blank",
                rel: "noopener noreferrer",
                "Licenses"
                lucide::ExternalLink { size: 14 }
            }
        },
        Licenses::Inline(_) => rsx! {
            button {
                class,
                r#type: "button",
                onclick: move |_| {
                    // Once a menu it is in has closed and given back its
                    // focus, which would dismiss the dialog; past the menu
                    let wait = time.after_ms(MENU_CLOSED_MS);
                    spawn_forever(async move {
                        wait.await;
                        licenses_shown.set(true);
                    });
                },
                "Licenses"
            }
        },
    }
}

/// Inline licenses, over the app while shown.
#[component]
fn LicensesDialog() -> Element {
    let ShownPages {
        pages,
        mut licenses_shown,
        ..
    } = use_context::<ShownPages>();
    let Licenses::Inline(html) = pages.licenses else {
        return rsx! {};
    };

    rsx! {
        Dialog {
            class: "licenses-dialog",
            open: licenses_shown(),
            on_open_change: move |open: bool| licenses_shown.set(open),
            Button {
                class: "licenses-dialog-close",
                variant: ButtonVariant::Ghost,
                size: ButtonSize::IconSm,
                aria_label: "Close",
                title: "Close",
                onclick: move |_| licenses_shown.set(false),
                lucide::X {}
            }
            // Its own page, of its own style
            iframe {
                class: "licenses-dialog-page",
                title: "Licenses",
                srcdoc: html,
            }
        }
    }
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
    let licenses_shown = use_signal(|| false);
    use_context_provider(|| ShownPages {
        pages,
        navbar,
        licenses_shown,
    });
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
                                    LicensesDialog {}
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
    let ShownPages { pages, navbar, .. } = use_context::<ShownPages>();
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
                        if let Licenses::Link(licenses) = pages.licenses.clone() {
                            NavbarItem {
                                index: licenses_index,
                                value: "licenses".to_string(),
                                to: NavigationTarget::<Route>::External(licenses),
                                new_tab: true,
                                "Licenses"
                            }
                        }
                    }
                    GithubLink {}
                    div {
                        class: "navbar-tools",
                        if let Licenses::Inline(_) = pages.licenses {
                            LicensesLink { class: "navbar-licenses" }
                        }
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
