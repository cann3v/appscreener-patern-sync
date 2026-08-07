use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow, ensure};
use serde::Deserialize;

use crate::api::{PatternType, QueryType};

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatternSettings {
    /// Необязательное переопределение имени.
    /// По умолчанию используется имя XML-файла без расширения.
    pub name: Option<String>,

    #[serde(rename = "type")]
    pub pattern_type: Option<PatternType>,

    #[serde(rename = "queryType", alias = "query_type")]
    pub query_type: Option<QueryType>,

    pub severity: Option<i32>,

    pub confidence: Option<i32>,

    pub active: Option<bool>,

    #[serde(rename = "fileRegex", alias = "file_regex")]
    pub file_regex: Option<String>,
}

impl PatternSettings {
    fn merged(defaults: &Self, specific: &Self) -> Self {
        Self {
            // Общее имя для defaults запрещено:
            // иначе все файлы получат одно имя.
            name: specific.name.clone(),

            pattern_type: specific.pattern_type.or(defaults.pattern_type),

            query_type: specific.query_type.or(defaults.query_type),

            severity: specific.severity.or(defaults.severity),

            confidence: specific.confidence.or(defaults.confidence),

            active: specific.active.or(defaults.active).or(Some(true)),

            file_regex: specific
                .file_regex
                .clone()
                .or_else(|| defaults.file_regex.clone()),
        }
    }

    fn validate(&self, pattern_label: &str) -> Result<()> {
        ensure!(
            self.pattern_type.is_some(),
            "{pattern_label}: `type` must be configured \
             in defaults or in the pattern entry"
        );

        if let Some(severity) = self.severity {
            ensure!(
                (0..=3).contains(&severity),
                "{pattern_label}: severity must be between 0 and 3"
            );
        }

        if let Some(name) = &self.name {
            ensure!(
                !name.trim().is_empty(),
                "{pattern_label}: name cannot be empty"
            );
        }

        if let Some(file_regex) = &self.file_regex {
            ensure!(
                !file_regex.is_empty(),
                "{pattern_label}: fileRegex cannot be empty"
            );
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub version: u32,

    #[serde(default)]
    pub defaults: PatternSettings,

    #[serde(default)]
    pub patterns: BTreeMap<String, PatternSettings>,
}

pub struct ResolvedSettings {
    pub settings: PatternSettings,

    /// Ключ manifest, который был использован для файла.
    pub manifest_key: Option<String>,
}

impl Manifest {
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read manifest {}", path.display()))?;

        let manifest: Self = serde_saphyr::from_str(&content)
            .map_err(|error| anyhow!("invalid manifest {}: {error}", path.display()))?;

        manifest.validate()?;

        Ok(manifest)
    }

    pub fn resolve(&self, file_stem: &str, file_name: &str) -> Result<ResolvedSettings> {
        let by_stem = self.patterns.get(file_stem);
        let by_file_name = self.patterns.get(file_name);

        ensure!(
            !(by_stem.is_some() && by_file_name.is_some()),
            "manifest contains two entries for {file_name}: \
         `{file_stem}` and `{file_name}`"
        );

        let (specific, manifest_key) = if let Some(settings) = by_stem {
            (settings.clone(), Some(file_stem.to_owned()))
        } else if let Some(settings) = by_file_name {
            (settings.clone(), Some(file_name.to_owned()))
        } else {
            (PatternSettings::default(), None)
        };

        let settings = PatternSettings::merged(&self.defaults, &specific);

        settings.validate(file_name)?;

        Ok(ResolvedSettings {
            settings,
            manifest_key,
        })
    }

    pub fn ensure_all_entries_used(&self, used_keys: &HashSet<String>) -> Result<()> {
        let unused: Vec<&str> = self
            .patterns
            .keys()
            .filter(|key| !used_keys.contains(*key))
            .map(String::as_str)
            .collect();

        ensure!(
            unused.is_empty(),
            "manifest entries without matching XML files: {}",
            unused.join(", ")
        );

        Ok(())
    }

    fn validate(&self) -> Result<()> {
        ensure!(
            self.version == 1,
            "unsupported manifest version {}; expected 1",
            self.version
        );

        ensure!(
            self.defaults.name.is_none(),
            "`defaults.name` is not allowed because every \
             pattern would receive the same name"
        );

        if let Some(severity) = self.defaults.severity {
            ensure!(
                (0..=3).contains(&severity),
                "defaults.severity must be between 0 and 3"
            );
        }

        for key in self.patterns.keys() {
            ensure!(
                !key.trim().is_empty(),
                "manifest pattern key cannot be empty"
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::PatternType;

    #[test]
    fn applies_defaults_and_specific_overrides() {
        let yaml = r#"
version: 1

defaults:
  type: DATAFLOW
  active: true
  confidence: 3

patterns:
  source:
    confidence: 5
    severity: 2
"#;

        let manifest: Manifest = serde_saphyr::from_str(yaml).unwrap();

        manifest.validate().unwrap();

        let resolved = manifest.resolve("source", "source.xml").unwrap();

        assert_eq!(resolved.settings.pattern_type, Some(PatternType::Dataflow));

        assert_eq!(resolved.settings.active, Some(true));

        assert_eq!(resolved.settings.confidence, Some(5));

        assert_eq!(resolved.settings.severity, Some(2));
    }

    #[test]
    fn rejects_missing_type() {
        let yaml = r#"
version: 1
defaults:
  active: true
patterns: {}
"#;

        let manifest: Manifest = serde_saphyr::from_str(yaml).unwrap();

        manifest.validate().unwrap();

        assert!(manifest.resolve("source", "source.xml").is_err());
    }

    #[test]
    fn rejects_unknown_fields() {
        let yaml = r#"
version: 1
defaults:
  type: DATAFLOW
  unexpected: true
"#;

        assert!(serde_saphyr::from_str::<Manifest>(yaml).is_err());
    }

    #[test]
    fn rejects_name_in_defaults() {
        let yaml = r#"
version: 1
defaults:
  name: same-name
  type: DATAFLOW
"#;

        let manifest: Manifest = serde_saphyr::from_str(yaml).unwrap();

        assert!(manifest.validate().is_err());
    }
}
