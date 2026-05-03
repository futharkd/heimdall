use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Planned,
    Skipped,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct OperationResult {
    pub id: &'static str,
    pub description: &'static str,
    pub status: OperationStatus,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct PlannedOperation {
    pub id: &'static str,
    pub description: &'static str,
    pub command: String,
    pub args: Vec<String>,
    pub requires_confirmation: bool,
    pub stdin_input: Option<String>,
}
