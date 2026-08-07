use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use tracing::{debug, info};

use crate::config::{Manifest, PatternSettings};
use crate::local::xml::{normalize_xml, validate_xml_fragment, xml_sha256};

#[derive(Clone, Debug)]
pub struct LocalPattern {
    pub source_path: PathBuf,
    pub file_name: String,
    pub name: String,
    pub xml: String,
    pub xml_hash: String,
    pub settings: PatternSettings,
}

pub fn load_local_patterns(directory: &Path, manifest: &Manifest) -> Result<Vec<LocalPattern>> {
    ensure!(
        directory.is_dir(),
        "patterns directory does not exist or is not a directory: {}",
        directory.display()
    );

    let mut entries = fs::read_dir(directory)
        .with_context(|| format!("failed to read patterns directory {}", directory.display()))?
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| {
            format!(
                "failed to enumerate patterns directory {}",
                directory.display()
            )
        })?;

    entries.sort_by_key(|entry| entry.file_name());

    let mut result = Vec::new();
    let mut pattern_names = HashSet::new();
    let mut used_manifest_keys = HashSet::new();

    for entry in entries {
        let path = entry.path();

        if !path.is_file() || !is_xml_file(&path) {
            continue;
        }

        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .with_context(|| format!("pattern filename is not valid UTF-8: {}", path.display()))?
            .to_owned();

        let file_stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .with_context(|| {
                format!(
                    "pattern filename has no valid UTF-8 stem: {}",
                    path.display()
                )
            })?
            .to_owned();

        let resolved = manifest.resolve(&file_stem, &file_name)?;

        if let Some(key) = resolved.manifest_key {
            used_manifest_keys.insert(key);
        }

        let name = resolved
            .settings
            .name
            .clone()
            .unwrap_or_else(|| file_stem.clone());

        ensure!(
            !name.trim().is_empty(),
            "pattern name cannot be empty: {}",
            path.display()
        );

        let normalized_name = name.to_lowercase();

        ensure!(
            pattern_names.insert(normalized_name),
            "duplicate local pattern name, ignoring case: {name:?}"
        );

        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read pattern as UTF-8: {}", path.display()))?;

        let xml = normalize_xml(&content);

        ensure!(!xml.is_empty(), "pattern file is empty: {}", path.display());

        validate_xml_fragment(&xml)
            .with_context(|| format!("invalid XML fragment: {}", path.display()))?;

        let xml_hash = xml_sha256(&xml);

        debug!(
            file = %path.display(),
            pattern_name = %name,
            bytes = xml.len(),
            sha256 = %xml_hash,
            "loaded local pattern"
        );

        result.push(LocalPattern {
            source_path: path,
            file_name,
            name,
            xml,
            xml_hash,
            settings: resolved.settings,
        });
    }

    manifest.ensure_all_entries_used(&used_manifest_keys)?;

    info!(
        directory = %directory.display(),
        patterns = result.len(),
        "loaded local patterns"
    );

    Ok(result)
}

fn is_xml_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("xml"))
}
