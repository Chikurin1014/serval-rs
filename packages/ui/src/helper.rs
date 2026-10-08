//! Utilities that do not depend on what the app is about: wrappers around
//! Dioxus, and text handling.

mod signal;
mod text;

pub(crate) use signal::make_owned;
pub use signal::set_if_changed;
pub(crate) use text::{csv_field, decode_utf8, format_bytes, single_line, unescape};
