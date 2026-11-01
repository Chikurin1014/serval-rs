//! Utilities independent of the app.

mod signal;
mod text;

pub use serial_runtime::Choice;
pub use signal::{make_owned, set_if_changed};
pub use text::{
    AnsiStripper, Delimiter, csv_field, decode_utf8, format_bytes, hex, single_line, split_lines,
    unescape, visible,
};
