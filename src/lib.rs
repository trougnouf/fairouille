// File: src/lib.rs
pub mod cache;
pub mod client;
pub mod color_utils;
pub mod config;
pub mod journal;
pub mod model;
pub mod paths;
pub mod storage;
pub mod store;

#[cfg(feature = "tui")]
pub mod tui;

#[cfg(feature = "gui")]
pub mod gui;

// --- ANDROID SUPPORT ---
#[cfg(target_os = "android")]
pub mod mobile;

#[cfg(target_os = "android")]
uniffi::setup_scaffolding!();
