use serde::Serialize;

use crate::core::operation::{OperationResult, OperationStatus};

#[derive(Debug, Clone, Serialize)]
pub struct UpdateReport {
    pub channel: String,
    pub binary_url: String,
    pub checksum_url: String,
    pub exe_path: String,
    pub local_digest: Option<String>,
    pub remote_digest: Option<String>,
    pub operations: Vec<OperationResult>,
}

impl UpdateReport {
    pub fn has_failures(&self) -> bool {
        self.operations
            .iter()
            .any(|operation| operation.status == OperationStatus::Failed)
    }
}
