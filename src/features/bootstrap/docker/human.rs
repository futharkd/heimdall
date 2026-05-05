use crate::output::{Style, format_operation_report};

use super::report::BootstrapDockerReport;

pub fn format_report_human(report: &BootstrapDockerReport, style: &Style) -> String {
    format_operation_report("heimdall bootstrap docker", &report.operations, style)
}
