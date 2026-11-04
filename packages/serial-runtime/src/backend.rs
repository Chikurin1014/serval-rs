//! The serial ports of each platform, as the app opens them.

#[cfg(feature = "web")]
pub mod bridge;
#[cfg(feature = "desktop")]
pub mod desktop;
#[cfg(feature = "web")]
pub mod web;
