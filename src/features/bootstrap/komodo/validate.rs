use anyhow::{Result, anyhow};

pub fn validate_komodo_host(host: &str) -> Result<()> {
    if !host.starts_with("https://") && !host.starts_with("http://") {
        return Err(anyhow!(
            "Komodo host must start with https:// or http:// (got: {})",
            host
        ));
    }
    if host.len() < 10 {
        return Err(anyhow!("Invalid host URL"));
    }
    Ok(())
}

pub fn validate_ws_address(addr: &str) -> Result<()> {
    if !addr.starts_with("ws://") && !addr.starts_with("wss://") {
        return Err(anyhow!(
            "Core address must start with ws:// or wss:// (got: {})",
            addr
        ));
    }
    if !addr.contains(':') || addr.split(':').nth(2).is_none() {
        return Err(anyhow!("Invalid WebSocket address format"));
    }
    Ok(())
}
