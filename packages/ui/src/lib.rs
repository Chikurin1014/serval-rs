//! This crate contains all shared UI for the workspace.

mod hero;
pub use hero::Hero;

mod navbar;
pub use navbar::Navbar;

pub mod component;
pub mod data;
pub mod serial;
pub mod time;

pub use time::TimeContext;
