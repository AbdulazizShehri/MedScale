//! Workload runtime adapters (Fixture only in Spec 008).

use medscale_contracts::objects::OpaqueId;
use medscale_contracts::packs::PackManifestV0;
use serde_json::{Value, json};

/// Output of a pack runtime invocation — never a ClinicalAssertion.
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeOutput {
    pub proposal_payload: Value,
    pub evidence_only: bool,
    pub pack_id: OpaqueId,
}

/// Workload-specific runtime (no single preferred engine).
pub trait PackRuntimeAdapter {
    fn run_fixture(&self, pack: &PackManifestV0, input: &str) -> RuntimeOutput;
}

/// Spec 008 default: no native model engine.
#[derive(Debug, Default, Clone, Copy)]
pub struct FixtureRuntime;

impl PackRuntimeAdapter for FixtureRuntime {
    fn run_fixture(&self, pack: &PackManifestV0, input: &str) -> RuntimeOutput {
        RuntimeOutput {
            proposal_payload: json!({
                "kind": "fixture_runtime_v0",
                "input_len": input.len(),
                "pack_version": pack.version,
                "note": "Proposal-only; never ClinicalAssertion"
            }),
            evidence_only: true,
            pack_id: pack.pack_id.clone(),
        }
    }
}
