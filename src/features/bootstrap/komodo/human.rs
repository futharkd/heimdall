use crate::features::bootstrap::komodo::report::BootstrapKomodoReport;
use crate::output::{Style, format_operation_report};

pub fn format_report_human(report: &BootstrapKomodoReport, style: &Style) -> String {
    format_operation_report("heimdall bootstrap komodo", &report.operations, style)
}
