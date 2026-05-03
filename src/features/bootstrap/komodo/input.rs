use crate::cli::{BootstrapKomodoCommand, KomodoMode, OutputFormat};
use anyhow::Result;
use inquire::{Confirm, Select, Text};
use std::fs;
use std::io::{IsTerminal, Read};

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

fn map_inquire<T>(r: Result<T, inquire::InquireError>) -> anyhow::Result<T> {
    r.map_err(|e| match e {
        inquire::InquireError::NotTTY => anyhow::anyhow!("not a TTY; pass the flag directly"),
        inquire::InquireError::OperationCanceled | inquire::InquireError::OperationInterrupted => {
            anyhow::anyhow!("cancelled")
        }
        other => anyhow::anyhow!("{other}"),
    })
}

fn resolve_mode(opts: &BootstrapKomodoCommand) -> Result<KomodoMode> {
    let stdin_is_tty = std::io::stdin().is_terminal();

    // If TTY, prompt for mode
    if stdin_is_tty {
        let mode = map_inquire(
            Select::new(
                "Komodo deployment mode:",
                vec![KomodoMode::Core, KomodoMode::Periphery],
            )
            .with_starting_cursor(0)
            .prompt(),
        )?;
        return Ok(mode);
    }

    // Use the default (which respects the --mode flag)
    Ok(opts.mode)
}

fn resolve_host(mode: KomodoMode, opts: &BootstrapKomodoCommand) -> Result<Option<String>> {
    if mode != KomodoMode::Core {
        return Ok(None);
    }

    // If host provided via flag, use it
    if let Some(host) = &opts.host {
        return Ok(Some(host.clone()));
    }

    // If not TTY, host is required for Core mode
    if !std::io::stdin().is_terminal() {
        return Ok(None);
    }

    // Prompt for host in Core mode
    let host =
        map_inquire(Text::new("Komodo Core host URL (e.g. https://komodo.example.com):").prompt())?;
    let trimmed = host.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_string()))
    }
}

fn resolve_admin_username(opts: &BootstrapKomodoCommand) -> Result<String> {
    if let Some(username) = &opts.admin_username {
        return Ok(username.clone());
    }

    if !std::io::stdin().is_terminal() {
        return Ok("admin".to_string());
    }

    let username = map_inquire(Text::new("Admin username:").with_default("admin").prompt())?;
    let trimmed = username.trim();
    if trimmed.is_empty() {
        Ok("admin".to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

fn resolve_admin_password(opts: &BootstrapKomodoCommand) -> Result<String> {
    if let Some(password) = &opts.admin_password {
        return Ok(password.clone());
    }

    if !std::io::stdin().is_terminal() {
        return Ok(generate_secret().unwrap_or_else(|_| "changeme".to_string()));
    }

    let use_auto = map_inquire(
        Confirm::new("Auto-generate admin password?")
            .with_default(true)
            .prompt(),
    )?;

    if use_auto {
        Ok(generate_secret().unwrap_or_else(|_| "changeme".to_string()))
    } else {
        let password = map_inquire(Text::new("Admin password:").prompt())?;
        let trimmed = password.trim();
        if trimmed.is_empty() {
            Ok(generate_secret().unwrap_or_else(|_| "changeme".to_string()))
        } else {
            Ok(trimmed.to_string())
        }
    }
}

fn resolve_db_password(opts: &BootstrapKomodoCommand) -> Result<String> {
    if let Some(password) = &opts.db_password {
        return Ok(password.clone());
    }

    if !std::io::stdin().is_terminal() {
        return Ok(generate_secret().unwrap_or_else(|_| "changeme".to_string()));
    }

    let use_auto = map_inquire(
        Confirm::new("Auto-generate database password?")
            .with_default(true)
            .prompt(),
    )?;

    if use_auto {
        Ok(generate_secret().unwrap_or_else(|_| "changeme".to_string()))
    } else {
        let password = map_inquire(Text::new("Database password:").prompt())?;
        let trimmed = password.trim();
        if trimmed.is_empty() {
            Ok(generate_secret().unwrap_or_else(|_| "changeme".to_string()))
        } else {
            Ok(trimmed.to_string())
        }
    }
}

fn resolve_core_address(mode: KomodoMode, opts: &BootstrapKomodoCommand) -> Result<Option<String>> {
    if mode != KomodoMode::Periphery {
        return Ok(opts.core_address.clone());
    }

    if let Some(addr) = &opts.core_address {
        return Ok(Some(addr.clone()));
    }

    if !std::io::stdin().is_terminal() {
        return Ok(None);
    }

    let addr = map_inquire(Text::new("Core WebSocket address (e.g. ws://core:9120):").prompt())?;
    let trimmed = addr.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_string()))
    }
}

pub fn resolve_inputs(opts: BootstrapKomodoCommand) -> Result<ResolvedKomodoInputs> {
    // Resolve interactive inputs first (all borrow opts)
    let mode = resolve_mode(&opts)?;
    let admin_username = resolve_admin_username(&opts)?;
    let admin_password = resolve_admin_password(&opts)?;
    let db_password = resolve_db_password(&opts)?;
    let host = resolve_host(mode, &opts)?;
    let core_address = resolve_core_address(mode, &opts)?;

    // Now consume opts for remaining fields
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
    let db_username = opts.db_username.unwrap_or_else(|| "admin".to_string());
    let backups_path = opts
        .backups_path
        .unwrap_or_else(|| "/etc/komodo/backups".to_string());
    let first_server_name = opts
        .first_server_name
        .unwrap_or_else(|| "Local".to_string());

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
