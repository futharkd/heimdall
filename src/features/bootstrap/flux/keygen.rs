use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

/// Create an ed25519 keypair at `private_key_path` (OpenSSH writes `private_key_path.pub`).
pub fn ssh_keygen_ed25519(private_key_path: &Path) -> Result<()> {
    let parent = private_key_path.parent().ok_or_else(|| {
        anyhow::anyhow!("private key path has no parent directory: {private_key_path:?}")
    })?;
    std::fs::create_dir_all(parent).with_context(|| format!("create_dir_all {parent:?}"))?;

    let status = Command::new("ssh-keygen")
        .args([
            "-t",
            "ed25519",
            "-N",
            "",
            "-f",
            private_key_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("private key path is not valid UTF-8"))?,
        ])
        .status()
        .with_context(|| "failed to spawn ssh-keygen")?;

    if !status.success() {
        bail!("ssh-keygen exited with {status}");
    }
    Ok(())
}
