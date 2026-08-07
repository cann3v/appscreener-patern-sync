use anyhow::Result;
use tracing::info;

use crate::cli::{Cli, Command};

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Plan(args) => {
            info!(
                rule_id = %args.rule_id,
                patterns_dir = %args.patterns_dir.display(),
                manifest = %args.manifest_path().display(),
                "plan command accepted"
            );

            println!("Plan command is configured correctly.");
        }

        Command::Apply(args) => {
            info!(
                rule_id = %args.sync.rule_id,
                patterns_dir = %args.sync.patterns_dir.display(),
                manifest = %args.sync.manifest_path().display(),
                snapshot = %args.snapshot_out.display(),
                allow_empty = args.allow_empty,
                "apply command accepted"
            );

            println!("Apply command is configured correctly.");
        }
    }

    Ok(())
}
