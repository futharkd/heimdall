use crate::modules::doctor::{CheckStatus, DoctorReport};

pub fn render_doctor_human(report: &DoctorReport) -> String {
    let mut lines = Vec::with_capacity(report.checks.len() + 1);
    lines.push("heimdall doctor report".to_string());

    for check in &report.checks {
        let icon = match check.status {
            CheckStatus::Pass => "PASS",
            CheckStatus::Warn => "WARN",
            CheckStatus::Fail => "FAIL",
        };
        lines.push(format!(
            "- [{icon}] {}: {}",
            check.description, check.detail
        ));
    }

    lines.join("\n")
}
