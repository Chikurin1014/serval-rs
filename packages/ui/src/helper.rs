//! Utilities independent of the app.

mod script;

pub use logic::helper::*;
pub(crate) use script::{clear_file, concat_scripts, open_picked_file, pick_file, use_script};
