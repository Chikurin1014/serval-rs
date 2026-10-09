//! Utilities independent of the app.

mod signal;
mod text;

pub(crate) use signal::make_owned;
pub use signal::set_if_changed;
pub use text::Delimiter;
pub(crate) use text::{
    AnsiStripper, csv_field, decode_utf8, format_bytes, hex, single_line, split_lines, unescape,
    visible,
};
