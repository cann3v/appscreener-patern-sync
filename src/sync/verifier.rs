use anyhow::{Result, ensure};
use tracing::debug;

use crate::api::{ApiClient, PatternDto};
use crate::local::LocalPattern;
use crate::sync::build_sync_plan;

/// Проверяет, что все локальные паттерны уже присутствуют
/// на сервере в желаемом состоянии.
///
/// Лишние серверные паттерны на этом этапе разрешены:
/// они удаляются только после этой проверки.
pub fn verify_desired_patterns_present(
    api: &ApiClient,
    rule_id: &str,
    local_patterns: &[LocalPattern],
) -> Result<Vec<PatternDto>> {
    let current = api.get_patterns(rule_id)?;

    let plan = build_sync_plan(rule_id, local_patterns, &current)?;

    let counts = plan.counts();

    ensure!(
        counts.create == 0 && counts.update == 0,
        "post-write verification failed: \
         create={}, update={}",
        counts.create,
        counts.update
    );

    debug!(
        local_patterns = local_patterns.len(),
        server_patterns = current.len(),
        extra_patterns = counts.delete,
        "desired patterns are present"
    );

    Ok(current)
}

/// Финальная проверка полного зеркала.
///
/// После выполнения удаления план не должен содержать
/// CREATE, UPDATE или DELETE.
pub fn verify_exact_state(
    api: &ApiClient,
    rule_id: &str,
    local_patterns: &[LocalPattern],
) -> Result<()> {
    let current = api.get_patterns(rule_id)?;

    let plan = build_sync_plan(rule_id, local_patterns, &current)?;

    let counts = plan.counts();

    ensure!(
        !plan.has_writes(),
        "final verification failed: \
         create={}, update={}, delete={}",
        counts.create,
        counts.update,
        counts.delete
    );

    ensure!(
        current.len() == local_patterns.len(),
        "final pattern count mismatch: \
         local={}, server={}",
        local_patterns.len(),
        current.len()
    );

    Ok(())
}
