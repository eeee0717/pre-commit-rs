use std::process::ExitCode;
use std::str::FromStr;

use anstream::{eprintln, ColorChoice};
use anyhow::{Context, Result};
use clap::Parser;
use owo_colors::OwoColorize;
use tracing::debug;
use tracing_subscriber::filter::Directive;
use tracing_subscriber::EnvFilter;

use crate::cli::{Cli, Command, ExitStatus};

mod cli;
mod config;
mod fs;
mod git;
mod hook;
mod identify;
mod languages;
mod store;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Level {
    /// Suppress all tracing output by default (overridable by `RUST_LOG`).
    #[default]
    Default,
    /// Show debug messages by default (overridable by `RUST_LOG`).
    Verbose,
    /// Show messages in a hierarchical span tree. By default, debug messages are shown (overridable by `RUST_LOG`).
    ExtraVerbose,
}

fn setup_logging(level: Level) -> Result<()> {
    let directive = match level {
        Level::Default => tracing::level_filters::LevelFilter::OFF.into(),
        Level::Verbose => Directive::from_str("pre_commit=debug")?,
        Level::ExtraVerbose => Directive::from_str("pre_commit=trace")?,
    };

    let filter = EnvFilter::builder()
        .with_default_directive(directive)
        .from_env()
        .context("Invalid RUST_LOG directive")?;

    let ansi = match anstream::Stderr::choice(&std::io::stderr()) {
        ColorChoice::Always | ColorChoice::AlwaysAnsi => true,
        ColorChoice::Never => false,
        // We just asked anstream for a choice, that can't be auto
        ColorChoice::Auto => unreachable!(),
    };

    let format = tracing_subscriber::fmt::format()
        .with_target(false)
        .without_time()
        .with_ansi(ansi);
    tracing_subscriber::fmt::fmt()
        .with_env_filter(filter)
        .event_format(format)
        .with_writer(std::io::stderr)
        .init();
    Ok(())
}

async fn run(cli: Cli) -> Result<ExitStatus> {
    ColorChoice::write_global(cli.global_args.color.into());

    setup_logging(match cli.global_args.verbose {
        0 => Level::Default,
        1 => Level::Verbose,
        _ => Level::ExtraVerbose,
    })?;

    // TODO: read git commit info
    debug!("pre-commit: {}", env!("CARGO_PKG_VERSION"));

    match cli.command {
        Command::Install(options) => cli::install(
            cli.global_args.config,
            options.hook_type,
            options.install_hooks,
        ),
        Command::Run(options) => {
            cli::run(cli.global_args.config, options.hook_id, options.hook_stage).await
        }
        _ => {
            eprintln!("Command not implemented yet");
            Ok(ExitStatus::Failure)
        }
    }
}

fn main() -> ExitCode {
    ctrlc::set_handler(move || {
        #[allow(clippy::exit, clippy::cast_possible_wrap)]
        std::process::exit(if cfg!(windows) {
            0xC000_013A_u32 as i32
        } else {
            130
        });
    })
    .expect("Error setting Ctrl-C handler");

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => err.exit(),
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");
    let result = runtime.block_on(Box::pin(run(cli)));
    runtime.shutdown_background();

    match result {
        Ok(code) => code.into(),
        Err(err) => {
            let mut causes = err.chain();
            eprintln!("{}: {}", "error".red().bold(), causes.next().unwrap());
            for err in causes {
                eprintln!("  {}: {}", "caused by".red().bold(), err);
            }
            ExitStatus::Error.into()
        }
    }
}
