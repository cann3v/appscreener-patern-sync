use anyhow::{Result, anyhow};
use tracing_subscriber::EnvFilter;

pub fn init(verbose: u8, quiet: bool) -> Result<()> {
    let default_level = if quiet {
        "error"
    } else {
        match verbose {
            0 => "info",
            1 => "debug",
            _ => "trace",
        }
    };

    let default_filter = format!(
        "appscreener_pattern_sync={default_level},\
         reqwest=warn,\
         hyper=warn,\
         rustls=warn"
    );

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(verbose > 0)
        .with_writer(std::io::stderr)
        .try_init()
        .map_err(|error| anyhow!("failed to install tracing subscriber: {error}"))
}
