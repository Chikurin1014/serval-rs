// No console beside the window, except while developing
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dioxus::{
    desktop::{
        Config, WindowBuilder, WindowCloseBehaviour,
        muda::{Menu, PredefinedMenuItem, Submenu},
        tao::{
            dpi::LogicalSize,
            event::{Event, WindowEvent},
            window::Icon,
        },
        use_wry_event_handler, window,
    },
    prelude::*,
};

use image::ImageFormat;
use ui::{AppPages, AppShell, Licenses, serial::SerialContext};

mod assets;
mod files;
mod serial;
mod time;

/// Its licenses, of its own dependencies (see `about.toml`): in it, so they
/// are there offline.
const LICENSES_PAGE: &str = include_str!("../third-party-licenses.html");

/// The web app's icon.
const ICON_PNG: &[u8] = include_bytes!("../../web/assets/favicon.png");

/// Room for the console beside the port's info, while it fits on a small
/// screen (e.g. 1366x768).
const WINDOW_SIZE: LogicalSize<f64> = LogicalSize::new(1200.0, 700.0);
const MIN_WINDOW_SIZE: LogicalSize<f64> = LogicalSize::new(480.0, 400.0);

fn main() {
    let _ = dioxus::logger::init(dioxus::logger::tracing::Level::WARN);
    let window = WindowBuilder::new()
        .with_title("Serval")
        .with_inner_size(WINDOW_SIZE)
        .with_min_inner_size(MIN_WINDOW_SIZE);
    let mut config = Config::new()
        .with_window(window)
        .with_menu(menu_bar())
        // Drags within the page (e.g. of graphs) as in a browser: on Windows,
        // the app's own handling of drops stops them. A file dropped on the
        // console is read by its script
        .with_disable_drag_drop_handler(true)
        // Nor is a file dropped elsewhere opened in place of the app
        .with_navigation_handler(|url| !url.starts_with("file:"));
    if let Some(icon) = icon() {
        config = config.with_icon(icon);
    }
    dioxus::LaunchBuilder::desktop()
        .with_cfg(config)
        .launch(App);
}

/// Its icon, of the window (and on Windows, the taskbar).
fn icon() -> Option<Icon> {
    let image = image::load_from_memory_with_format(ICON_PNG, ImageFormat::Png)
        .ok()?
        .into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).ok()
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
    use_context_provider(files::record_picker);

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

#[cfg(test)]
mod tests {
    #[test]
    fn has_its_icon() {
        assert!(super::icon().is_some());
    }
}
