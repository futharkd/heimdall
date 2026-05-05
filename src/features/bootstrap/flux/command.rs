use std::fs;
use std::io::IsTerminal;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use crate::cli::{BootstrapFluxCommand, GlobalOpts, OutputFormat};
use crate::output::{Style, execution_footer_line};
use crate::runner::{IoMode, LocalRunner};
use crate::runtime::ExitStatus;

use super::execute::execute_plan;
use super::human::format_report_human;
use super::input::{
    BootstrapFluxConfig, kubeconfig_requires_elevated_access, probe_flux_namespace,
    probe_flux_on_path, resolve_inputs, wait_enter_after_deploy_key_prompt,
};
use super::plan::build_plan;

pub fn run(opts: BootstrapFluxCommand, global: &GlobalOpts) -> Result<ExitStatus> {
    let mut resolved = resolve_inputs(opts)?;
    let runner = LocalRunner;

    resolved.config.kube_elevated =
        kubeconfig_requires_elevated_access(&resolved.config.kubeconfig);

    resolved.config.namespace_exists = probe_flux_namespace(
        &runner,
        &resolved.config.kubeconfig,
        &resolved.config.namespace,
        resolved.config.kube_elevated,
    )?;

    if !resolved.config.namespace_exists {
        prepare_bootstrap_private_key(&mut resolved.config)?;
    } else {
        resolved.config.private_key_bootstrap_path = None;
        resolved.config.ephemeral_key_pair_root = None;
    }

    if !resolved.config.namespace_exists && !resolved.config.force && probe_flux_on_path(&runner) {
        resolved.config.skip_flux_cli_install = true;
        eprintln!(
            "note: flux found on PATH; skipping install script (use --force to re-run installer)"
        );
    }

    let plan = build_plan(&resolved.config)?;
    let live_execution = matches!(
        (resolved.output, resolved.config.dry_run),
        (OutputFormat::Human, false)
    );
    let io_mode = if live_execution {
        IoMode::LiveTee
    } else {
        IoMode::Buffered
    };
    let report = execute_plan(&runner, &resolved.config, &plan, io_mode);

    let bootstrap_ok = !report.has_failures();
    cleanup_ephemeral_keys(&resolved.config, bootstrap_ok)
        .unwrap_or_else(|e| eprintln!("note: failed to clean up ephemeral SSH key files: {e}"));

    let style = match resolved.output {
        OutputFormat::Human => Style::for_human(global.color),
        OutputFormat::Json => Style::plain(),
    };

    match resolved.output {
        OutputFormat::Human if live_execution => {
            println!("{}", execution_footer_line(&report.operations))
        }
        OutputFormat::Human => println!("{}", format_report_human(&report, &style)),
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
    }

    Ok(if report.has_failures() {
        ExitStatus::Failure
    } else {
        ExitStatus::Success
    })
}

fn prepare_bootstrap_private_key(cfg: &mut BootstrapFluxConfig) -> Result<()> {
    if cfg.byok_private_key.is_some() {
        cfg.private_key_bootstrap_path = cfg.byok_private_key.clone();
        return Ok(());
    }
    if cfg.dry_run {
        cfg.private_key_bootstrap_path =
            Some(PathBuf::from("/tmp/heimdall-flux-dryrun-private-key"));
        return Ok(());
    }
    if !std::io::stdin().is_terminal() {
        bail!(
            "stdin is not a TTY: use --private-key-file for non-interactive Flux bootstrap, or run from a terminal for generated deploy keys"
        );
    }

    let key_path = std::env::temp_dir().join(format!(
        "heimdall-flux-deploy-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));

    super::keygen::ssh_keygen_ed25519(&key_path)
        .context("generate SSH deploy key with ssh-keygen")?;

    let pub_path = key_path.with_extension("pub");
    let pubkey = fs::read_to_string(&pub_path)
        .with_context(|| format!("read generated public key {pub_path:?}"))?;

    eprintln!();
    eprintln!("Add this SSH public key as a deploy key on your Git forge with write access:");
    eprintln!(
        "  Why: `flux bootstrap git` pushes the initial Flux manifests and sync metadata into the repo; read-only keys cannot do that."
    );
    eprintln!(
        "  GitLab: Project → Settings → Repository → Deploy keys (enable “Write access” / “Grant write permissions”)"
    );
    eprintln!("  GitHub: Repository → Settings → Deploy keys (“Allow write access”)");
    eprintln!();
    eprintln!("{}", pubkey.trim_end());

    wait_enter_after_deploy_key_prompt()?;

    cfg.private_key_bootstrap_path = Some(key_path.clone());
    cfg.ephemeral_key_pair_root = Some(key_path);
    cfg.ephemeral_key_generated = true;
    Ok(())
}

fn cleanup_ephemeral_keys(cfg: &BootstrapFluxConfig, bootstrap_succeeded: bool) -> Result<()> {
    if !cfg.ephemeral_key_generated {
        return Ok(());
    }
    let Some(ref priv_path) = cfg.ephemeral_key_pair_root else {
        return Ok(());
    };
    let pub_path = priv_path.with_extension("pub");

    if bootstrap_succeeded && let Some(ref dir) = cfg.keep_generated_key_dir {
        fs::create_dir_all(dir).with_context(|| format!("create_dir_all {dir:?}"))?;
        let dest_priv = dir.join("deploy_key");
        let dest_pub = dir.join("deploy_key.pub");
        fs::copy(priv_path, &dest_priv)
            .with_context(|| format!("copy private key to {dest_priv:?}"))?;
        fs::copy(&pub_path, &dest_pub)
            .with_context(|| format!("copy public key to {dest_pub:?}"))?;
    }

    let _ = fs::remove_file(priv_path);
    let _ = fs::remove_file(&pub_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::cli::{BootstrapAction, Cli, Command, OutputFormat};

    #[test]
    fn cli_parses_bootstrap_flux_flags() {
        let parsed = Cli::try_parse_from([
            "heimdall",
            "bootstrap",
            "flux",
            "--url",
            "ssh://git@gitlab.com/g/r.git",
            "--branch",
            "main",
            "--path",
            "clusters/x",
            "--namespace",
            "flux-system",
            "--kubeconfig",
            "/tmp/kube",
            "--private-key-file",
            "/tmp/id_flux",
            "--dry-run",
            "--yes",
            "--output",
            "json",
        ])
        .expect("parses");

        let Command::Bootstrap(bootstrap) = parsed.command else {
            panic!("expected bootstrap");
        };
        let BootstrapAction::Flux(f) = bootstrap.action else {
            panic!("expected flux");
        };
        assert_eq!(f.url.as_deref(), Some("ssh://git@gitlab.com/g/r.git"));
        assert_eq!(f.branch.as_deref(), Some("main"));
        assert_eq!(f.path.as_deref(), Some("clusters/x"));
        assert_eq!(f.namespace.as_deref(), Some("flux-system"));
        assert_eq!(f.kubeconfig.as_deref(), Some("/tmp/kube"));
        assert_eq!(f.private_key_file.as_deref(), Some("/tmp/id_flux"));
        assert!(f.dry_run);
        assert!(matches!(f.output, OutputFormat::Json));
    }
}
