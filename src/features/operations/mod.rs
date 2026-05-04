//! Shared operation kinds for features that mix shell commands and package installs.

pub mod package;

pub use package::{detect_package_manager, install_invocation, run_ensure_package};
