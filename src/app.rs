use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, ensure};
use serde::Serialize;
use tracing::{info, warn};

use crate::api::{ApiClient, PatternDto};
use crate::cli::{ApplyArgs, Cli, Command, SyncArgs};
use crate::config::Manifest;
use crate::local::{LocalPattern, load_local_patterns};
use crate::sync::{SyncPlan, build_sync_plan, execute_sync_plan};

struct PreparedSync {
    api: ApiClient,
    rule_id: String,
    local_patterns: Vec<LocalPattern>,
    server_patterns: Vec<PatternDto>,
    plan: SyncPlan,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot<'a> {
    version: u32,
    rule_id: &'a str,
    patterns: &'a [PatternDto],
}

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Plan(args) => run_plan(args),

        Command::Apply(args) => run_apply(args),
    }
}

fn run_plan(args: SyncArgs) -> Result<()> {
    let prepared = prepare(&args)?;

    prepared.plan.print_human();

    let counts = prepared.plan.counts();

    info!(
        create = counts.create,
        update = counts.update,
        skip = counts.skip,
        delete = counts.delete,
        "synchronization plan created"
    );

    if prepared.plan.has_writes() {
        println!();
        println!("Dry run only. Use the `apply` command to execute this plan.");
    } else {
        println!();
        println!("The rule already matches the local directory.");
    }

    Ok(())
}

fn run_apply(args: ApplyArgs) -> Result<()> {
    let prepared = prepare(&args.sync)?;

    prepared.plan.print_human();

    if !prepared.plan.has_writes() {
        println!();
        println!("The rule already matches the local directory. No changes applied.");

        return Ok(());
    }

    ensure!(
        !prepared.local_patterns.is_empty() || args.allow_empty,
        "local directory contains no XML patterns; \
         refusing to erase the rule without --allow-empty"
    );

    write_snapshot(
        &args.snapshot_out,
        &prepared.rule_id,
        &prepared.server_patterns,
    )?;

    warn!(
        snapshot = %args.snapshot_out.display(),
        "applying synchronization plan"
    );

    execute_sync_plan(
        &prepared.api,
        &prepared.rule_id,
        &prepared.local_patterns,
        &prepared.plan,
    )?;

    println!();
    println!("Rule pattern set now matches the local directory.");

    Ok(())
}

fn prepare(args: &SyncArgs) -> Result<PreparedSync> {
    let manifest_path = args.manifest_path();

    let manifest = Manifest::load(&manifest_path)?;

    let local_patterns = load_local_patterns(&args.patterns_dir, &manifest)?;

    let rule_id = args.rule_id.to_string();

    let api = ApiClient::new(
        &args.base_url,
        &args.token,
        Duration::from_secs(args.timeout),
    )?;

    info!(
        rule_id = %rule_id,
        "checking custom rule"
    );

    api.verify_custom_rule(&rule_id)?;

    let server_patterns = api.get_patterns(&rule_id)?;

    info!(
        rule_id = %rule_id,
        patterns = server_patterns.len(),
        "loaded server patterns"
    );

    let plan = build_sync_plan(&rule_id, &local_patterns, &server_patterns)?;

    Ok(PreparedSync {
        api,
        rule_id,
        local_patterns,
        server_patterns,
        plan,
    })
}

fn write_snapshot(path: &Path, rule_id: &str, patterns: &[PatternDto]) -> Result<()> {
    /*
     * create_new не позволяет случайно перезаписать
     * существующий snapshot.
     */
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| {
            format!(
                "failed to create snapshot {}; \
                 the file must not already exist",
                path.display()
            )
        })?;

    let snapshot = Snapshot {
        version: 1,
        rule_id,
        patterns,
    };

    serde_json::to_writer_pretty(&mut file, &snapshot)
        .with_context(|| format!("failed to serialize snapshot {}", path.display()))?;

    file.write_all(b"\n")
        .with_context(|| format!("failed to finalize snapshot {}", path.display()))?;

    file.sync_all()
        .with_context(|| format!("failed to flush snapshot {}", path.display()))?;

    info!(
        path = %path.display(),
        patterns = patterns.len(),
        "server snapshot written"
    );

    Ok(())
}
