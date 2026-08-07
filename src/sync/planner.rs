use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, ensure};

use crate::api::{PatternDto, PatternWrite};
use crate::local::{LocalPattern, xml_sha256};
use crate::sync::model::{PlannedOperation, SyncPlan};

pub fn build_sync_plan(
    rule_id: &str,
    local_patterns: &[LocalPattern],
    server_patterns: &[PatternDto],
) -> Result<SyncPlan> {
    validate_server_patterns(server_patterns)?;

    let server_by_name = index_server_patterns(server_patterns)?;

    let local_names: HashSet<String> = local_patterns
        .iter()
        .map(|pattern| normalized_name(&pattern.name))
        .collect();

    let mut operations = Vec::new();

    /*
     * LocalPattern приходит из loader в порядке имён файлов,
     * поэтому CREATE/UPDATE/SKIP также будут детерминированы.
     */
    for local in local_patterns {
        let key = normalized_name(&local.name);

        let current = server_by_name.get(&key).copied();

        let desired = build_desired_pattern(rule_id, local, current)?;

        match current {
            None => {
                operations.push(PlannedOperation::Create { desired });
            }

            Some(current) => {
                let changes = detect_changes(current, &desired);

                if changes.is_empty() {
                    operations.push(PlannedOperation::Skip {
                        name: local.name.clone(),
                    });
                } else {
                    operations.push(PlannedOperation::Update { desired, changes });
                }
            }
        }
    }

    let mut extra_server_patterns: Vec<PatternDto> = server_patterns
        .iter()
        .filter(|pattern| !local_names.contains(&normalized_name(&pattern.name)))
        .cloned()
        .collect();

    extra_server_patterns.sort_by_key(|pattern| normalized_name(&pattern.name));

    for current in extra_server_patterns {
        operations.push(PlannedOperation::Delete { current });
    }

    Ok(SyncPlan::new(operations))
}

fn validate_server_patterns(server_patterns: &[PatternDto]) -> Result<()> {
    for pattern in server_patterns {
        let uuid = pattern.uuid.as_deref().unwrap_or_default();

        ensure!(
            !uuid.trim().is_empty(),
            "server pattern {:?} has no UUID",
            pattern.name
        );

        ensure!(
            !pattern.name.trim().is_empty(),
            "server returned a pattern with an empty name"
        );
    }

    Ok(())
}

fn index_server_patterns(server_patterns: &[PatternDto]) -> Result<HashMap<String, &PatternDto>> {
    let mut result = HashMap::new();

    for pattern in server_patterns {
        let key = normalized_name(&pattern.name);

        if let Some(previous) = result.insert(key, pattern) {
            ensure!(
                false,
                "server contains duplicate pattern names \
                 ignoring case: {:?} and {:?}",
                previous.name,
                pattern.name
            );
        }
    }

    Ok(result)
}

fn build_desired_pattern(
    rule_id: &str,
    local: &LocalPattern,
    current: Option<&PatternDto>,
) -> Result<PatternWrite> {
    let pattern_type = local
        .settings
        .pattern_type
        .context("internal error: pattern type was not resolved")?;

    /*
     * Неуказанные необязательные параметры:
     *
     * - при CREATE остаются None и не отправляются;
     * - при UPDATE сохраняются с существующего паттерна.
     *
     * type, active, name и XML всегда управляются
     * локальным состоянием.
     */
    Ok(PatternWrite {
        uuid: current.and_then(|pattern| pattern.uuid.clone()),

        rule_id: rule_id.to_owned(),

        severity: local
            .settings
            .severity
            .or_else(|| current.and_then(|pattern| pattern.severity)),

        confidence: local
            .settings
            .confidence
            .or_else(|| current.and_then(|pattern| pattern.confidence)),

        name: local.name.clone(),

        xml: local.xml.clone(),

        pattern_type,

        active: local.settings.active.unwrap_or(true),

        query_type: local
            .settings
            .query_type
            .or_else(|| current.and_then(|pattern| pattern.query_type)),

        file_regex: local
            .settings
            .file_regex
            .clone()
            .or_else(|| current.and_then(|pattern| pattern.file_regex.clone())),
    })
}

fn detect_changes(current: &PatternDto, desired: &PatternWrite) -> Vec<String> {
    let mut changes = Vec::new();

    if current.name != desired.name {
        changes.push("name changed".to_owned());
    }

    if xml_sha256(&current.xml) != xml_sha256(&desired.xml) {
        changes.push("XML changed".to_owned());
    }

    if current.pattern_type != Some(desired.pattern_type) {
        changes.push(format!(
            "type: {:?} -> {:?}",
            current.pattern_type, desired.pattern_type
        ));
    }

    if current.active.unwrap_or(false) != desired.active {
        changes.push(format!(
            "active: {:?} -> {}",
            current.active, desired.active
        ));
    }

    if current.severity != desired.severity {
        changes.push(format!(
            "severity: {:?} -> {:?}",
            current.severity, desired.severity
        ));
    }

    if current.confidence != desired.confidence {
        changes.push(format!(
            "confidence: {:?} -> {:?}",
            current.confidence, desired.confidence
        ));
    }

    if current.query_type != desired.query_type {
        changes.push(format!(
            "queryType: {:?} -> {:?}",
            current.query_type, desired.query_type
        ));
    }

    if current.file_regex != desired.file_regex {
        changes.push("fileRegex changed".to_owned());
    }

    changes
}

fn normalized_name(value: &str) -> String {
    value.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::api::PatternType;
    use crate::config::PatternSettings;

    fn local_pattern(name: &str, xml: &str) -> LocalPattern {
        LocalPattern {
            name: name.to_owned(),
            xml: xml.to_owned(),

            settings: PatternSettings {
                pattern_type: Some(PatternType::Dataflow),

                active: Some(true),

                ..PatternSettings::default()
            },
        }
    }

    fn server_pattern(uuid: &str, name: &str, xml: &str) -> PatternDto {
        PatternDto {
            uuid: Some(uuid.to_owned()),
            rule_id: Some("rule-id".to_owned()),
            severity: None,
            confidence: None,
            name: name.to_owned(),
            xml: xml.to_owned(),

            pattern_type: Some(PatternType::Dataflow),

            active: Some(true),
            shared: None,
            user: None,
            query_type: None,
            file_regex: None,
        }
    }

    #[test]
    fn creates_updates_skips_and_deletes() {
        let local = vec![
            local_pattern("unchanged", "<condition/>"),
            local_pattern("changed", "<condition id=\"new\"/>"),
            local_pattern("new-pattern", "<condition id=\"new-pattern\"/>"),
        ];

        let server = vec![
            server_pattern("uuid-1", "unchanged", "<condition/>"),
            server_pattern("uuid-2", "changed", "<condition id=\"old\"/>"),
            server_pattern("uuid-3", "obsolete", "<condition/>"),
        ];

        let plan = build_sync_plan("rule-id", &local, &server).unwrap();

        let counts = plan.counts();

        assert_eq!(counts.create, 1);
        assert_eq!(counts.update, 1);
        assert_eq!(counts.skip, 1);
        assert_eq!(counts.delete, 1);
    }

    #[test]
    fn rejects_duplicate_server_names() {
        let server = vec![
            server_pattern("uuid-1", "Pattern", "<condition/>"),
            server_pattern("uuid-2", "pattern", "<condition/>"),
        ];

        let error = build_sync_plan("rule-id", &[], &server).unwrap_err();

        assert!(error.to_string().contains("duplicate pattern names"));
    }

    #[test]
    fn updates_pattern_when_type_is_missing() {
        let local = vec![local_pattern("source", "<condition/>")];

        let mut current = server_pattern("uuid-1", "source", "<condition/>");

        current.pattern_type = None;

        let plan = build_sync_plan("rule-id", &local, &[current]).unwrap();

        assert_eq!(plan.counts().update, 1);
    }

    #[test]
    fn matches_names_ignoring_case() {
        let local = vec![local_pattern("Pattern", "<condition/>")];

        let server = vec![server_pattern("uuid-1", "pattern", "<condition/>")];

        let plan = build_sync_plan("rule-id", &local, &server).unwrap();

        assert_eq!(plan.counts().update, 1);
    }
}
