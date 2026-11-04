//! The serial ports of each platform, as the app opens them.

/// The web app's in a page that hosts it (as the VS Code extension's panel),
/// through the `serial-bridge` program.
#[cfg(feature = "web")]
pub mod bridge;
#[cfg(feature = "desktop")]
pub mod desktop;
#[cfg(feature = "web")]
pub mod web;
