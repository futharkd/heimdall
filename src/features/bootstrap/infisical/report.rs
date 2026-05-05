use crate::core::operation::OperationResult;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct BootstrapInfisicalReport {
    pub operations: Vec<OperationResult>,
}

impl BootstrapInfisicalReport {
    pub fn has_failures(&self) -> bool {
        self.operations
            .iter()
            .any(|op| op.status == crate::core::operation::OperationStatus::Failed)
    }
}
