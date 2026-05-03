use crate::cli::{BootstrapKomodoCommand, KomodoMode, OutputFormat};
use anyhow::Result;
use std::fs;
use std::io::Read;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BootstrapKomodoConfig {
    pub mode: KomodoMode,
    pub dir: String,
    pub image_tag: String,
    pub force: bool,
    pub no_up: bool,
    pub dry_run: bool,
    pub confirmed: bool,
    pub output: OutputFormat,

    // Core mode
    pub host: Option<String>,
    pub title: String,
    pub port: u16,
    pub admin_username: String,
    pub admin_password: String,
    pub db_username: String,
    pub db_password: String,
    pub backups_path: String,
    pub first_server_name: String,

    // Periphery mode
    pub core_address: Option<String>,
    pub connect_as: String,
    pub core_public_key_content: Option<String>,
    pub periphery_root: String,
}

pub struct ResolvedKomodoInputs {
    pub config: BootstrapKomodoConfig,
}

pub fn resolve_inputs(opts: BootstrapKomodoCommand) -> Result<ResolvedKomodoInputs> {
    let mode = opts.mode;
    let dir = opts
        .dir
        .unwrap_or_else(|| "/etc/heimdall/komodo".to_string());
    let image_tag = opts.image_tag.unwrap_or_else(|| "2".to_string());
    let force = opts.force;
    let no_up = opts.no_up;
    let dry_run = opts.dry_run;
    let output = opts.output;
    let confirmed = opts.yes;

    let title = opts.title.unwrap_or_else(|| "Komodo".to_string());
    let port = opts.port.unwrap_or(9120);
    let admin_username = opts.admin_username.unwrap_or_else(|| "admin".to_string());
    let admin_password = opts
        .admin_password
        .unwrap_or_else(|| generate_secret().unwrap_or_else(|_| "changeme".to_string()));
    let db_username = opts.db_username.unwrap_or_else(|| "admin".to_string());
    let db_password = opts
        .db_password
        .unwrap_or_else(|| generate_secret().unwrap_or_else(|_| "changeme".to_string()));
    let backups_path = opts
        .backups_path
        .unwrap_or_else(|| "/etc/komodo/backups".to_string());
    let first_server_name = opts
        .first_server_name
        .unwrap_or_else(|| "Local".to_string());

    let host = opts.host;
    let core_address = opts.core_address;
    let connect_as = opts.connect_as.unwrap_or_else(|| first_server_name.clone());
    let periphery_root = opts
        .periphery_root
        .unwrap_or_else(|| "/etc/komodo".to_string());

    // Read core public key from file if provided
    let core_public_key_content = if let Some(key_file) = opts.core_public_key_file {
        Some(fs::read_to_string(&key_file)?)
    } else {
        None
    };

    let config = BootstrapKomodoConfig {
        mode,
        dir,
        image_tag,
        force,
        no_up,
        dry_run,
        confirmed,
        output,
        host,
        title,
        port,
        admin_username,
        admin_password,
        db_username,
        db_password,
        backups_path,
        first_server_name,
        core_address,
        connect_as,
        core_public_key_content,
        periphery_root,
    };

    Ok(ResolvedKomodoInputs { config })
}

fn generate_secret() -> Result<String> {
    let mut buf = [0u8; 32];
    let mut f = fs::File::open("/dev/urandom")?;
    f.read_exact(&mut buf)?;
    Ok(buf.iter().map(|b| format!("{:02x}", b)).collect())
}
