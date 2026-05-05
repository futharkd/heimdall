use anyhow::{Result, anyhow};

pub fn validate_address(address: &str) -> Result<()> {
    if !address.starts_with("https://") && !address.starts_with("http://") {
        return Err(anyhow!(
            "Infisical address must start with https:// or http://"
        ));
    }
    Ok(())
}
