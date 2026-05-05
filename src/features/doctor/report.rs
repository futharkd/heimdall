use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorCheck {
    pub id: &'static str,
    pub description: &'static str,
    pub status: CheckStatus,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheck>,
}

impl DoctorReport {
    pub fn has_failures(&self) -> bool {
        self.checks.iter().any(|c| c.status == CheckStatus::Fail)
    }
}

#[cfg(test)]
mod tests {
    use super::{CheckStatus, DoctorCheck, DoctorReport};

    #[test]
    fn report_detects_failures() {
        let report = DoctorReport {
            checks: vec![
                DoctorCheck {
                    id: "a",
                    description: "a",
                    status: CheckStatus::Pass,
                    detail: "ok".to_string(),
                },
                DoctorCheck {
                    id: "b",
                    description: "b",
                    status: CheckStatus::Fail,
                    detail: "no".to_string(),
                },
            ],
        };
        assert!(report.has_failures());
    }
}
