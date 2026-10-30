use std::{path::Path, sync::LazyLock};

use dioxus::{
    desktop::{
        use_asset_handler,
        wry::http::{Response, StatusCode, header},
    },
    prelude::*,
};
use embed_assets::Assets;

/// The assets embedded in the app's binary (by `embed-assets`), if they are.
static EMBEDDED: LazyLock<Option<Assets>> = LazyLock::new(|| {
    let exe = std::env::current_exe().ok()?;
    Assets::read(&exe).ok().flatten()
});

/// Whether the assets are in the app's binary, not in a folder beside it.
pub fn embedded() -> bool {
    EMBEDDED.is_some()
}

/// Serves the assets from the app's binary. Rendered first, so they are
/// served from it as the app first asks for them.
#[component]
pub fn EmbeddedAssets() -> Element {
    use_asset_handler("assets", |request, responder| {
        let path = request.uri().path().trim_start_matches('/');
        let response = match EMBEDDED.as_ref().and_then(|assets| assets.get(path)) {
            Some(data) => Response::builder()
                .header(header::CONTENT_TYPE, mime(path))
                .body(data),
            None => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(&[][..]),
        };
        if let Ok(response) = response {
            responder.respond(response);
        }
    });

    rsx! {}
}

/// The type of the asset at `path`, by its extension.
fn mime(path: &str) -> &'static str {
    match Path::new(path).extension().and_then(|ext| ext.to_str()) {
        Some("css") => "text/css",
        Some("js" | "mjs") => "text/javascript",
        Some("json") => "application/json",
        Some("wasm") => "application/wasm",
        Some("html") => "text/html",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("ttf") => "font/ttf",
        _ => "application/octet-stream",
    }
}
