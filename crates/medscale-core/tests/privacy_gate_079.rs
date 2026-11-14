//! Spec 079 Privacy Gate Core authority integration tests.
//!
//! Every operation travels request -> session -> realm/scope -> Privacy Gate
//! authority -> storage through `CoreFacade::dispatch`. Synthetic data only.
//! Covers `security.md` T1-T11 and the frozen acceptance requirements in
//! `SPEC_079_PROMOTION.md` (T079-04..07).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_contracts::privacy_gate::{
    ClassificationBasis, DataClass, DeidReceipt, DeidReceiptStatus, EffectiveClassification,
    EgressBoundary, EgressDecision, EgressOutcome, EgressReason, PrivacyPolicyProfile, ProfileRule,
    PseudonymMapRef, RecognizerFamily, RecognizerStatus, ReidentificationAudit,
    ReidentificationOutcome, ResidualScanStatus, SensitiveSpanKind, TransformOp, is_pseudonym,
};
use medscale_core::CoreFacade;
use medscale_keys::{KeyStore, KeyStoreError, MemoryKeyStore};
use serde_json::Value;

const REALM: &str = "realm-a";
const SCOPE: &str = "scope-a";
const VAULT: &str = "vault-1";

fn tmp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("medscale-079c-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn onnx_pack() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../evidence/069-real-local-model-runtime-hf-pack-path/fixtures/pack-tiny-token-classifier-v0",
    )
}

fn corpus() -> Vec<Value> {
    let raw = include_str!("fixtures/privacy_079/corpus.json");
    serde_json::from_str::<Value>(raw)
        .unwrap()
        .as_array()
        .unwrap()
        .clone()
}

/// A key store shared across facade instances, standing in for a durable
/// OS keyring so keys survive a simulated process restart.
#[derive(Clone, Default)]
struct SharedKeys(Arc<MemoryKeyStore>);

impl KeyStore for SharedKeys {
    fn put(&self, account: &str, secret: &[u8]) -> Result<(), KeyStoreError> {
        self.0.put(account, secret)
    }
    fn get(&self, account: &str) -> Result<Vec<u8>, KeyStoreError> {
        self.0.get(account)
    }
    fn delete(&self, account: &str) -> Result<(), KeyStoreError> {
        self.0.delete(account)
    }
}

fn request(scope: &str, id: u64, capability: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new(format!("req-{id}")),
        VaultId::new(VAULT),
        RealmId::new(REALM),
        AuthorityScopeId::new(scope),
        capability,
        body,
    )
}

struct Harness {
    facade: CoreFacade,
    session: OpaqueId,
    holder: OpaqueId,
    dir: PathBuf,
    keys: SharedKeys,
    next_req: u64,
}

impl Harness {
    fn setup(name: &str) -> Self {
        Self::open_at(tmp_dir(name), SharedKeys::default())
    }

    fn open_at(dir: PathBuf, keys: SharedKeys) -> Self {
        let mut facade = CoreFacade::new();
        facade.set_privacy_key_store(Box::new(keys.clone()));
        let mut h = Harness {
            facade,
            session: OpaqueId::new("pending"),
            holder: OpaqueId::new("pending"),
            dir,
            keys,
            next_req: 0,
        };
        let ResponseBody::Lease { holder_id, .. } = h
            .raw(
                None,
                Capability::AcquireLease,
                RequestBody::AcquireLease {
                    client_id: OpaqueId::new("actor-a"),
                    holder_id_hint: Some(OpaqueId::new("actor-a")),
                },
            )
            .unwrap()
        else {
            panic!("lease");
        };
        h.holder = holder_id.clone();
        h.session = h.open_session(Capability::operator_grants());
        let vault_root = h.dir.to_str().unwrap().to_owned();
        h.call(
            Capability::OpenSyntheticVault,
            RequestBody::OpenSyntheticVault { vault_root },
        )
        .expect("vault");
        h
    }

    fn open_session(&mut self, granted: Vec<Capability>) -> OpaqueId {
        let ResponseBody::Session { session_id, .. } = self
            .raw(
                None,
                Capability::OpenSession,
                RequestBody::OpenSession {
                    holder_id: self.holder.clone(),
                    granted,
                    ttl_ticks: 1_000_000,
                },
            )
            .unwrap()
        else {
            panic!("session");
        };
        session_id
    }

    /// Simulates a restart. `keep_keys` false models a machine without the
    /// original key store.
    fn reopen(self, keep_keys: bool) -> Self {
        let dir = self.dir.clone();
        let keys = if keep_keys {
            self.keys.clone()
        } else {
            SharedKeys::default()
        };
        drop(self);
        Self::open_at(dir, keys)
    }

    fn raw(
        &mut self,
        session: Option<OpaqueId>,
        capability: Capability,
        body: RequestBody,
    ) -> Result<ResponseBody, AuthorityError> {
        self.next_req += 1;
        let mut req = request(SCOPE, self.next_req, capability, body);
        req.session_id = session;
        self.facade.dispatch(req).result
    }

    fn call(
        &mut self,
        capability: Capability,
        body: RequestBody,
    ) -> Result<ResponseBody, AuthorityError> {
        let session = Some(self.session.clone());
        self.raw(session, capability, body)
    }

    fn call_in_scope(
        &mut self,
        scope: &str,
        capability: Capability,
        body: RequestBody,
    ) -> Result<ResponseBody, AuthorityError> {
        self.next_req += 1;
        let mut req = request(scope, self.next_req, capability, body);
        req.session_id = Some(self.session.clone());
        self.facade.dispatch(req).result
    }

    fn project(&mut self, name: &str) -> OpaqueId {
        match self
            .call(
                Capability::ProjectCreate,
                RequestBody::ProjectCreate {
                    name: name.to_owned(),
                    description: None,
                },
            )
            .unwrap()
        {
            ResponseBody::Project { project } => project.header.id,
            other => panic!("{other:?}"),
        }
    }

    fn source(&mut self, media_type: &str, bytes: &[u8]) -> OpaqueId {
        match self
            .call(
                Capability::CreateSourceRecord,
                RequestBody::CreateSourceRecord {
                    media_type: media_type.to_owned(),
                    bytes: bytes.to_vec(),
                },
            )
            .unwrap()
        {
            ResponseBody::Created { object_id } => object_id,
            other => panic!("{other:?}"),
        }
    }

    fn read_bytes(&mut self, id: &OpaqueId) -> Vec<u8> {
        match self
            .call(
                Capability::ReadObject,
                RequestBody::ReadObject {
                    object_id: id.clone(),
                },
            )
            .unwrap()
        {
            ResponseBody::Object { value: object } => {
                let bytes = object["bytes"].as_array().expect("bytes");
                bytes
                    .iter()
                    .map(|b| u8::try_from(b.as_u64().unwrap()).unwrap())
                    .collect()
            }
            other => panic!("{other:?}"),
        }
    }

    fn classify(
        &mut self,
        project: &OpaqueId,
        artifact: &OpaqueId,
        class: DataClass,
        expected: Option<u64>,
    ) -> Result<ResponseBody, AuthorityError> {
        self.call(
            Capability::PrivacyClassify,
            RequestBody::PrivacyClassify {
                project_id: project.clone(),
                artifact_id: artifact.clone(),
                data_class: class,
                expected_revision: expected,
            },
        )
    }

    fn effective(&mut self, project: &OpaqueId, artifact: &OpaqueId) -> EffectiveClassification {
        match self
            .call(
                Capability::PrivacyRead,
                RequestBody::PrivacyClassificationGet {
                    project_id: project.clone(),
                    artifact_id: artifact.clone(),
                },
            )
            .unwrap()
        {
            ResponseBody::PrivacyEffectiveClassification { effective } => *effective,
            other => panic!("{other:?}"),
        }
    }

    fn profile_with(
        &mut self,
        project: &OpaqueId,
        rules: Vec<ProfileRule>,
        use_model: bool,
    ) -> Result<PrivacyPolicyProfile, AuthorityError> {
        match self.call(
            Capability::PrivacyProfileCreate,
            RequestBody::PrivacyProfileCreate {
                project_id: project.clone(),
                name: "research export".to_owned(),
                target_class: DataClass::ExternalDeidentified,
                rules,
                use_model_recognizer: use_model,
            },
        )? {
            ResponseBody::PrivacyProfile { profile } => Ok(*profile),
            other => panic!("{other:?}"),
        }
    }

    fn standard_profile(&mut self, project: &OpaqueId, use_model: bool) -> PrivacyPolicyProfile {
        self.profile_with(project, standard_rules(), use_model)
            .unwrap()
    }

    fn map(&mut self, project: &OpaqueId) -> PseudonymMapRef {
        match self
            .call(
                Capability::PrivacyMapCreate,
                RequestBody::PrivacyMapCreate {
                    project_id: project.clone(),
                },
            )
            .unwrap()
        {
            ResponseBody::PrivacyMap { map } => *map,
            other => panic!("{other:?}"),
        }
    }

    fn transform_full(
        &mut self,
        project: &OpaqueId,
        source: &OpaqueId,
        profile: &OpaqueId,
        map: Option<&OpaqueId>,
        model: Option<(OpaqueId, String)>,
        synthetic_only: bool,
    ) -> Result<DeidReceipt, AuthorityError> {
        let (model_pack_id, model_pack_path) = match model {
            Some((id, path)) => (Some(id), Some(path)),
            None => (None, None),
        };
        match self.call(
            Capability::PrivacyTransform,
            RequestBody::PrivacyTransform {
                project_id: project.clone(),
                source_artifact_id: source.clone(),
                profile_id: profile.clone(),
                pseudonym_map_id: map.cloned(),
                model_pack_id,
                model_pack_path,
                synthetic_only,
            },
        )? {
            ResponseBody::PrivacyReceipt { receipt } => Ok(*receipt),
            other => panic!("{other:?}"),
        }
    }

    fn transform(
        &mut self,
        project: &OpaqueId,
        source: &OpaqueId,
        profile: &OpaqueId,
        map: Option<&OpaqueId>,
    ) -> Result<DeidReceipt, AuthorityError> {
        self.transform_full(project, source, profile, map, None, true)
    }

    fn egress(
        &mut self,
        project: &OpaqueId,
        artifact: &OpaqueId,
        boundary: EgressBoundary,
    ) -> EgressDecision {
        match self
            .call(
                Capability::PrivacyEgressEvaluate,
                RequestBody::PrivacyEgressEvaluate {
                    project_id: project.clone(),
                    artifact_id: artifact.clone(),
                    boundary,
                },
            )
            .unwrap()
        {
            ResponseBody::PrivacyEgressDecision { decision } => *decision,
            other => panic!("{other:?}"),
        }
    }

    fn reidentify(
        &mut self,
        map: &OpaqueId,
        pseudonym: &str,
    ) -> Result<(ReidentificationAudit, Option<String>), AuthorityError> {
        let session = self.open_session(vec![Capability::PrivacyReidentify]);
        match self.raw(
            Some(session),
            Capability::PrivacyReidentify,
            RequestBody::PrivacyReidentify {
                map_id: map.clone(),
                pseudonym: pseudonym.to_owned(),
                reason: "synthetic study follow-up".to_owned(),
            },
        )? {
            ResponseBody::PrivacyReidentified { audit, value } => Ok((*audit, value)),
            other => panic!("{other:?}"),
        }
    }

    fn meta_path(&self) -> PathBuf {
        self.dir.join("meta.sqlite3")
    }
}

fn standard_rules() -> Vec<ProfileRule> {
    SensitiveSpanKind::ALL
        .iter()
        .map(|kind| ProfileRule {
            kind: *kind,
            op: match kind {
                SensitiveSpanKind::PersonName | SensitiveSpanKind::Identifier => {
                    TransformOp::Pseudonymize
                }
                SensitiveSpanKind::Date
                | SensitiveSpanKind::PostalCode
                | SensitiveSpanKind::IpAddress => TransformOp::Generalize,
                SensitiveSpanKind::Url => TransformOp::Drop,
                SensitiveSpanKind::Email => TransformOp::Tokenize,
                _ => TransformOp::Redact,
            },
        })
        .collect()
}

fn corpus_values(doc: &Value) -> Vec<String> {
    doc["expected"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["value"].as_str().unwrap().to_owned())
        .collect()
}

/// Every Privacy Gate row in the vault metadata, as JSON text.
fn privacy_rows_text(meta: &Path) -> String {
    let conn = rusqlite::Connection::open(meta).unwrap();
    let mut out = String::new();
    for table in [
        "privacy_classifications",
        "privacy_profiles",
        "privacy_deid_receipts",
        "privacy_pseudonym_maps",
        "privacy_pseudonym_entries",
        "privacy_reid_audit",
        "privacy_egress_decisions",
    ] {
        let mut stmt = conn
            .prepare(&format!("SELECT body_json FROM {table}"))
            .unwrap();
        let rows = stmt.query_map([], |r| r.get::<_, String>(0)).unwrap();
        for row in rows {
            out.push_str(&row.unwrap());
            out.push('\n');
        }
    }
    out
}

// ---------------------------------------------------------------------------
// classification (T1, T9, T10)
// ---------------------------------------------------------------------------

#[test]
fn unclassified_is_local_phi_and_declarations_are_revision_safe_and_durable() {
    let mut h = Harness::setup("classify");
    let project = h.project("p1");
    let other = h.project("p2");
    let art = h.source("text/plain", b"Public reference text.");

    let e = h.effective(&project, &art);
    assert_eq!(e.data_class, DataClass::LocalPhi);
    assert_eq!(e.basis, ClassificationBasis::DefaultUnclassified);
    assert!(e.classification.is_none());

    h.classify(&project, &art, DataClass::Public, None).unwrap();
    assert!(matches!(
        h.classify(&project, &art, DataClass::TeamProtected, None),
        Err(AuthorityError::Conflict { .. })
    ));
    assert!(matches!(
        h.classify(&project, &art, DataClass::TeamProtected, Some(7)),
        Err(AuthorityError::Conflict { .. })
    ));
    h.classify(&project, &art, DataClass::TeamProtected, Some(1))
        .unwrap();

    // Another Project in the same scope sees its own (absent) row.
    assert_eq!(
        h.effective(&other, &art).basis,
        ClassificationBasis::DefaultUnclassified
    );
    // A foreign scope cannot name the Project or the artifact.
    assert!(
        h.call_in_scope(
            "scope-b",
            Capability::PrivacyRead,
            RequestBody::PrivacyClassificationGet {
                project_id: project.clone(),
                artifact_id: art.clone(),
            },
        )
        .is_err()
    );

    let mut h = h.reopen(true);
    let e = h.effective(&project, &art);
    assert_eq!(e.data_class, DataClass::TeamProtected);
    assert_eq!(e.basis, ClassificationBasis::Declared);
    assert_eq!(e.classification.unwrap().revision, 2);
}

// ---------------------------------------------------------------------------
// profiles
// ---------------------------------------------------------------------------

#[test]
fn incomplete_profiles_are_refused_before_any_write() {
    let mut h = Harness::setup("profile");
    let project = h.project("p1");
    let mut rules = standard_rules();
    rules.pop();
    assert!(matches!(
        h.profile_with(&project, rules, false),
        Err(AuthorityError::InvalidArgument { .. })
    ));
    let ResponseBody::PrivacyProfileList { profiles } = h
        .call(
            Capability::PrivacyRead,
            RequestBody::PrivacyProfileList {
                project_id: project.clone(),
            },
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(profiles.is_empty());
}

// ---------------------------------------------------------------------------
// transform (T2, T3, T8, T11) over the synthetic corpus
// ---------------------------------------------------------------------------

#[test]
fn corpus_transforms_write_new_artifacts_and_bind_receipts_without_leaking_values() {
    let mut h = Harness::setup("corpus");
    let project = h.project("p1");
    let profile = h.standard_profile(&project, false);
    let map = h.map(&project);
    let mut all_values = Vec::new();

    for doc in corpus() {
        let text = doc["text"].as_str().unwrap();
        let media = doc["media_type"].as_str().unwrap();
        let source = h.source(media, text.as_bytes());
        let receipt = h
            .transform(&project, &source, &profile.header.id, Some(&map.header.id))
            .unwrap();

        // T2: source unchanged; a distinct new artifact carries the output.
        assert_eq!(h.read_bytes(&source), text.as_bytes());
        assert_ne!(receipt.output_artifact_id, source);
        let output = String::from_utf8(h.read_bytes(&receipt.output_artifact_id)).unwrap();
        assert_eq!(
            receipt.output_digest,
            medscale_contracts::objects::DigestSha256::of(output.as_bytes())
        );
        assert_eq!(
            receipt.source_digest,
            medscale_contracts::objects::DigestSha256::of(text.as_bytes())
        );
        assert_eq!(receipt.profile_revision, profile.revision);
        assert!(receipt.validate().is_ok());

        // No expected sensitive value survives in the output.
        for value in corpus_values(&doc) {
            assert!(
                !output.contains(&value),
                "{}: {value:?} survived in {output}",
                doc["id"]
            );
            all_values.push(value);
        }
        if media.contains("fhir") {
            let parsed: Value = serde_json::from_str(&output).expect("FHIR output stays JSON");
            assert_eq!(parsed["birthDate"], "1980");
            assert_eq!(parsed["gender"], "female");
        }
        assert_eq!(
            receipt.residual.status,
            ResidualScanStatus::NoResidualDetectedByAdmittedRecognizers,
            "{}: {output}",
            doc["id"]
        );
        let output_class = h.effective(&project, &receipt.output_artifact_id);
        assert_eq!(output_class.data_class, DataClass::ExternalDeidentified);
        assert_eq!(output_class.basis, ClassificationBasis::DeidReceipt);
        let _ = h.egress(
            &project,
            &receipt.output_artifact_id,
            EgressBoundary::Browse,
        );
        let _ = h.egress(&project, &source, EgressBoundary::Browse);
    }

    // T3: no persisted Privacy Gate row carries a corpus value.
    let rows = privacy_rows_text(&h.meta_path());
    for value in &all_values {
        let as_json = serde_json::to_string(value).unwrap();
        assert!(
            !contains_value(&rows, as_json.trim_matches('"')),
            "{value:?} leaked into a Privacy Gate row"
        );
    }
}

/// Whether `value` occurs in `text` other than inside a longer alphanumeric
/// run. Short corpus values (a five-digit postal code) otherwise match by
/// chance inside hex digests, ids and timestamps, which made this check
/// fail nondeterministically (found by the Spec 092 campaign CI).
fn contains_value(text: &str, value: &str) -> bool {
    text.match_indices(value).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + value.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

#[test]
fn value_matching_ignores_digest_collisions_but_finds_leaks() {
    assert!(!contains_value("\"digest\":\"ab10115cd\"", "10115"));
    assert!(!contains_value("\"at_ms\":1790101150000", "10115"));
    assert!(contains_value(
        "\"text\":\"Hauptstrasse 5, 10115 Berlin\"",
        "10115"
    ));
    assert!(contains_value("\"zip\":\"10115\"", "10115"));
    assert!(contains_value("Lukas Schneider", "Lukas Schneider"));
}

#[test]
fn pseudonyms_are_stable_within_a_map_and_distinct_across_maps() {
    let mut h = Harness::setup("pseudo");
    let project = h.project("p1");
    let profile = h.standard_profile(&project, false);
    let map_a = h.map(&project);
    let map_b = h.map(&project);
    let text = b"Patient: Maria Lopez, MRN: 998877.";
    let s1 = h.source("text/plain", text);
    let s2 = h.source("text/plain", text);
    let r1 = h
        .transform(&project, &s1, &profile.header.id, Some(&map_a.header.id))
        .unwrap();
    let r2 = h
        .transform(&project, &s2, &profile.header.id, Some(&map_a.header.id))
        .unwrap();
    let r3 = h
        .transform(&project, &s1, &profile.header.id, Some(&map_b.header.id))
        .unwrap();
    let o1 = h.read_bytes(&r1.output_artifact_id);
    let o2 = h.read_bytes(&r2.output_artifact_id);
    let o3 = h.read_bytes(&r3.output_artifact_id);
    assert_eq!(o1, o2, "same map, same values -> same pseudonyms");
    assert_ne!(o1, o3, "different maps -> different pseudonyms");
    let ResponseBody::PrivacyMapList { maps } = h
        .call(
            Capability::PrivacyRead,
            RequestBody::PrivacyMapList {
                project_id: project.clone(),
            },
        )
        .unwrap()
    else {
        panic!()
    };
    let a = maps
        .iter()
        .find(|m| m.header.id == map_a.header.id)
        .unwrap();
    assert_eq!(a.entry_count, 2, "a repeated value is stored once per map");

    // A pseudonymizing profile without a map is refused before any write.
    assert!(matches!(
        h.transform(&project, &s1, &profile.header.id, None),
        Err(AuthorityError::InvalidArgument { .. })
    ));
}

// ---------------------------------------------------------------------------
// re-identification (T4, T5, T6)
// ---------------------------------------------------------------------------

#[test]
fn reidentification_needs_its_own_capability_is_audited_and_ends_at_revocation() {
    let mut h = Harness::setup("reid");
    let project = h.project("p1");
    let profile = h.standard_profile(&project, false);
    let map = h.map(&project);
    let source = h.source("text/plain", b"Patient: Maria Lopez, MRN: 998877.");
    let receipt = h
        .transform(&project, &source, &profile.header.id, Some(&map.header.id))
        .unwrap();
    let output = String::from_utf8(h.read_bytes(&receipt.output_artifact_id)).unwrap();
    let pseudonym = output
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .find(|w| is_pseudonym(w))
        .unwrap()
        .to_owned();

    // The operator session does not hold PrivacyReidentify.
    let denied = h.call(
        Capability::PrivacyReidentify,
        RequestBody::PrivacyReidentify {
            map_id: map.header.id.clone(),
            pseudonym: pseudonym.clone(),
            reason: "no grant".to_owned(),
        },
    );
    assert!(denied.is_err(), "{denied:?}");
    assert!(!Capability::operator_grants().contains(&Capability::PrivacyReidentify));

    let (audit, value) = h.reidentify(&map.header.id, &pseudonym).unwrap();
    assert_eq!(audit.outcome, ReidentificationOutcome::Returned);
    assert!(matches!(value.as_deref(), Some("Maria Lopez" | "998877")));

    let (audit, value) = h.reidentify(&map.header.id, "PSN-000000000000").unwrap();
    assert_eq!(
        audit.outcome,
        ReidentificationOutcome::DeniedUnknownPseudonym
    );
    assert!(value.is_none());

    // T4: the key is in the key store only, never in vault metadata.
    let key = h.keys.get(&map.key_account).unwrap();
    let meta_bytes = fs::read(h.meta_path()).unwrap();
    assert!(!contains(&meta_bytes, &key));
    let key_hex: String = key.iter().map(|b| format!("{b:02x}")).collect();
    assert!(!contains(&meta_bytes, key_hex.as_bytes()));

    // A machine without the key store: re-identification denies, audited.
    let original_keys = h.keys.clone();
    let mut h2 = h.reopen(false);
    let (audit, value) = h2.reidentify(&map.header.id, &pseudonym).unwrap();
    assert_eq!(audit.outcome, ReidentificationOutcome::DeniedKeyUnavailable);
    assert!(value.is_none());
    let dir = h2.dir.clone();
    drop(h2);
    let mut h = Harness::open_at(dir, original_keys);
    let (audit, _) = h.reidentify(&map.header.id, &pseudonym).unwrap();
    assert_eq!(audit.outcome, ReidentificationOutcome::Returned);

    // Revocation destroys the key; later requests deny and are audited.
    let current = h
        .call(
            Capability::PrivacyRead,
            RequestBody::PrivacyMapList {
                project_id: project.clone(),
            },
        )
        .unwrap();
    let ResponseBody::PrivacyMapList { maps } = current else {
        panic!()
    };
    let rev = maps[0].revision;
    h.call(
        Capability::PrivacyMapRevoke,
        RequestBody::PrivacyMapRevoke {
            map_id: map.header.id.clone(),
            expected_revision: rev,
        },
    )
    .unwrap();
    assert!(matches!(
        h.keys.get(&map.key_account),
        Err(KeyStoreError::NotFound)
    ));
    let (audit, value) = h.reidentify(&map.header.id, &pseudonym).unwrap();
    assert_eq!(audit.outcome, ReidentificationOutcome::DeniedRevoked);
    assert!(value.is_none());
    // A revoked map accepts no new transform.
    assert!(matches!(
        h.transform(&project, &source, &profile.header.id, Some(&map.header.id)),
        Err(AuthorityError::Conflict { .. })
    ));

    let ResponseBody::PrivacyReidentificationAuditList { audits } = h
        .call(
            Capability::PrivacyRead,
            RequestBody::PrivacyReidentificationAuditList {
                map_id: map.header.id.clone(),
            },
        )
        .unwrap()
    else {
        panic!()
    };
    let outcomes: Vec<_> = audits.iter().map(|a| a.outcome).collect();
    assert_eq!(
        outcomes,
        vec![
            ReidentificationOutcome::Returned,
            ReidentificationOutcome::DeniedUnknownPseudonym,
            ReidentificationOutcome::DeniedKeyUnavailable,
            ReidentificationOutcome::Returned,
            ReidentificationOutcome::DeniedRevoked,
        ]
    );
    // Audit rows never hold the resolved value.
    let rows = privacy_rows_text(&h.meta_path());
    assert!(!rows.contains("Maria Lopez"));
    assert!(!rows.contains("998877"));
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
}

// ---------------------------------------------------------------------------
// egress (T1, T6, T7, T8)
// ---------------------------------------------------------------------------

#[test]
fn egress_fails_closed_and_every_decision_is_persisted() {
    let mut h = Harness::setup("egress");
    let project = h.project("p1");
    let profile = h.standard_profile(&project, false);
    let map = h.map(&project);
    let source = h.source("text/plain", b"Patient: Maria Lopez, seen 2024-03-12.");
    let public = h.source("text/plain", b"Public guideline excerpt.");
    h.classify(&project, &public, DataClass::Public, None)
        .unwrap();
    let team = h.source("text/plain", b"Team note.");
    h.classify(&project, &team, DataClass::TeamProtected, None)
        .unwrap();
    let declared_deid = h.source("text/plain", b"Claimed de-identified.");
    h.classify(
        &project,
        &declared_deid,
        DataClass::ExternalDeidentified,
        None,
    )
    .unwrap();
    let receipt = h
        .transform(&project, &source, &profile.header.id, Some(&map.header.id))
        .unwrap();
    let output = receipt.output_artifact_id.clone();

    let mut expected = 0;
    for boundary in EgressBoundary::ALL {
        let b = *boundary;
        assert_eq!(
            h.egress(&project, &source, b).reason,
            EgressReason::DeniedUnclassified
        );
        assert_eq!(
            h.egress(&project, &public, b).reason,
            EgressReason::AllowedPublic
        );
        assert_eq!(
            h.egress(&project, &output, b).reason,
            EgressReason::AllowedDeidentified
        );
        assert_eq!(
            h.egress(&project, &declared_deid, b).reason,
            EgressReason::DeniedNoReceipt
        );
        let team_reason = h.egress(&project, &team, b).reason;
        if b == EgressBoundary::Hub {
            assert_eq!(team_reason, EgressReason::AllowedTeamHub);
        } else {
            assert_eq!(team_reason, EgressReason::DeniedTeamProtectedOffHub);
        }
        expected += 5;
    }
    h.classify(&project, &source, DataClass::LocalPhi, None)
        .unwrap();
    assert_eq!(
        h.egress(&project, &source, EgressBoundary::Compute).reason,
        EgressReason::DeniedLocalPhi
    );
    expected += 1;

    // Revoking the receipt makes its output deny.
    h.call(
        Capability::PrivacyReceiptRevoke,
        RequestBody::PrivacyReceiptRevoke {
            receipt_id: receipt.header.id.clone(),
            expected_revision: 1,
        },
    )
    .unwrap();
    let d = h.egress(&project, &output, EgressBoundary::Browse);
    assert_eq!(d.reason, EgressReason::DeniedReceiptRevoked);
    assert_eq!(d.outcome, EgressOutcome::Deny);
    expected += 1;

    // A revoked profile also denies outputs it produced.
    let source2 = h.source("text/plain", b"Patient: Ana Diaz.");
    let receipt2 = h
        .transform(&project, &source2, &profile.header.id, Some(&map.header.id))
        .unwrap();
    h.call(
        Capability::PrivacyProfileRevoke,
        RequestBody::PrivacyProfileRevoke {
            profile_id: profile.header.id.clone(),
            expected_revision: 1,
        },
    )
    .unwrap();
    assert_eq!(
        h.egress(
            &project,
            &receipt2.output_artifact_id,
            EgressBoundary::Browse
        )
        .reason,
        EgressReason::DeniedProfileRevoked
    );
    expected += 1;
    // And refuses new transforms.
    assert!(matches!(
        h.transform(&project, &source2, &profile.header.id, Some(&map.header.id)),
        Err(AuthorityError::Conflict { .. })
    ));

    // Cross-Project: the output's classification lives in p1 only.
    let other = h.project("p2");
    assert_eq!(
        h.egress(&other, &receipt2.output_artifact_id, EgressBoundary::Browse)
            .reason,
        EgressReason::DeniedUnclassified
    );
    expected += 1;

    let mut h = h.reopen(true);
    let ResponseBody::PrivacyEgressDecisionList { decisions } = h
        .call(
            Capability::PrivacyRead,
            RequestBody::PrivacyEgressDecisionList {
                project_id: project.clone(),
                artifact_id: None,
            },
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(decisions.len(), expected - 1, "every p1 decision persisted");
    assert!(decisions.iter().all(|d| d.validate().is_ok()));
}

#[test]
fn a_receipt_whose_output_digest_no_longer_matches_denies() {
    let mut h = Harness::setup("digest");
    let project = h.project("p1");
    let profile = h.standard_profile(&project, false);
    let map = h.map(&project);
    let source = h.source("text/plain", b"Patient: Maria Lopez.");
    let receipt = h
        .transform(&project, &source, &profile.header.id, Some(&map.header.id))
        .unwrap();
    let meta = h.meta_path();
    let h = h.reopen(true);
    drop(h);
    // Point the receipt at a different digest (body and columns stay
    // consistent, so only the egress check can catch it).
    let conn = rusqlite::Connection::open(&meta).unwrap();
    let body: String = conn
        .query_row(
            "SELECT body_json FROM privacy_deid_receipts WHERE receipt_id = ?1",
            [receipt.header.id.as_str()],
            |r| r.get(0),
        )
        .unwrap();
    let mut value: Value = serde_json::from_str(&body).unwrap();
    value["output_digest"] =
        serde_json::to_value(medscale_contracts::objects::DigestSha256::of(b"other")).unwrap();
    conn.execute(
        "UPDATE privacy_deid_receipts SET body_json = ?1 WHERE receipt_id = ?2",
        rusqlite::params![value.to_string(), receipt.header.id.as_str()],
    )
    .unwrap();
    drop(conn);
    let mut h = Harness::open_at(meta.parent().unwrap().to_path_buf(), SharedKeys::default());
    assert_eq!(
        h.egress(
            &project,
            &receipt.output_artifact_id,
            EgressBoundary::Browse
        )
        .reason,
        EgressReason::DeniedDigestMismatch
    );
}

// ---------------------------------------------------------------------------
// recognizer availability, model recognizer, gates, malformed input
// ---------------------------------------------------------------------------

#[test]
fn unavailable_or_failed_recognizers_never_yield_a_clean_residual() {
    let mut h = Harness::setup("unavailable");
    let project = h.project("p1");
    let model_profile = h.standard_profile(&project, true);
    let plain_profile = h.standard_profile(&project, false);
    let map = h.map(&project);

    // Model recognizer requested by the profile but no Pack supplied.
    let source = h.source("text/plain", b"Patient: Maria Lopez.");
    let r = h
        .transform(
            &project,
            &source,
            &model_profile.header.id,
            Some(&map.header.id),
        )
        .unwrap();
    assert_eq!(r.residual.status, ResidualScanStatus::Unavailable);
    assert_eq!(
        h.egress(&project, &r.output_artifact_id, EgressBoundary::Browse)
            .reason,
        EgressReason::DeniedResidualUnavailable
    );

    // A Pack path that does not admit: the model recognizer is Unavailable.
    let r = h
        .transform_full(
            &project,
            &source,
            &model_profile.header.id,
            Some(&map.header.id),
            Some((OpaqueId::new("pack-x"), "/nonexistent/pack".to_owned())),
            true,
        )
        .unwrap();
    let model = r
        .recognizers
        .iter()
        .find(|x| x.recognizer.family == RecognizerFamily::LocalModel)
        .unwrap();
    assert_eq!(model.status, RecognizerStatus::Unavailable);
    assert_eq!(r.residual.status, ResidualScanStatus::Unavailable);

    // Declared FHIR that does not parse: the FHIR recognizer failed.
    let bad = h.source("application/fhir+json", b"{\"resourceType\": \"Patient\", ");
    let r = h
        .transform(
            &project,
            &bad,
            &plain_profile.header.id,
            Some(&map.header.id),
        )
        .unwrap();
    assert!(
        r.recognizers
            .iter()
            .any(|x| x.status == RecognizerStatus::Failed)
    );
    assert_eq!(r.residual.status, ResidualScanStatus::Unavailable);
    assert_eq!(
        h.egress(&project, &r.output_artifact_id, EgressBoundary::Browse)
            .reason,
        EgressReason::DeniedResidualUnavailable
    );
}

#[test]
fn admitted_local_model_recognizer_runs_and_is_recorded() {
    let mut h = Harness::setup("model");
    let project = h.project("p1");
    let pack_path = onnx_pack().display().to_string();
    let pack_id = match h
        .call(
            Capability::PacksInstallLocal,
            RequestBody::PacksInstallLocal {
                local_path: pack_path.clone(),
            },
        )
        .unwrap()
    {
        ResponseBody::PackAdmit { result } => result.pack_id.expect("admitted"),
        other => panic!("{other:?}"),
    };
    let profile = h.standard_profile(&project, true);
    let map = h.map(&project);
    let source = h.source("text/plain", b"Patient: Maria Lopez was seen today.");
    let r = h
        .transform_full(
            &project,
            &source,
            &profile.header.id,
            Some(&map.header.id),
            Some((pack_id.clone(), pack_path)),
            true,
        )
        .unwrap();
    let model = r
        .recognizers
        .iter()
        .find(|x| x.recognizer.family == RecognizerFamily::LocalModel)
        .expect("model recognizer recorded");
    assert_eq!(model.status, RecognizerStatus::Completed);
    assert_eq!(model.recognizer.model_pack_id.as_ref(), Some(&pack_id));
    assert!(r.limitations.contains(
        &medscale_contracts::privacy_gate::ReceiptLimitation::ModelRecognizerNotQualified
    ));
}

#[test]
fn real_phi_malformed_and_bounded_inputs_are_refused_without_writes() {
    let mut h = Harness::setup("refusals");
    let project = h.project("p1");
    let profile = h.standard_profile(&project, false);
    let map = h.map(&project);
    let source = h.source("text/plain", b"Patient: Maria Lopez.");
    assert!(matches!(
        h.transform_full(
            &project,
            &source,
            &profile.header.id,
            Some(&map.header.id),
            None,
            false
        ),
        Err(AuthorityError::ExternalGateRequired { .. })
    ));
    let binary = h.source("application/octet-stream", &[0xff, 0xfe, 0x00, 0x81]);
    assert!(matches!(
        h.transform(&project, &binary, &profile.header.id, Some(&map.header.id)),
        Err(AuthorityError::InvalidArgument { .. })
    ));
    let big = vec![b'a'; medscale_contracts::privacy_gate::TRANSFORM_INPUT_MAX_BYTES + 1];
    let big = h.source("text/plain", &big);
    assert!(matches!(
        h.transform(&project, &big, &profile.header.id, Some(&map.header.id)),
        Err(AuthorityError::InvalidArgument { .. })
    ));
    let ResponseBody::PrivacyReceiptList { receipts } = h
        .call(
            Capability::PrivacyRead,
            RequestBody::PrivacyReceiptList {
                project_id: project.clone(),
            },
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(receipts.is_empty(), "refusals write no receipt");
    // A receipt status is Valid until revoked.
    let r = h
        .transform(&project, &source, &profile.header.id, Some(&map.header.id))
        .unwrap();
    assert_eq!(r.status, DeidReceiptStatus::Valid);
}
