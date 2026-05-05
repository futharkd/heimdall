use crate::features::bootstrap::infisical::report::BootstrapInfisicalReport;
use crate::output::{Style, format_operation_report};

pub fn format_report_human(report: &BootstrapInfisicalReport, style: &Style) -> String {
    format_operation_report("heimdall bootstrap infisical", &report.operations, style)
}
