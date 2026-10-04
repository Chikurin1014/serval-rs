//! This crate contains all shared UI for the workspace.

mod hero;
pub use hero::Hero;

mod navbar;
pub use navbar::Navbar;

pub mod components;
pub mod data;
pub mod elements;
pub mod graph;
pub mod serial;
pub mod theme;
pub mod time;
pub mod views;

pub use theme::ThemeProvider;
pub use time::TimeContext;
