use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand};
use uuid::Uuid;

#[derive(Debug, Parser)]
#[command(
    name = "appscreener-pattern-sync",
    version,
    about = "Synchronize local XML patterns with an existing Solar appScreener custom rule"
)]
pub struct Cli {
    /// Increase logging verbosity: -v enables DEBUG, -vv enables TRACE.
    #[arg(
        short = 'v',
        long = "verbose",
        action = ArgAction::Count,
        global = true,
        conflicts_with = "quiet"
    )]
    pub verbose: u8,

    /// Suppress all logs except errors.
    #[arg(short = 'q', long = "quiet", global = true, conflicts_with = "verbose")]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Build and print a synchronization plan without modifying appScreener.
    Plan(SyncArgs),

    /// Synchronize the rule with the local directory.
    Apply(ApplyArgs),
}

#[derive(Debug, Clone, Args)]
pub struct SyncArgs {
    /// appScreener origin or API base URL.
    ///
    /// Both forms are accepted:
    ///   http://appscreener.example
    ///   http://appscreener.example/app/api/v1
    #[arg(long)]
    pub base_url: String,

    /// JWT access token.
    ///
    /// Prefer the APPSCREENER_TOKEN environment variable to avoid exposing
    /// the token in the process command line.
    #[arg(long, env = "APPSCREENER_TOKEN", hide_env_values = true)]
    pub token: String,

    /// UUID of an existing custom rule.
    #[arg(long)]
    pub rule_id: Uuid,

    /// Directory containing local *.xml pattern files.
    #[arg(long)]
    pub patterns_dir: PathBuf,

    /// Path to patterns.yaml.
    ///
    /// Defaults to <patterns-dir>/patterns.yaml.
    #[arg(long)]
    pub manifest: Option<PathBuf>,

    /// HTTP request timeout in seconds.
    #[arg(
        long,
        default_value_t = 60,
        value_parser = clap::value_parser!(u64).range(1..)
    )]
    pub timeout: u64,
}

impl SyncArgs {
    pub fn manifest_path(&self) -> PathBuf {
        self.manifest
            .clone()
            .unwrap_or_else(|| self.patterns_dir.join("patterns.yaml"))
    }
}

#[derive(Debug, Args)]
pub struct ApplyArgs {
    #[command(flatten)]
    pub sync: SyncArgs,

    /// File where the pre-change server PatternDto array will be saved.
    ///
    /// A snapshot is mandatory for apply mode.
    #[arg(long)]
    pub snapshot_out: PathBuf,

    /// Permit removal of every server pattern when the local directory
    /// contains no XML files.
    #[arg(long)]
    pub allow_empty: bool,
}
