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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::validate_spec;
    use crate::scaffold::model::{RuleScaffoldSpec, ScaffoldPatternType};
    use crate::scaffold::test_support::TestDirectory;

    fn test_spec(rules_root: &Path) -> RuleScaffoldSpec {
        RuleScaffoldSpec {
            rules_root: rules_root.to_path_buf(),
            dir_name: "custom-memory-rule".to_owned(),
            title: None,
            cwe: None,
            pattern_type: ScaffoldPatternType::Reporting,
            severity: 3,
            confidence: 1,
        }
    }

    #[test]
    fn accepts_arbitrary_directory_name_without_cwe() {
        let rules_root = TestDirectory::new("valid-spec");
        let spec = test_spec(rules_root.path());

        let target = validate_spec(&spec).unwrap();

        assert_eq!(target, rules_root.path().join("custom-memory-rule"));
    }

    #[test]
    fn rejects_missing_or_non_directory_rules_root() {
        let temporary = TestDirectory::new("invalid-root");

        let missing_root = temporary.path().join("missing");
        let missing_spec = test_spec(&missing_root);

        let error = validate_spec(&missing_spec).unwrap_err();

        assert!(
            error.to_string().contains("rules root does not exist"),
            "unexpected error: {error:#}"
        );

        let file_root = temporary.path().join("rules.txt");
        fs::write(&file_root, "not a directory").unwrap();

        let file_spec = test_spec(&file_root);
        let error = validate_spec(&file_spec).unwrap_err();

        assert!(
            error.to_string().contains("rules root is not a directory"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn rejects_invalid_directory_names() {
        let rules_root = TestDirectory::new("invalid-names");

        let invalid_names = [
            "",
            " leading-space",
            "trailing-space ",
            "trailing-dot.",
            "nested/directory",
            r"nested\directory",
            "invalid:name",
            "invalid*name",
            "CON",
            "con.txt",
            "NUL",
            "COM1",
            "lpt9.log",
            ".",
            "..",
        ];

        for name in invalid_names {
            let mut spec = test_spec(rules_root.path());
            spec.dir_name = name.to_owned();

            assert!(
                validate_spec(&spec).is_err(),
                "directory name `{name}` should be rejected"
            );
        }

        let mut spec = test_spec(rules_root.path());
        spec.dir_name = "a".repeat(256);

        assert!(
            validate_spec(&spec).is_err(),
            "a directory name longer than 255 UTF-16 code units should be rejected"
        );
    }

    #[test]
    fn rejects_invalid_metadata_values() {
        let rules_root = TestDirectory::new("invalid-metadata");

        let mut spec = test_spec(rules_root.path());
        spec.title = Some("   ".to_owned());
        assert!(validate_spec(&spec).is_err());

        let mut spec = test_spec(rules_root.path());
        spec.title = Some("invalid\ntitle".to_owned());
        assert!(validate_spec(&spec).is_err());

        let mut spec = test_spec(rules_root.path());
        spec.cwe = Some(0);
        assert!(validate_spec(&spec).is_err());

        for severity in [-1, 4] {
            let mut spec = test_spec(rules_root.path());
            spec.severity = severity;

            assert!(
                validate_spec(&spec).is_err(),
                "severity {severity} should be rejected"
            );
        }
    }

    #[test]
    fn rejects_existing_target_directory() {
        let rules_root = TestDirectory::new("existing-target");
        let spec = test_spec(rules_root.path());

        fs::create_dir(spec.target_path()).unwrap();

        let error = validate_spec(&spec).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("target directory already exists"),
            "unexpected error: {error:#}"
        );
    }
}
