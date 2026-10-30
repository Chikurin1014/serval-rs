use dioxus::{
    desktop::{
        Config, WindowBuilder, WindowCloseBehaviour,
        muda::{Menu, PredefinedMenuItem, Submenu},
        tao::event::{Event, WindowEvent},
        use_wry_event_handler, window,
    },
    prelude::*,
};

use ui::{AppPages, AppShell, Licenses, serial::SerialContext};

mod assets;
mod files;
mod serial;
mod time;

/// Its licenses, of its own dependencies (see `about.toml`): in it, so they
/// are there offline.
const LICENSES_PAGE: &str = include_str!("../../../third-party-licenses-desktop.html");

fn main() {
    let _ = dioxus::logger::init(dioxus::logger::tracing::Level::WARN);
    let window = WindowBuilder::new().with_title("Serval");
    let config = Config::new()
        .with_window(window)
        .with_menu(menu_bar())
        // Drags within the page (e.g. of graphs) as in a browser: on Windows,
        // the app's own handling of drops stops them. A file dropped on the
        // console is read by its script
        .with_disable_drag_drop_handler(true)
        // Nor is a file dropped elsewhere opened in place of the app
        .with_navigation_handler(|url| !url.starts_with("file:"));
    dioxus::LaunchBuilder::desktop()
        .with_cfg(config)
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
    use_context_provider(files::saver);

    rsx! {
        if assets::embedded() {
            assets::EmbeddedAssets {}
        }
        AppShell {
            pages: AppPages {
                documents: None,
                licenses: Licenses::Inline(LICENSES_PAGE),
            },
            navbar: false,
            RecordCloseGuard {}
        }
    }
}

/// Ends the record as the app closes, so its file has all of it (with its
/// unfinished line): while one is written, closing the window hides it, and
/// it closes once the file is done.
#[component]
fn RecordCloseGuard() -> Element {
    let serial = use_context::<SerialContext>();
    use_effect(move || {
        window().set_close_behavior(if serial.record_format().is_some() {
            WindowCloseBehaviour::WindowHides
        } else {
            WindowCloseBehaviour::WindowCloses
        });
    });
    use_wry_event_handler(move |event, _| {
        let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        else {
            return;
        };
        if let Some(closed) = serial.finish_record() {
            spawn(async move {
                let _ = closed.await;
                let window = window();
                window.set_close_behavior(WindowCloseBehaviour::WindowCloses);
                window.close();
            });
        }
    });
    rsx! {}
}
