use dioxus::{
    desktop::{
        Config, WindowBuilder,
        muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
        use_muda_event_handler,
    },
    prelude::*,
};

use ui::AppShell;

mod serial;
mod time;

/// The pages of the Help menu, by their items' ids: in the browser. The web
/// app's documents and licenses, until they are in the desktop app.
const PAGES: [(&str, &str, &str); 3] = [
    (
        "documents",
        "Documents",
        "https://serval-5qv.pages.dev/docs/",
    ),
    (
        "licenses",
        "Licenses",
        "https://serval-5qv.pages.dev/third-party-licenses.html",
    ),
    (
        "github",
        "Serval on GitHub",
        "https://github.com/Chikurin1014/serval-rs",
    ),
];

fn main() {
    let _ = dioxus::logger::init(dioxus::logger::tracing::Level::WARN);
    let window = WindowBuilder::new().with_title("Serval");
    dioxus::LaunchBuilder::desktop()
        .with_cfg(Config::new().with_window(window).with_menu(menu_bar()))
        .launch(App);
}

/// Help, of the pages; and on macOS, the app's menu and Edit, as its
/// shortcuts (e.g. to copy) are of the menu there.
fn menu_bar() -> Menu {
    let menu = Menu::new();
    if cfg!(target_os = "macos") {
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
    }
    let help = Submenu::new("Help", true);
    for (id, name, _) in PAGES {
        let _ = help.append(&MenuItem::with_id(id, name, true, None));
    }
    let _ = menu.append(&help);
    menu
}

#[component]
fn App() -> Element {
    use_context_provider(time::context);
    use_context_provider(serial::backend);
    use_muda_event_handler(|event: &MenuEvent| {
        if let Some((.., url)) = PAGES.iter().find(|(id, ..)| event.id == *id) {
            let _ = webbrowser::open(url);
        }
    });

    rsx! {
        AppShell {}
    }
}
