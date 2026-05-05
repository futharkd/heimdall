use crate::output::{Style, format_operation_report};

use super::report::BootstrapFluxReport;

pub fn format_report_human(report: &BootstrapFluxReport, style: &Style) -> String {
    format_operation_report("heimdall bootstrap flux", &report.operations, style)
}
