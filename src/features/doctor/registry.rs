//! Aggregates [`super::providers`] — register new probe modules in [`providers::collect_checks`](super::providers::collect_checks).

use crate::core::doctor::DoctorContext;

use super::providers;
use super::report::DoctorReport;

pub fn build_report(ctx: &DoctorContext) -> DoctorReport {
    DoctorReport {
        checks: providers::collect_checks(ctx),
    }
}
