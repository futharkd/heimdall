use crate::core::operation::{OperationResult, OperationStatus};

use super::{StatusTone, Style};

pub fn format_operation_report(
    title: &str,
    operations: &[OperationResult],
    style: &Style,
) -> String {
    let mut lines = Vec::with_capacity(operations.len() * 4 + 4);
    lines.push(style.command_heading_block(title));

    for operation in operations {
        let (label, tone) = match operation.status {
            OperationStatus::Planned => ("PLAN", StatusTone::Planned),
            OperationStatus::Skipped => ("SKIP", StatusTone::Skip),
            OperationStatus::Succeeded => ("OK", StatusTone::Ok),
            OperationStatus::Failed => ("FAIL", StatusTone::Fail),
        };
        let token = style.status_token(label, tone);
        lines.push(format!(
            "{token}  {}",
            style.bold(operation.description.as_str())
        ));
        let detail = operation.detail.trim();
        if !detail.is_empty() {
            for line in detail.lines() {
                lines.push(format!("         {}", style.dim(line)));
            }
        }
        lines.push(String::new());
    }

    lines.join("\n")
}

pub fn execution_footer_line(operations: &[OperationResult]) -> String {
    let mut planned = 0usize;
    let mut skipped = 0usize;
    let mut succeeded = 0usize;
    let mut failed = 0usize;
    for op in operations {
        match op.status {
            OperationStatus::Planned => planned += 1,
            OperationStatus::Skipped => skipped += 1,
            OperationStatus::Succeeded => succeeded += 1,
            OperationStatus::Failed => failed += 1,
        }
    }
    format!(
        "done: {succeeded} succeeded, {skipped} skipped, {failed} failed, {planned} planned of {} total",
        operations.len()
    )
}
