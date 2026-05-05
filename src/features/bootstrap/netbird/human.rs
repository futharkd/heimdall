use crate::output::{Style, format_operation_report};

use super::report::BootstrapNetbirdReport;

pub fn format_report_human(report: &BootstrapNetbirdReport, style: &Style) -> String {
    format_operation_report("heimdall bootstrap netbird", &report.operations, style)
}
