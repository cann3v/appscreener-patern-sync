mod model;
mod templates;
mod validation;
mod writer;

pub use model::{RuleScaffoldSpec, ScaffoldPatternType, ScaffoldResult};

use anyhow::Result;

pub fn create_rule_scaffold(spec: &RuleScaffoldSpec) -> Result<ScaffoldResult> {
    let target = validation::validate_spec(spec)?;

    writer::write_scaffold(spec, target)
}
