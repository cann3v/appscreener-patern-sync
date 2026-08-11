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
