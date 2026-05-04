use serde::Serialize;

use crate::core::operation::{OperationResult, OperationStatus};

#[derive(Debug, Serialize)]
pub struct BootstrapDockerReport {
    pub operations: Vec<OperationResult>,
}

impl BootstrapDockerReport {
    pub fn has_failures(&self) -> bool {
        self.operations
            .iter()
            .any(|op| op.status == OperationStatus::Failed)
    }
}
