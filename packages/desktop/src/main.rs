use dioxus::{
    desktop::{
        Config, WindowBuilder,
        muda::{Menu, PredefinedMenuItem, Submenu},
    },
    prelude::*,
};

use ui::{AppPages, AppShell};

mod assets;
mod serial;
mod time;

/// The web app's licenses, until they are in the desktop app: in the browser,
/// as the app opens any page on the web.
const LICENSES_PAGE: &str = "https://serval-5qv.pages.dev/third-party-licenses.html";

fn main() {
    let _ = dioxus::logger::init(dioxus::logger::tracing::Level::WARN);
    let window = WindowBuilder::new().with_title("Serval");
    dioxus::LaunchBuilder::desktop()
        .with_cfg(Config::new().with_window(window).with_menu(menu_bar()))
        .launch(App);
}

/// None, for room: all is in the toolbar's menu. On macOS, the app's menu and
/// Edit, as its shortcuts (e.g. to copy) are of the menu there, which is out
/// of the window.
fn menu_bar() -> Option<Menu> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let menu = Menu::new();
    let app = Submenu::with_items(
        "Serval",
        true,
        &[
            &PredefinedMenuItem::hide(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::quit(None),
        ],
    );
    let edit = Submenu::with_items(
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(None),
            &PredefinedMenuItem::redo(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::cut(None),
            &PredefinedMenuItem::copy(None),
            &PredefinedMenuItem::paste(None),
            &PredefinedMenuItem::select_all(None),
        ],
    );
    for submenu in [app, edit].iter().flatten() {
        let _ = menu.append(submenu);
    }
    Some(menu)
}

#[component]
fn App() -> Element {
    use_context_provider(time::context);
    use_context_provider(serial::backend);

    rsx! {
        if assets::embedded() {
            assets::EmbeddedAssets {}
        }
        AppShell {
            pages: AppPages {
                documents: None,
                licenses: LICENSES_PAGE.to_string(),
            },
            navbar: false,
        }
    }
}
