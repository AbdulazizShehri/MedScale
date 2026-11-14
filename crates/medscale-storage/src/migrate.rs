//! Migration journal types.

/// Durable migration journal view.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MigrationJournal {
    pub finished_version: u32,
    pub started_version: Option<u32>,
}

impl MigrationJournal {
    /// Returns true when a migration was interrupted (started without finish).
    #[must_use]
    pub fn interrupted(&self) -> bool {
        match self.started_version {
            Some(started) => started > self.finished_version,
            None => false,
        }
    }
}
