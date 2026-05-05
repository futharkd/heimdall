//! Shared output primitives (styling). Per-feature human formatting lives under `src/features/.../human.rs`.

pub mod human;
pub mod style;

pub use human::{execution_footer_line, format_operation_report};
pub use style::{ColorArg, StatusTone, Style};
