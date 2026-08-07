use crate::api::{PatternDto, PatternWrite};

#[derive(Clone, Debug)]
pub enum PlannedOperation {
    Create {
        desired: PatternWrite,
    },

    Update {
        before: PatternDto,
        desired: PatternWrite,
        changes: Vec<String>,
    },

    Skip {
        name: String,
    },

    Delete {
        current: PatternDto,
    },
}

impl PlannedOperation {
    pub fn action_name(&self) -> &'static str {
        match self {
            Self::Create { .. } => "CREATE",
            Self::Update { .. } => "UPDATE",
            Self::Skip { .. } => "SKIP",
            Self::Delete { .. } => "DELETE",
        }
    }

    pub fn pattern_name(&self) -> &str {
        match self {
            Self::Create { desired } => &desired.name,

            Self::Update { desired, .. } => &desired.name,

            Self::Skip { name } => name,

            Self::Delete { current } => &current.name,
        }
    }

    pub fn details(&self) -> String {
        match self {
            Self::Create { .. } => "not present on server".to_owned(),

            Self::Update { changes, .. } => changes.join(", "),

            Self::Skip { .. } => "already synchronized".to_owned(),

            Self::Delete { .. } => "not present in local directory".to_owned(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PlanCounts {
    pub create: usize,
    pub update: usize,
    pub skip: usize,
    pub delete: usize,
}

impl PlanCounts {
    pub fn writes(self) -> usize {
        self.create + self.update + self.delete
    }
}

#[derive(Clone, Debug)]
pub struct SyncPlan {
    operations: Vec<PlannedOperation>,
}

impl SyncPlan {
    pub fn new(operations: Vec<PlannedOperation>) -> Self {
        Self { operations }
    }

    pub fn operations(&self) -> &[PlannedOperation] {
        &self.operations
    }

    pub fn counts(&self) -> PlanCounts {
        let mut counts = PlanCounts::default();

        for operation in &self.operations {
            match operation {
                PlannedOperation::Create { .. } => {
                    counts.create += 1;
                }

                PlannedOperation::Update { .. } => {
                    counts.update += 1;
                }

                PlannedOperation::Skip { .. } => {
                    counts.skip += 1;
                }

                PlannedOperation::Delete { .. } => {
                    counts.delete += 1;
                }
            }
        }

        counts
    }

    pub fn has_writes(&self) -> bool {
        self.counts().writes() > 0
    }

    pub fn print_human(&self) {
        println!("{:<8} {:<45} DETAILS", "ACTION", "PATTERN");

        println!("{:-<8} {:-<45} {:-<30}", "", "", "");

        for operation in &self.operations {
            println!(
                "{:<8} {:<45} {}",
                operation.action_name(),
                operation.pattern_name(),
                operation.details()
            );
        }

        let counts = self.counts();

        println!();
        println!(
            "Summary: create={}, update={}, skip={}, delete={}",
            counts.create, counts.update, counts.skip, counts.delete
        );
    }

    pub fn updates(&self) -> impl Iterator<Item = &PlannedOperation> {
        self.operations
            .iter()
            .filter(|operation| matches!(operation, PlannedOperation::Update { .. }))
    }

    pub fn creates(&self) -> impl Iterator<Item = &PlannedOperation> {
        self.operations
            .iter()
            .filter(|operation| matches!(operation, PlannedOperation::Create { .. }))
    }

    pub fn deletes(&self) -> impl Iterator<Item = &PlannedOperation> {
        self.operations
            .iter()
            .filter(|operation| matches!(operation, PlannedOperation::Delete { .. }))
    }
}
