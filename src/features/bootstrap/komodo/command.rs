use crate::cli::BootstrapKomodoCommand;
use crate::cli::GlobalOpts;
use crate::features::bootstrap::komodo::execute;
use crate::features::bootstrap::komodo::human;
use crate::features::bootstrap::komodo::input;
use crate::features::bootstrap::komodo::plan;
use crate::output::{Style, execution_footer_line};
use crate::runner::IoMode;
use crate::runner::LocalRunner;
use crate::runtime::ExitStatus;
use anyhow::Result;

pub fn run(opts: BootstrapKomodoCommand, global: &GlobalOpts) -> Result<ExitStatus> {
    // Resolve inputs
    let resolved = input::resolve_inputs(opts.clone())?;
    let config = &resolved.config;

    // Validate inputs
    if let Some(ref host) = config.host {
        crate::features::bootstrap::komodo::validate::validate_komodo_host(host)?;
    }
    if let Some(ref addr) = config.core_address {
        crate::features::bootstrap::komodo::validate::validate_ws_address(addr)?;
    }

    // Build plan
    let operations = plan::build_plan(config)?;

    // Select I/O mode (use Buffered for dry-run or JSON output)
    let live_execution =
        matches!(config.output, crate::cli::OutputFormat::Human) && !config.dry_run;
    let io_mode = if live_execution {
        IoMode::LiveTee
    } else {
        IoMode::Buffered
    };

    // Execute plan
    let runner = LocalRunner;
    let report = execute::execute_plan(&runner, config, operations, io_mode);

    // Format output
    let style = Style::for_human(global.color);
    match config.output {
        crate::cli::OutputFormat::Human => {
            if live_execution {
                println!("{}", execution_footer_line(&report.operations));
            } else {
                print!("{}", human::format_report_human(&report, &style));
            }
        }
        crate::cli::OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }

    if report.has_failures() {
        Ok(ExitStatus::Failure)
    } else {
        Ok(ExitStatus::Success)
    }
}
