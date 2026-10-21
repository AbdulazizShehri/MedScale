//! Model Center and comparative-evidence presentation (Specs 068–071).
//!
//! Spec 070 distinguishes session-admitted Pack inventory from qualification
//! references. Desktop reads Pack state through the Core-owned `CliSession` API;
//! it never imports the runtime crate or treats Hugging Face as an authority plane.
//! Spec 071 binds comparative presentation to the pinned OpenMed claim ledger and
//! fails closed when matched BenchmarkManifest evidence is absent.

use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::packs::{PackArtifactKind, PackManifestV0, PackPromotionState};
use medscale_core::CliSession;
use serde_json::Value;
use std::collections::HashSet;

const OPENMED_CLAIM_LEDGER_JSON: &str =
    include_str!("../../../evidence/071-openmed-evidence-center/CLAIM_LEDGER.json");
const OPENMED_BASELINE_COMMIT: &str = "59d9cb0a2e0ccbba8fa3d891a66d83ffaf45e837";
const OPENMED_BASELINE_TREE: &str = "1c949e35b2b8f2ea69da4284b370074fc4bf84ab";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelStatusVm {
    pub scope: String,
    pub task: String,
    pub model: String,
    pub source: String,
    pub runtime: String,
    pub device: String,
    pub trust: String,
    pub benchmark: String,
    pub digest: String,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompetitiveEvidenceVm {
    pub capability: String,
    pub medscale: String,
    pub openmed: String,
    pub verdict: String,
    pub evidence: String,
    pub limitations: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ClaimLedgerSummary {
    valid: bool,
    total: usize,
    unmeasured: usize,
    waived: usize,
    structural_only: usize,
    pattern_only: usize,
    benchmark_manifests: usize,
}

impl ClaimLedgerSummary {
    const fn invalid() -> Self {
        Self {
            valid: false,
            total: 0,
            unmeasured: 0,
            waived: 0,
            structural_only: 0,
            pattern_only: 0,
            benchmark_manifests: 0,
        }
    }
}

fn summarize_claim_ledger(raw: &str) -> ClaimLedgerSummary {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return ClaimLedgerSummary::invalid();
    };
    let Some(rows) = value.get("rows").and_then(Value::as_array) else {
        return ClaimLedgerSummary::invalid();
    };
    let baseline = value.get("openmed_baseline");
    let baseline_ok = baseline
        .and_then(|entry| entry.get("baseline_commit"))
        .and_then(Value::as_str)
        == Some(OPENMED_BASELINE_COMMIT)
        && baseline
            .and_then(|entry| entry.get("baseline_tree"))
            .and_then(Value::as_str)
            == Some(OPENMED_BASELINE_TREE);
    let benchmark_manifests = value
        .get("benchmark_manifest_count")
        .and_then(Value::as_u64)
        .and_then(|count| usize::try_from(count).ok())
        .unwrap_or(usize::MAX);
    if !baseline_ok || rows.len() != 39 || benchmark_manifests == usize::MAX {
        return ClaimLedgerSummary::invalid();
    }

    let mut summary = ClaimLedgerSummary {
        valid: true,
        total: rows.len(),
        unmeasured: 0,
        waived: 0,
        structural_only: 0,
        pattern_only: 0,
        benchmark_manifests,
    };
    let mut capability_ids = HashSet::with_capacity(rows.len());
    for row in rows {
        let Some(capability_id) = row.get("capability_id").and_then(Value::as_str) else {
            return ClaimLedgerSummary::invalid();
        };
        if capability_id.is_empty()
            || !capability_ids.insert(capability_id)
            || row.get("baseline_commit").and_then(Value::as_str) != Some(OPENMED_BASELINE_COMMIT)
            || row.get("parity_claim").and_then(Value::as_bool) != Some(false)
            || row.get("surpass_claim").and_then(Value::as_bool) != Some(false)
            || row
                .get("benchmark_manifest_present")
                .and_then(Value::as_bool)
                != Some(false)
        {
            return ClaimLedgerSummary::invalid();
        }
        match row.get("claim_state").and_then(Value::as_str) {
            Some("UNMEASURED") => summary.unmeasured += 1,
            Some("WAIVED") => summary.waived += 1,
            Some("STRUCTURAL_DIFFERENTIATION_ONLY") => summary.structural_only += 1,
            Some("PATTERN_ABSORPTION_ONLY") => summary.pattern_only += 1,
            _ => return ClaimLedgerSummary::invalid(),
        }
    }
    if summary.unmeasured + summary.waived + summary.structural_only + summary.pattern_only
        != summary.total
    {
        return ClaimLedgerSummary::invalid();
    }
    summary
}

pub struct ProductIntelligenceVm {
    pub model_runtime_summary: String,
    pub model_runtime_boundary: String,
    pub model_source_summary: String,
    pub model_inventory_summary: String,
    pub openmed_baseline: String,
    pub competitive_summary: String,
    pub models: Vec<ModelStatusVm>,
    pub evidence: Vec<CompetitiveEvidenceVm>,
}

impl ProductIntelligenceVm {
    pub fn from_session(session: &mut CliSession) -> Result<Self, AuthorityError> {
        let packs = session.packs_list()?;
        Ok(Self::from_packs(&packs))
    }

    #[must_use]
    pub fn current_truth() -> Self {
        Self::from_packs(&[])
    }

    #[must_use]
    pub fn from_packs(packs: &[PackManifestV0]) -> Self {
        let mut models = Vec::with_capacity(packs.len() + 4);
        if packs.is_empty() {
            models.push(ModelStatusVm {
                scope: "SESSION INVENTORY".to_owned(),
                task: "Local Pack inventory".to_owned(),
                model: "No Pack admitted in this Desktop session".to_owned(),
                source: "Use local Pack admission to inspect a signed Pack".to_owned(),
                runtime: "No session runtime selected".to_owned(),
                device: "Local only".to_owned(),
                trust: "No Pack authority present".to_owned(),
                benchmark: "No execution or quality claim".to_owned(),
                digest: "—".to_owned(),
                state: "EMPTY".to_owned(),
            });
        } else {
            models.extend(packs.iter().map(pack_row));
        }

        models.push(ModelStatusVm {
            scope: "QUALIFICATION REFERENCE".to_owned(),
            task: "NER runtime qualification".to_owned(),
            model: "onnx-community/bert-base-NER-ONNX".to_owned(),
            source: "Hugging Face · revision 9faa2f4a2d59…".to_owned(),
            runtime: "tract-onnx 0.22.4 · portable CPU".to_owned(),
            device: "Local CPU".to_owned(),
            trust: "Signed Pack provenance · external weights excluded from Git".to_owned(),
            benchmark: "Release baseline PASS · warm p95 596.100 ms · accelerated runtime pending"
                .to_owned(),
            digest: "805168abca14c2131c26ed20c5b46a46e8ccc759f1f78264b139f0aa86c8f93d".to_owned(),
            state: "QUALIFIED BASELINE".to_owned(),
        });
        models.push(ModelStatusVm {
            scope: "CAPABILITY GAP".to_owned(),
            task: "PII / de-identification".to_owned(),
            model: "production clinical model not promoted".to_owned(),
            source: "Curated signed Pack path only".to_owned(),
            runtime: "Portable ONNX runtime available".to_owned(),
            device: "Local worker boundary".to_owned(),
            trust: "Real PHI remains denied".to_owned(),
            benchmark: "High-risk PHI recall qualification pending".to_owned(),
            digest: "—".to_owned(),
            state: "MODEL GAP".to_owned(),
        });
        models.push(ModelStatusVm {
            scope: "CAPABILITY GAP".to_owned(),
            task: "Accelerated inference".to_owned(),
            model: "No accelerated runtime promoted".to_owned(),
            source: "Spec 071: accelerated comparison remains unmeasured".to_owned(),
            runtime: "Portable tract baseline only".to_owned(),
            device: "CPU baseline".to_owned(),
            trust: "No runtime-winner claim".to_owned(),
            benchmark: "No matched accelerated-runtime BenchmarkManifest".to_owned(),
            digest: "—".to_owned(),
            state: "NOT ADMITTED".to_owned(),
        });

        let ledger = summarize_claim_ledger(OPENMED_CLAIM_LEDGER_JSON);
        let evidence = vec![
            CompetitiveEvidenceVm {
                capability: "Source custody".to_owned(),
                medscale: "Immutable source bytes + explicit transformation/loss chain".to_owned(),
                openmed: "Pinned v2.2.0 documents provenance and signed audit/review evidence surfaces".to_owned(),
                verdict: "STRUCTURAL ONLY".to_owned(),
                evidence: "MedScale Specs 002/003 · OpenMed docs/feature-map.md @ 59d9cb0a · claim ledger: source-custody".to_owned(),
                limitations: "Different semantic targets; no matched BenchmarkManifest proves comparative superiority.".to_owned(),
            },
            CompetitiveEvidenceVm {
                capability: "Identity safety".to_owned(),
                medscale: "Explicit IdentityAssertion; no silent patient merge".to_owned(),
                openmed: "Pinned baseline has provenance/clinical processing surfaces; identity authority is not a matched benchmark axis".to_owned(),
                verdict: "STRUCTURAL ONLY".to_owned(),
                evidence: "MedScale Specs 002/019 · OpenMed pinned feature map · claim ledger: identity".to_owned(),
                limitations: "Structural contract difference only; no parity or surpass claim.".to_owned(),
            },
            CompetitiveEvidenceVm {
                capability: "Longitudinal truth / conflict".to_owned(),
                medscale: "Versioned assertions, absence/freshness/conflict without silent resolution".to_owned(),
                openmed: "Pinned baseline documents timeline/provenance capabilities".to_owned(),
                verdict: "STRUCTURAL ONLY".to_owned(),
                evidence: "MedScale Specs 004/019 · OpenMed pinned feature map · claim ledger: longitudinal-truth-conflict".to_owned(),
                limitations: "No matched longitudinal benchmark; superiority is not claimed.".to_owned(),
            },
            CompetitiveEvidenceVm {
                capability: "Controlled actions".to_owned(),
                medscale: "Durable intent + payload identity + UNKNOWN reconciliation; blind retry forbidden".to_owned(),
                openmed: "Pinned baseline documents service, agent and operational surfaces".to_owned(),
                verdict: "STRUCTURAL ONLY".to_owned(),
                evidence: "MedScale Specs 014/034/063 · OpenMed pinned feature map · claim ledger: controlled-external-actions".to_owned(),
                limitations: "Authority semantics differ; no matched BenchmarkManifest proves an advantage.".to_owned(),
            },
            CompetitiveEvidenceVm {
                capability: "Clinical NER".to_owned(),
                medscale: "Real portable ONNX execution is proven; clinical-domain quality comparison is not measured".to_owned(),
                openmed: "Pinned v2.2.0 exposes clinical NER families and an evaluation harness".to_owned(),
                verdict: "UNMEASURED".to_owned(),
                evidence: "MedScale Spec 069 · OpenMed docs/feature-map.md + docs/eval-harness.md · claim ledger: clinical-ner".to_owned(),
                limitations: "No same-model/same-corpus F1, precision, recall, latency and RSS BenchmarkManifest.".to_owned(),
            },
            CompetitiveEvidenceVm {
                capability: "PII / de-identification".to_owned(),
                medscale: "Runtime path exists; no production de-identification model or PHI benchmark is promoted".to_owned(),
                openmed: "Pinned baseline documents multilingual PII, de-identification methods and validation/eval contracts".to_owned(),
                verdict: "UNMEASURED".to_owned(),
                evidence: "OpenMed docs/anonymization.md + docs/clinical-validation-protocol.md @ 59d9cb0a · claim ledger: pii-detection/de-identification".to_owned(),
                limitations: "Feature presence is verified; matched recall/leakage/false-negative comparison is absent.".to_owned(),
            },
            CompetitiveEvidenceVm {
                capability: "Model registry breadth".to_owned(),
                medscale: "Signed provenance-first Pack lifecycle; raw catalogue cardinality is deliberately not a success metric".to_owned(),
                openmed: "Pinned models.jsonl contains 2266 manifest rows and a manifest-backed registry".to_owned(),
                verdict: "ANTI-METRIC".to_owned(),
                evidence: "OpenMed docs/model-registry.md + models.jsonl @ 59d9cb0a · claim ledger: model-catalogue-breadth".to_owned(),
                limitations: "2266 is dated inventory context only; model count does not establish quality, trust or parity.".to_owned(),
            },
            CompetitiveEvidenceVm {
                capability: "Apple Silicon inference".to_owned(),
                medscale: "tract CPU baseline measured; no accelerated Apple Silicon runtime is admitted".to_owned(),
                openmed: "Pinned v2.2.0 documents Python MLX and Swift MLX paths for supported model families".to_owned(),
                verdict: "UNMEASURED".to_owned(),
                evidence: "MedScale evidence/071/RUNTIME_COMPARISON.md · OpenMed docs/mlx-backend.md @ 59d9cb0a · claim ledger: apple-silicon-mlx".to_owned(),
                limitations: "No same-model/same-corpus/same-hardware BenchmarkManifest; runtime winner is explicitly refused.".to_owned(),
            },
        ];

        let admitted = packs.len();
        Self {
            model_runtime_summary: format!(
                "Model Fabric · real portable ONNX runtime · {admitted} Pack(s) admitted this session"
            ),
            model_runtime_boundary: "Core owns Pack admission, promotion and execution; real portable ONNX runtime admitted for synthetic/evidence-only evaluation. Model Center is an operator projection: session-local admitted inventory is read through Core; a qualification reference is not an installed model. Real PHI and accelerated-runtime promotion remain denied.".to_owned(),
            model_source_summary: "External model registries are artifact sources, never authority planes. Session inventory shows Core-admitted manifest state. Pack v0 signing binds identity/version/epoch, content digest, rights URI and SBOM reference; runtime requirements, benchmark links and promotion state are declarations/Core state, not signature authority. Exact external source revision appears only for the pinned qualification reference where repository evidence proves it.".to_owned(),
            model_inventory_summary: if packs.is_empty() {
                "No Pack is admitted in this Desktop session. Pack registry state is currently session-local, not restart-persistent.".to_owned()
            } else {
                format!("{admitted} Core-admitted Pack(s) in this Desktop session. Restart persistence is not claimed.")
            },
            openmed_baseline: format!(
                "OpenMed v2.2.0 · commit {} · tree {}",
                OPENMED_BASELINE_COMMIT, OPENMED_BASELINE_TREE
            ),
            competitive_summary: if ledger.valid {
                format!(
                    "Claim ledger {}/39 accounted · {} unmeasured · {} structural-only · {} waived · {} pattern-only · {} BenchmarkManifest(s). No parity, surpass, privacy-superiority or runtime-winner claim is authorized.",
                    ledger.total,
                    ledger.unmeasured,
                    ledger.structural_only,
                    ledger.waived,
                    ledger.pattern_only,
                    ledger.benchmark_manifests
                )
            } else {
                "Comparative evidence ledger is invalid. Fail closed: no parity, surpass, structural-advantage or runtime-winner claim is authorized.".to_owned()
            },
            models,
            evidence,
        }
    }
}

fn pack_row(pack: &PackManifestV0) -> ModelStatusVm {
    let digest = pack.content_digest.to_hex();
    let task = if pack
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == PackArtifactKind::OnnxModel)
    {
        "Local ONNX Pack"
    } else if pack
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == PackArtifactKind::FixtureBytes)
    {
        "Local fixture Pack"
    } else {
        "Local Pack"
    };
    let benchmark = if pack.benchmark_links.is_empty() {
        "No benchmark links declared".to_owned()
    } else {
        format!("Declared evidence · {}", pack.benchmark_links.join(" · "))
    };
    let state = match pack.promotion_state {
        PackPromotionState::Candidate => "CANDIDATE",
        PackPromotionState::Current => "CURRENT",
        PackPromotionState::LastGreen => "LAST GREEN",
        PackPromotionState::Canary => "CANARY",
    };

    ModelStatusVm {
        scope: "SESSION INVENTORY".to_owned(),
        task: task.to_owned(),
        model: format!("{} · v{}", pack.pack_id.as_str(), pack.version),
        source: format!("Signed Pack · rights {}", pack.rights_uri),
        runtime: format!("Declared · {}", pack.runtime_requirements),
        device: runtime_device(&pack.runtime_requirements).to_owned(),
        trust: format!("Core-admitted · trust root {}", pack.trust_root_id),
        benchmark,
        digest,
        state: state.to_owned(),
    }
}

fn runtime_device(runtime: &str) -> &'static str {
    let lower = runtime.to_ascii_lowercase();
    if lower.contains("cpu") || lower.contains("tract") {
        "Local CPU / worker boundary"
    } else {
        "Local worker boundary"
    }
}

#[cfg(test)]
mod tests {
    use super::{
        OPENMED_BASELINE_COMMIT, OPENMED_BASELINE_TREE, OPENMED_CLAIM_LEDGER_JSON,
        ProductIntelligenceVm, summarize_claim_ledger,
    };
    use medscale_contracts::objects::{DigestSha256, OpaqueId};
    use medscale_contracts::packs::{
        PackArtifactEntry, PackArtifactKind, PackManifestV0, PackPromotionState,
    };

    fn manifest(state: PackPromotionState) -> PackManifestV0 {
        PackManifestV0 {
            pack_id: OpaqueId::new("pack-model-center-070"),
            version: "1.2.3".to_owned(),
            pack_epoch: 7,
            content_digest: DigestSha256::of(b"model-center-070"),
            artifacts: vec![PackArtifactEntry {
                relative_path: "model.onnx".to_owned(),
                kind: PackArtifactKind::OnnxModel,
                digest: DigestSha256::of(b"onnx"),
            }],
            rights_uri: "https://example.test/rights".to_owned(),
            sbom_ref: "sbom:test".to_owned(),
            runtime_requirements: "tract-onnx 0.22.4 · portable CPU".to_owned(),
            benchmark_links: vec!["evidence://070/runtime".to_owned()],
            promotion_state: state,
            trust_root_id: "synthetic-pack-trust-v1".to_owned(),
            signature_hex: "00".repeat(64),
        }
    }

    #[test]
    fn empty_inventory_is_explicit_and_qualification_is_not_installed() {
        let vm = ProductIntelligenceVm::current_truth();
        assert_eq!(vm.models[0].scope, "SESSION INVENTORY");
        assert_eq!(vm.models[0].state, "EMPTY");
        assert!(vm.model_inventory_summary.contains("session-local"));
        assert!(vm.models.iter().any(|row| {
            row.scope == "QUALIFICATION REFERENCE" && row.state == "QUALIFIED BASELINE"
        }));
    }

    #[test]
    fn admitted_pack_row_exposes_runtime_trust_digest_benchmark_and_promotion() {
        let pack = manifest(PackPromotionState::Current);
        let expected_digest = pack.content_digest.to_hex();
        let vm = ProductIntelligenceVm::from_packs(&[pack]);
        let row = &vm.models[0];
        assert_eq!(row.scope, "SESSION INVENTORY");
        assert_eq!(row.state, "CURRENT");
        assert!(row.model.contains("pack-model-center-070"));
        assert!(row.runtime.contains("Declared"));
        assert!(row.runtime.contains("tract-onnx"));
        assert!(row.trust.contains("synthetic-pack-trust-v1"));
        assert_eq!(row.digest, expected_digest);
        assert!(row.benchmark.contains("evidence://070/runtime"));
        assert!(row.source.contains("https://example.test/rights"));
    }

    #[test]
    fn session_projection_reads_core_admitted_pack_inventory() {
        let mut session =
            medscale_core::CliSession::connect_pack_operator("desktop-model-center-test")
                .expect("connect model center test session");
        let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../evidence/008-local-ai-capability-fabric/fixtures/pack-fixture-ner-v0");
        let admitted = session
            .packs_install_local(&fixture.display().to_string())
            .expect("admit signed local fixture through Core");
        assert!(admitted.admitted);

        let denied = session.open_synthetic_vault("/tmp/model-center-must-not-open-vault");
        assert!(
            matches!(
                denied,
                Err(medscale_contracts::envelopes::AuthorityError::SessionDenied)
            ),
            "Model Center session must not inherit broad CLI authority"
        );

        let vm = ProductIntelligenceVm::from_session(&mut session)
            .expect("read Core-backed session inventory");
        let row = &vm.models[0];
        assert_eq!(row.scope, "SESSION INVENTORY");
        assert!(row.model.contains("pack-fixture-ner-v0"));
        assert_ne!(row.state, "EMPTY");
        assert!(!row.digest.is_empty());
    }

    #[test]
    fn current_truth_never_fakes_model_runtime_or_openmed_superiority() {
        let vm = ProductIntelligenceVm::current_truth();
        assert!(
            vm.model_runtime_summary
                .contains("real portable ONNX runtime")
        );
        assert!(vm.models.iter().any(|row| row.state == "MODEL GAP"));
        assert!(vm.evidence.iter().any(|row| row.verdict == "UNMEASURED"));
        assert!(
            vm.evidence
                .iter()
                .any(|row| row.verdict == "STRUCTURAL ONLY")
        );
        assert!(vm.evidence.iter().any(|row| row.verdict == "ANTI-METRIC"));
        assert!(vm.evidence.iter().all(|row| {
            !matches!(
                row.verdict.as_str(),
                "PROVEN ADVANTAGE" | "OPENMED AHEAD" | "PARITY" | "SURPASS"
            )
        }));
        assert!(vm.competitive_summary.contains("39/39 accounted"));
        assert!(vm.competitive_summary.contains("0 BenchmarkManifest(s)"));
        assert!(vm.competitive_summary.contains("No parity, surpass"));
        assert!(vm.openmed_baseline.contains(OPENMED_BASELINE_COMMIT));
        assert!(vm.openmed_baseline.contains(OPENMED_BASELINE_TREE));
        assert!(vm.model_source_summary.contains("not signature authority"));
    }

    #[test]
    fn spec_071_claim_ledger_is_complete_and_fail_closed() {
        let summary = summarize_claim_ledger(OPENMED_CLAIM_LEDGER_JSON);
        assert!(summary.valid);
        assert_eq!(summary.total, 39);
        assert_eq!(summary.unmeasured, 22);
        assert_eq!(summary.waived, 10);
        assert_eq!(summary.structural_only, 6);
        assert_eq!(summary.pattern_only, 1);
        assert_eq!(summary.benchmark_manifests, 0);
    }

    #[test]
    fn duplicate_capability_claim_ledger_fails_closed() {
        let mut value: serde_json::Value =
            serde_json::from_str(OPENMED_CLAIM_LEDGER_JSON).expect("valid canonical ledger");
        let rows = value["rows"].as_array_mut().expect("ledger rows");
        rows[1]["capability_id"] = rows[0]["capability_id"].clone();
        let malformed = serde_json::to_string(&value).expect("serialize malformed ledger");
        let summary = summarize_claim_ledger(&malformed);
        assert!(!summary.valid);
    }

    #[test]
    fn malformed_claim_ledger_fails_closed() {
        let summary = summarize_claim_ledger(r#"{"rows": []}"#);
        assert!(!summary.valid);
        assert_eq!(summary.benchmark_manifests, 0);
    }
}
