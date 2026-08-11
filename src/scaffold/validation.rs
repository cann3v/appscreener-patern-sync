use std::path::{Component, Path, PathBuf};

use anyhow::{Result, ensure};

use crate::scaffold::model::RuleScaffoldSpec;

pub fn validate_spec(spec: &RuleScaffoldSpec) -> Result<PathBuf> {
    validate_rules_root(&spec.rules_root)?;

    validate_directory_name(&spec.dir_name)?;

    validate_title(spec.title.as_deref())?;

    if let Some(cwe) = spec.cwe {
        ensure!(cwe > 0, "CWE number must be greater than zero");
    }

    ensure!(
        (0..=3).contains(&spec.severity),
        "severity must be between 0 and 3"
    );

    let target = spec.target_path();

    ensure!(
        !target.exists(),
        "target directory already exists: {}",
        target.display()
    );

    Ok(target)
}

fn validate_rules_root(rules_root: &Path) -> Result<()> {
    ensure!(
        rules_root.exists(),
        "rules root does not exist: {}",
        rules_root.display()
    );

    ensure!(
        rules_root.is_dir(),
        "rules root is not a directory: {}",
        rules_root.display()
    );

    Ok(())
}

fn validate_directory_name(name: &str) -> Result<()> {
    ensure!(!name.is_empty(), "directory name cannot be empty");

    ensure!(
        name == name.trim(),
        "directory name must not contain \
         leading or trailing whitespace"
    );

    ensure!(
        !name.ends_with('.'),
        "directory name must not end with a dot"
    );

    ensure!(
        !name.chars().any(char::is_control),
        "directory name must not contain \
         control characters"
    );

    ensure!(
        !name.chars().any(is_invalid_windows_character),
        "directory name contains a character \
         forbidden on Windows"
    );

    ensure!(
        name.encode_utf16().count() <= 255,
        "directory name is too long"
    );

    let mut components = Path::new(name).components();

    ensure!(
        matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none(),
        "directory name must be a single \
         relative path component"
    );

    ensure!(
        name != "." && name != "..",
        "directory name cannot be `.` or `..`"
    );

    ensure!(
        !is_reserved_windows_name(name),
        "directory name is reserved on Windows: {name}"
    );

    Ok(())
}

fn validate_title(title: Option<&str>) -> Result<()> {
    let Some(title) = title else {
        return Ok(());
    };

    ensure!(!title.trim().is_empty(), "title cannot be empty");

    ensure!(
        !title.chars().any(char::is_control),
        "title must not contain control characters"
    );

    Ok(())
}

fn is_invalid_windows_character(character: char) -> bool {
    matches!(
        character,
        '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
    )
}

fn is_reserved_windows_name(name: &str) -> bool {
    let base = name.split('.').next().unwrap_or(name).to_ascii_uppercase();

    matches!(
        base.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}
