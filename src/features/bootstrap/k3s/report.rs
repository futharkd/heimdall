use serde::Serialize;

use crate::core::operation::OperationResult;

#[derive(Debug, Clone, Serialize)]
pub struct BootstrapK3sReport {
    pub operations: Vec<OperationResult>,
}

impl BootstrapK3sReport {
    pub fn has_failures(&self) -> bool {
        self.operations
            .iter()
            .any(|operation| operation.status == crate::core::operation::OperationStatus::Failed)
    }
}
