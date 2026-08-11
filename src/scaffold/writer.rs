use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};

use crate::scaffold::model::{RuleScaffoldSpec, ScaffoldResult};
use crate::scaffold::templates;

pub fn write_scaffold(spec: &RuleScaffoldSpec, target: PathBuf) -> Result<ScaffoldResult> {
    let mut staging = StagingDirectory::create(&spec.rules_root, &spec.dir_name)?;

    let root = staging.path();

    create_directories(root)?;

    let files = vec![
        PathBuf::from("README.md"),
        PathBuf::from("docs/pattern-catalog.md"),
        PathBuf::from("rule/rule-metadata.md"),
        PathBuf::from("rule/patterns/patterns.yaml"),
        PathBuf::from("tests/build.sh"),
        PathBuf::from("tests/include/.gitkeep"),
        PathBuf::from("tests/src/.gitkeep"),
    ];

    write_file(&root.join(&files[0]), &templates::readme(spec))?;

    write_file(&root.join(&files[1]), &templates::pattern_catalog(spec))?;

    write_file(&root.join(&files[2]), &templates::rule_metadata(spec))?;

    write_file(&root.join(&files[3]), &templates::patterns_manifest(spec))?;

    write_file(&root.join(&files[4]), templates::build_script())?;

    write_file(&root.join(&files[5]), "")?;

    write_file(&root.join(&files[6]), "")?;

    set_build_script_executable(&root.join(&files[4]))?;

    ensure!(
        !target.exists(),
        "target directory appeared during generation: {}",
        target.display()
    );

    staging.commit(&target)?;

    Ok(ScaffoldResult { target, files })
}

fn create_directories(root: &Path) -> Result<()> {
    for relative in [
        "docs",
        "rule",
        "rule/patterns",
        "tests",
        "tests/include",
        "tests/src",
    ] {
        let directory = root.join(relative);

        fs::create_dir_all(&directory)
            .with_context(|| format!("failed to create directory {}", directory.display()))?;
    }

    Ok(())
}

fn write_file(path: &Path, content: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("failed to create file {}", path.display()))?;

    file.write_all(content.as_bytes())
        .with_context(|| format!("failed to write file {}", path.display()))?;

    file.sync_all()
        .with_context(|| format!("failed to flush file {}", path.display()))?;

    Ok(())
}

#[cfg(unix)]
fn set_build_script_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .with_context(|| format!("failed to read permissions for {}", path.display()))?
        .permissions();

    permissions.set_mode(0o755);

    fs::set_permissions(path, permissions)
        .with_context(|| format!("failed to make {} executable", path.display()))
}

#[cfg(not(unix))]
fn set_build_script_executable(_path: &Path) -> Result<()> {
    Ok(())
}

struct StagingDirectory {
    path: PathBuf,
    committed: bool,
}

impl StagingDirectory {
    fn create(parent: &Path, target_name: &str) -> Result<Self> {
        let process_id = std::process::id();

        for attempt in 0..1000_u32 {
            let temporary_name = format!(".{target_name}.tmp-{process_id}-{attempt}");

            let path = parent.join(temporary_name);

            match fs::create_dir(&path) {
                Ok(()) => {
                    return Ok(Self {
                        path,
                        committed: false,
                    });
                }

                Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                    continue;
                }

                Err(error) => {
                    return Err(error).with_context(|| {
                        format!(
                            "failed to create temporary directory in {}",
                            parent.display()
                        )
                    });
                }
            }
        }

        bail!(
            "failed to allocate a temporary directory in {}",
            parent.display()
        )
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn commit(&mut self, target: &Path) -> Result<()> {
        fs::rename(&self.path, target).with_context(|| {
            format!("failed to move generated scaffold to {}", target.display())
        })?;

        self.committed = true;

        Ok(())
    }
}

impl Drop for StagingDirectory {
    fn drop(&mut self) {
        if self.committed {
            return;
        }

        if let Err(error) = fs::remove_dir_all(&self.path) {
            eprintln!(
                "warning: failed to remove temporary directory {}: {}",
                self.path.display(),
                error
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use crate::scaffold::model::{RuleScaffoldSpec, ScaffoldPatternType};
    use crate::scaffold::templates;
    use crate::scaffold::test_support::TestDirectory;
    use crate::scaffold::{create_rule_scaffold, validation};

    fn test_spec(rules_root: &Path) -> RuleScaffoldSpec {
        RuleScaffoldSpec {
            rules_root: rules_root.to_path_buf(),
            dir_name: "custom-rule".to_owned(),
            title: Some("Custom Rule".to_owned()),
            cwe: Some(273),
            pattern_type: ScaffoldPatternType::Dataflow,
            severity: 3,
            confidence: 1,
        }
    }

    #[test]
    fn creates_complete_rule_scaffold() {
        let rules_root = TestDirectory::new("writer-success");
        let spec = test_spec(rules_root.path());

        let result = create_rule_scaffold(&spec).unwrap();
        let target = rules_root.path().join("custom-rule");

        assert_eq!(result.target, target);
        assert!(target.is_dir());

        for directory in [
            "docs",
            "rule",
            "rule/patterns",
            "tests",
            "tests/include",
            "tests/src",
        ] {
            assert!(
                target.join(directory).is_dir(),
                "missing generated directory: {directory}"
            );
        }

        let expected_files = [
            "README.md",
            "docs/pattern-catalog.md",
            "rule/rule-metadata.md",
            "rule/patterns/patterns.yaml",
            "tests/build.sh",
            "tests/include/.gitkeep",
            "tests/src/.gitkeep",
        ];

        for relative_path in &expected_files {
            assert!(
                target.join(relative_path).is_file(),
                "missing generated file: {relative_path}"
            );
        }

        let actual_files: Vec<PathBuf> = result.relative_files().map(Path::to_path_buf).collect();

        let expected_paths: Vec<PathBuf> = expected_files.iter().map(PathBuf::from).collect();

        assert_eq!(actual_files, expected_paths);

        assert_eq!(
            fs::read_to_string(target.join("README.md")).unwrap(),
            templates::readme(&spec)
        );

        assert_eq!(
            fs::read_to_string(target.join("docs/pattern-catalog.md")).unwrap(),
            templates::pattern_catalog(&spec)
        );

        assert_eq!(
            fs::read_to_string(target.join("rule/rule-metadata.md")).unwrap(),
            templates::rule_metadata(&spec)
        );

        assert_eq!(
            fs::read_to_string(target.join("rule/patterns/patterns.yaml")).unwrap(),
            templates::patterns_manifest(&spec)
        );

        assert_eq!(
            fs::read_to_string(target.join("tests/build.sh")).unwrap(),
            templates::build_script()
        );

        assert_eq!(
            fs::metadata(target.join("tests/include/.gitkeep"))
                .unwrap()
                .len(),
            0
        );

        assert_eq!(
            fs::metadata(target.join("tests/src/.gitkeep"))
                .unwrap()
                .len(),
            0
        );

        let temporary_prefix = ".custom-rule.tmp-";

        let temporary_directories: Vec<_> = fs::read_dir(rules_root.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .filter(|name| name.to_string_lossy().starts_with(temporary_prefix))
            .collect();

        assert!(
            temporary_directories.is_empty(),
            "temporary directories were not removed: {temporary_directories:?}"
        );
    }

    #[test]
    fn does_not_overwrite_existing_target() {
        let rules_root = TestDirectory::new("writer-existing");
        let spec = test_spec(rules_root.path());
        let target = spec.target_path();

        fs::create_dir(&target).unwrap();
        fs::write(target.join("sentinel.txt"), "keep me").unwrap();

        let error = create_rule_scaffold(&spec).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("target directory already exists"),
            "unexpected error: {error:#}"
        );

        assert_eq!(
            fs::read_to_string(target.join("sentinel.txt")).unwrap(),
            "keep me"
        );
    }

    #[test]
    fn rejects_target_created_after_validation() {
        let rules_root = TestDirectory::new("writer-race");
        let spec = test_spec(rules_root.path());

        let target = validation::validate_spec(&spec).unwrap();

        fs::create_dir(&target).unwrap();
        fs::write(target.join("sentinel.txt"), "keep me").unwrap();

        let error = super::write_scaffold(&spec, target.clone()).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("target directory appeared during generation"),
            "unexpected error: {error:#}"
        );

        assert_eq!(
            fs::read_to_string(target.join("sentinel.txt")).unwrap(),
            "keep me"
        );

        let temporary_directories: Vec<_> = fs::read_dir(rules_root.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .filter(|name| name.to_string_lossy().starts_with(".custom-rule.tmp-"))
            .collect();

        assert!(
            temporary_directories.is_empty(),
            "staging directory was not cleaned up: {temporary_directories:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn makes_build_script_executable_on_unix() {
        use std::os::unix::fs::PermissionsExt;

        let rules_root = TestDirectory::new("writer-permissions");
        let spec = test_spec(rules_root.path());

        let result = create_rule_scaffold(&spec).unwrap();

        let mode = fs::metadata(result.target.join("tests/build.sh"))
            .unwrap()
            .permissions()
            .mode();

        assert_eq!(mode & 0o111, 0o111);
    }
}
