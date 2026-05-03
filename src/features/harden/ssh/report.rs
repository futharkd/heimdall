use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct OperationResultOwned {
    pub id: String,
    pub description: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct HardenSshReport {
    pub operations: Vec<OperationResultOwned>,
}

impl HardenSshReport {
    pub fn has_failures(&self) -> bool {
        self.operations
            .iter()
            .any(|op| op.status == "failed")
    }
}
