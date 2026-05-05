use serde::Serialize;
use std::path::PathBuf;

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
    pub description: String,
    pub status: OperationStatus,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    Shell {
        command: String,
        args: Vec<String>,
        env: Vec<(String, String)>,
        stdin_input: Option<String>,
    },
    EnsurePackage {
        package: String,
    },
    WriteFile {
        path: PathBuf,
        content: String,
        mode: Option<u32>,
    },
    #[allow(dead_code)]
    InheritIo {
        command: String,
        args: Vec<String>,
    },
}

impl OperationKind {
    pub fn is_read_only(&self) -> bool {
        match self {
            OperationKind::Shell { command, args, .. } => {
                let command = command.as_str();
                let first = args.first().map(String::as_str).unwrap_or("");
                matches!(command, "grep" | "test" | "cat" | "ls" | "which")
                    || (command == "command" && first == "-v")
                    || (command == "kubectl" && matches!(first, "get" | "describe"))
            }
            OperationKind::EnsurePackage { .. } => false,
            OperationKind::WriteFile { .. } => false,
            OperationKind::InheritIo { .. } => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VerifyStep {
    pub description: String,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationSlot {
    After,
}

#[derive(Debug, Clone)]
pub struct PlannedOperation {
    pub id: &'static str,
    pub description: String,
    pub kind: OperationKind,
    pub requires_confirmation: bool,
    pub failure_is_warning: bool,
    pub verify: Option<VerifyStep>,
}

impl PlannedOperation {
    pub fn validation_slot(&self) -> Option<ValidationSlot> {
        self.verify.as_ref().map(|_| ValidationSlot::After)
    }
}
