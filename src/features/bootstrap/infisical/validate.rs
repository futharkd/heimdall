use crate::core::validation::{Validate, ValidationDiagnostic};
use anyhow::{Result, anyhow};

pub fn validate_address(address: &str) -> Result<()> {
    if !address.starts_with("https://") && !address.starts_with("http://") {
        return Err(anyhow!(
            "Infisical address must start with https:// or http://"
        ));
    }
    Ok(())
}

pub struct AddressValidator<'a> {
    pub address: &'a str,
}

impl Validate for AddressValidator<'_> {
    fn validate(&self) -> std::result::Result<(), ValidationDiagnostic> {
        if self.address.starts_with("https://") || self.address.starts_with("http://") {
            Ok(())
        } else {
            Err(ValidationDiagnostic {
                field: "address",
                message: "Infisical address must start with https:// or http://".to_string(),
            })
        }
    }
}
