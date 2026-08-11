use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    pub fn new(label: &str) -> Self {
        let temporary_root = std::env::temp_dir();
        let process_id = std::process::id();

        for _ in 0..1000 {
            let directory_id = NEXT_DIRECTORY_ID.fetch_add(1, Ordering::Relaxed);

            let path = temporary_root.join(format!(
                "appscreener-pattern-sync-{label}-{process_id}-{directory_id}"
            ));

            match fs::create_dir(&path) {
                Ok(()) => return Self { path },

                Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                    continue;
                }

                Err(error) => {
                    panic!(
                        "failed to create test directory {}: {error}",
                        path.display()
                    );
                }
            }
        }

        panic!("failed to allocate a unique test directory");
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        match fs::remove_dir_all(&self.path) {
            Ok(()) => {}

            Err(error) if error.kind() == ErrorKind::NotFound => {}

            Err(error) => {
                eprintln!(
                    "warning: failed to remove test directory {}: {error}",
                    self.path.display()
                );
            }
        }
    }
}
