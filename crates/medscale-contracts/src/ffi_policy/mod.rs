//! Native/FFI admission checklist record.

use serde::{Deserialize, Serialize};

use crate::objects::PlacementClass;

/// Admission record required before linking native/FFI code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FfiAdmissionRecord {
    pub component_name: String,
    pub upstream_url: String,
    pub revision: String,
    pub build_flags: String,
    pub abi_notes: String,
    pub ownership_freeing: String,
    pub thread_affinity: String,
    pub no_panic_across_extern_c: bool,
    pub typed_errors: bool,
    pub allocator_assumptions: String,
    pub arches: Vec<String>,
    pub placement_class: PlacementClass,
    pub fuzz_sanitizer_evidence: String,
    pub sbom_path: String,
    pub update_strategy: String,
    pub exit_strategy: String,
}

impl FfiAdmissionRecord {
    /// Returns true when all required checklist fields are non-empty / affirmed.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        !self.component_name.is_empty()
            && !self.upstream_url.is_empty()
            && !self.revision.is_empty()
            && !self.build_flags.is_empty()
            && !self.abi_notes.is_empty()
            && !self.ownership_freeing.is_empty()
            && !self.thread_affinity.is_empty()
            && self.no_panic_across_extern_c
            && self.typed_errors
            && !self.allocator_assumptions.is_empty()
            && !self.arches.is_empty()
            && !self.fuzz_sanitizer_evidence.is_empty()
            && !self.sbom_path.is_empty()
            && !self.update_strategy.is_empty()
            && !self.exit_strategy.is_empty()
    }
}
