//! Shared context for read-only [`crate::features::doctor`] diagnostics.

use crate::runner::{CommandRunner, IoMode};

/// Inputs for registered doctor probes (subprocess + streaming mode).
pub struct DoctorContext<'a> {
    pub runner: &'a dyn CommandRunner,
    pub io_mode: IoMode,
}
