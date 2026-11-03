//! The serial ports of each platform, as the app opens them.

#[cfg(feature = "desktop")]
pub mod desktop;
/// The web app's in the VS Code extension.
#[cfg(feature = "web")]
pub mod extension;
#[cfg(feature = "web")]
pub mod web;
