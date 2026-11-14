//! `MedScale` trusted core (Specs 001–006, 024 OS IPC).
//!
//! Spec 024 qualifies localhost OS IPC (named pipe / UDS) over the same CoreFacade.
//! Product runtime network egress remains DEFAULT_DENY.

pub mod authority;
pub mod cli_session;
pub mod compute_supervisor;
pub mod doctor;
pub mod effects;
pub mod hub_sync;
pub mod institutional_transport;
pub mod ipc;
pub mod model_acquisition;
pub mod model_catalog;
pub mod process;
pub mod r_workspace_host;
pub mod release_sbom;
pub mod text;
pub mod validate;
pub mod vault_guard;
pub mod workflow;

pub use authority::{
    CoreFacade, DEFAULT_CORPUS_ID, DEFAULT_CORPUS_VERSION, SCALE_CORPUS_DOC_COUNT_10K,
    SCALE_CORPUS_ID, SCALE_CORPUS_VERSION_10K, admit_corpus_bytes, admit_corpus_dir,
    build_synthetic_lexical_scale_corpus, default_synthetic_corpus, scale_synthetic_corpus_10k,
};
pub use cli_session::CliSession;
pub use doctor::{
    build_doctor_report, build_doctor_report_full, build_doctor_report_with_allowlist,
    privacy_proof_artifact_present,
};
pub use ipc::{
    HostIpcClient, HostIpcError, HostIpcServer, MAX_FRAME_BYTES, endpoint_for_vault_root,
};
pub use medscale_contracts::{MEDSCALE_VERSION, WorkspaceIdentity};
pub use process::SessionEnforcement;
pub use vault_guard::{VaultWriterGuard, VaultWriterGuardError};
pub use workflow::{JourneyConfig, run_minimum_lovable_journey};

/// Non-authoritative bootstrap health report for operators.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapReport {
    /// Workspace identity constants.
    pub identity: WorkspaceIdentity,
    /// Explicit statement that this process holds no vault/key/network handles.
    pub local_only: bool,
    /// Spec 002 adds object/authority semantics but no clinical product surface.
    pub medical_functionality: bool,
}

impl CoreFacade {
    /// Returns a bootstrap report proving local-only operation.
    #[must_use]
    pub fn bootstrap_report() -> BootstrapReport {
        BootstrapReport {
            identity: WorkspaceIdentity::current(),
            local_only: true,
            medical_functionality: false,
        }
    }

    /// Returns the workspace version string.
    #[must_use]
    pub fn version() -> &'static str {
        MEDSCALE_VERSION
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_report_is_local_and_non_medical() {
        let report = CoreFacade::bootstrap_report();
        assert!(report.local_only);
        assert!(!report.medical_functionality);
        assert_eq!(report.identity.version, CoreFacade::version());
    }
}
