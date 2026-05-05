use crate::features::bootstrap::infisical::report::BootstrapInfisicalReport;
use crate::output::{Style, format_operation_report};

pub fn format_report_human(report: &BootstrapInfisicalReport, style: &Style) -> String {
    let mut output = String::new();

    // Add environment information if available
    if let Some(environment) = &report.environment {
        let env_display = match environment.as_str() {
            "prod" => "Production (prod)",
            "dev" => "Development (dev)",
            "staging" => "Staging (staging)",
            other => other,
        };
        output.push_str(&format!("Environment: {}\n\n", style.dim(env_display)));
    }

    // Add operation report
    output.push_str(&format_operation_report(
        "heimdall bootstrap infisical",
        &report.operations,
        style,
    ));
    output
}
