use std::path::{Path, PathBuf};

use clap::ValueEnum;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ScaffoldPatternType {
    Reporting,
    Dataflow,
}

impl ScaffoldPatternType {
    pub fn as_manifest_value(self) -> &'static str {
        match self {
            Self::Reporting => "REPORTING",
            Self::Dataflow => "DATAFLOW",
        }
    }

    pub fn analysis_label(self) -> &'static str {
        match self {
            Self::Reporting => "AST Matcher / Reporting",

            Self::Dataflow => "DataFlow",
        }
    }
}

impl std::fmt::Display for ScaffoldPatternType {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_manifest_value())
    }
}

#[derive(Clone, Debug)]
pub struct RuleScaffoldSpec {
    pub rules_root: PathBuf,
    pub dir_name: String,
    pub title: Option<String>,
    pub cwe: Option<u32>,
    pub pattern_type: ScaffoldPatternType,
    pub severity: i32,
    pub confidence: i32,
}

impl RuleScaffoldSpec {
    pub fn target_path(&self) -> PathBuf {
        self.rules_root.join(&self.dir_name)
    }

    pub fn display_title(&self) -> &str {
        self.title.as_deref().unwrap_or(&self.dir_name)
    }

    pub fn cwe_label(&self) -> String {
        match self.cwe {
            Some(cwe) => format!("CWE-{cwe}"),
            None => "TODO".to_owned(),
        }
    }

    pub fn document_heading(&self) -> String {
        match self.cwe {
            Some(cwe) => {
                format!("CWE-{cwe} — {}", self.display_title())
            }

            None => self.display_title().to_owned(),
        }
    }

    pub fn pattern_catalog_heading(&self) -> String {
        match self.cwe {
            Some(cwe) => {
                format!("Каталог паттернов CWE-{cwe}")
            }

            None => {
                format!("Каталог паттернов {}", self.display_title())
            }
        }
    }
}

#[derive(Debug)]
pub struct ScaffoldResult {
    pub target: PathBuf,
    pub files: Vec<PathBuf>,
}

impl ScaffoldResult {
    pub fn relative_files(&self) -> impl Iterator<Item = &Path> {
        self.files.iter().map(PathBuf::as_path)
    }
}
