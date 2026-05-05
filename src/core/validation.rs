use std::future::Future;
use std::pin::Pin;

use anyhow::{Result, anyhow};

#[derive(Debug, Clone)]
pub struct ValidationDiagnostic {
    pub field: &'static str,
    pub message: String,
}

pub trait Validate {
    fn validate(&self) -> Result<(), ValidationDiagnostic>;
}

#[expect(
    dead_code,
    reason = "Phase C groundwork: async validation used by future remote validators"
)]
pub trait AsyncValidate {
    fn validate_async<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<(), ValidationDiagnostic>> + 'a>>;
}

pub fn ensure_valid<T: Validate>(value: &T) -> Result<()> {
    value
        .validate()
        .map_err(|d| anyhow!("validation failed for {}: {}", d.field, d.message))
}
