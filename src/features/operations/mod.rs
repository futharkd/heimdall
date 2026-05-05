//! Shared operation kinds for features that mix shell commands and package installs.

pub mod package;

pub use package::run_ensure_package;
