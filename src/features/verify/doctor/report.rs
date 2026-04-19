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
