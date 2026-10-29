//! Utilities independent of the app.

mod choice;
mod script;
mod signal;
mod text;

pub use choice::Choice;
pub(crate) use script::{clear_file, concat_scripts, download_text, pick_file, use_script};
pub(crate) use signal::make_owned;
pub use signal::set_if_changed;
pub use text::Delimiter;
pub(crate) use text::{
    AnsiStripper, csv_field, decode_utf8, format_bytes, hex, single_line, split_lines, unescape,
    visible,
};
