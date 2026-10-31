//! Utilities independent of the app.

mod script;

pub use logic::helper::*;
pub(crate) use script::{clear_file, concat_scripts, pick_file, use_script};
