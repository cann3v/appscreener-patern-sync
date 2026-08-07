use anyhow::{Context, Result, ensure};
use tracing::{info, warn};

use crate::api::ApiClient;
use crate::local::LocalPattern;
use crate::sync::model::{PlannedOperation, SyncPlan};
use crate::sync::planner::build_sync_plan;
use crate::sync::verifier::{verify_desired_patterns_present, verify_exact_state};

pub fn execute_sync_plan(
    api: &ApiClient,
    rule_id: &str,
    local_patterns: &[LocalPattern],
    initial_plan: &SyncPlan,
) -> Result<()> {
    /*
     * Фаза 1: UPDATE.
     *
     * Сначала обновляем уже существующие паттерны.
     * При ошибке ничего ещё не удалено.
     */
    for operation in initial_plan.updates() {
        let PlannedOperation::Update { desired, .. } = operation else {
            unreachable!("updates() returned a non-update operation");
        };

        ensure!(
            desired.uuid.is_some(),
            "cannot update pattern {:?}: UUID is missing",
            desired.name
        );

        info!(
            pattern_name = %desired.name,
            pattern_uuid = ?desired.uuid,
            "updating pattern"
        );

        api.update_pattern(desired)?;
    }

    /*
     * Фаза 2: CREATE.
     *
     * Создаём отсутствующие паттерны.
     * Старые лишние паттерны всё ещё не удаляются.
     */
    for operation in initial_plan.creates() {
        let PlannedOperation::Create { desired } = operation else {
            unreachable!("creates() returned a non-create operation");
        };

        info!(
            pattern_name = %desired.name,
            "creating pattern"
        );

        let created = api.create_pattern(desired)?;

        let created_uuid = created
            .uuid
            .clone()
            .context("appScreener created a pattern but returned no UUID")?;

        info!(
            pattern_name = %created.name,
            pattern_uuid = %created_uuid,
            "pattern created"
        );

        let mut finalized = desired.clone();

        finalized.uuid = Some(created_uuid.clone());

        /*
         * Сохраняем серверные поля из ответа POST,
         * чтобы последующий PUT соответствовал запросу UI.
         */
        finalized.shared = created.shared;

        finalized.user = created.user.clone();

        finalized.confidence = finalized.confidence.or(created.confidence);

        finalized.query_type = finalized.query_type.or(created.query_type);

        finalized.file_regex = finalized.file_regex.or_else(|| created.file_regex.clone());

        info!(
            pattern_name = %finalized.name,
            pattern_uuid = %created_uuid,
            severity = finalized.severity,
            confidence = ?finalized.confidence,
            shared = ?finalized.shared,
            "saving newly created pattern"
        );

        api.update_pattern(&finalized)?;

        info!(
            pattern_name = %finalized.name,
            pattern_uuid = %created_uuid,
            "new pattern saved"
        );
    }

    /*
     * Фаза 3: проверяем наличие всего локального набора.
     *
     * Если хотя бы один CREATE/UPDATE не применился корректно,
     * функция завершится до удаления старых паттернов.
     */
    let current = verify_desired_patterns_present(api, rule_id, local_patterns)?;

    /*
     * Строим план удаления заново по свежему состоянию.
     *
     * Это безопаснее, чем слепо применять DELETE из первоначального
     * плана: серверное состояние могло измениться между запросами.
     */
    let cleanup_plan = build_sync_plan(rule_id, local_patterns, &current)?;

    let cleanup_counts = cleanup_plan.counts();

    ensure!(
        cleanup_counts.create == 0 && cleanup_counts.update == 0,
        "server state changed before cleanup: \
         create={}, update={}",
        cleanup_counts.create,
        cleanup_counts.update
    );

    /*
     * Фаза 4: DELETE.
     *
     * Выполняется только после успешной проверки нового набора.
     */
    for operation in cleanup_plan.deletes() {
        let PlannedOperation::Delete { current } = operation else {
            unreachable!("deletes() returned a non-delete operation");
        };

        let uuid = current.uuid.as_deref().unwrap_or_default();

        ensure!(
            !uuid.is_empty(),
            "cannot delete pattern {:?}: UUID is missing",
            current.name
        );

        warn!(
            pattern_name = %current.name,
            pattern_uuid = %uuid,
            "deleting pattern absent from local directory"
        );

        api.delete_pattern(uuid, &current.name)?;
    }

    /*
     * Фаза 5: точное равенство.
     */
    verify_exact_state(api, rule_id, local_patterns)?;

    info!(
        patterns = local_patterns.len(),
        "rule pattern set matches local directory"
    );

    Ok(())
}
