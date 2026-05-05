//! Shared context for read-only [`crate::features::doctor`] diagnostics.

use crate::core::elevation::PrivilegeContext;
use crate::runner::{CommandRunner, IoMode};

/// Inputs for registered doctor probes (subprocess + streaming mode).
pub struct DoctorContext<'a> {
    pub runner: &'a dyn CommandRunner,
    pub io_mode: IoMode,
    /// Session policy for probes (doctor defaults to [`PrivilegeContext::USER_SESSION`]).
    #[allow(dead_code)]
    pub privilege: PrivilegeContext,
}
