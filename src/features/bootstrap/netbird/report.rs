use serde::Serialize;

use crate::core::operation::OperationResult;

#[derive(Debug, Clone, Serialize)]
pub struct BootstrapNetbirdReport {
    pub operations: Vec<OperationResult>,
}

impl BootstrapNetbirdReport {
    pub fn has_failures(&self) -> bool {
        self.operations
            .iter()
            .any(|operation| operation.status == crate::core::operation::OperationStatus::Failed)
    }
}
