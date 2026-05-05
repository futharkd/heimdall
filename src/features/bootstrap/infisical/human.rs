use crate::core::operation::OperationStatus;
use crate::features::bootstrap::infisical::report::BootstrapInfisicalReport;
use crate::output::Style;

pub fn format_report_human(report: &BootstrapInfisicalReport, style: &Style) -> String {
    let mut lines = vec![];

    let sep = "─".repeat(48 - "heimdall bootstrap infisical".len());
    lines.push(format!(
        "\n{} {}\n",
        style.bold("heimdall bootstrap infisical"),
        style.dim(&sep)
    ));

    for op in &report.operations {
        let status_str = match op.status {
            OperationStatus::Planned => style.dim("[PLAN]"),
            OperationStatus::Succeeded => format!("{}  ", style.green("[OK]")),
            OperationStatus::Failed => style.red("[FAIL]"),
            OperationStatus::Skipped => format!("{}  ", style.dim("[SKIP]")),
        };

        lines.push(format!("{} {}", status_str, style.bold(op.description)));

        if !op.detail.is_empty() {
            lines.push(format!("  {}", style.dim(&op.detail)));
        }
    }

    lines.push(String::new());

    lines.join("\n")
}
