use crate::core::operation::OperationResult;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct BootstrapKomodoReport {
    pub operations: Vec<OperationResult>,
}

impl BootstrapKomodoReport {
    pub fn has_failures(&self) -> bool {
        self.operations
            .iter()
            .any(|op| op.status == crate::core::operation::OperationStatus::Failed)
    }
}
