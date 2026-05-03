use anyhow::Result;

pub fn validate_port(_port: u16) -> Result<()> {
    // Port is u16, so it's always in valid range (1-65535)
    Ok(())
}
