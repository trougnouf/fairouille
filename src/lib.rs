pub mod client;
pub mod config;
pub mod model;

// We include the UI module in the library,
// but ONLY if the "tui" feature is enabled.
#[cfg(feature = "tui")]
pub mod ui;
