mod api;
mod app;
mod cli;
mod config;
mod error;
mod local;
mod logging;
mod sync;

use std::process::ExitCode;

use clap::Parser;
use tracing::error;

use crate::cli::Cli;

fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Err(error) = logging::init(cli.verbose, cli.quiet) {
        eprintln!("failed to initialize logging: {error:#}");

        return ExitCode::FAILURE;
    }

    match app::run(cli) {
        Ok(()) => ExitCode::SUCCESS,

        Err(error) => {
            error!(
                error = %format!("{error:#}"),
                "command failed"
            );

            ExitCode::FAILURE
        }
    }
}
