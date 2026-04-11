//! # Overview
//! 
//! Core Package of Karbeat. Handles the Audio Thread and Business Logic of the App.
//! This package also includes the Core API that is generic and reusable
//! for any kind of UI implementation usage.

pub mod api;
pub mod audio;
pub mod commands;
pub mod context;
pub mod core;
pub mod lock;
pub mod plugin_types;
pub mod test;
pub mod utils;
