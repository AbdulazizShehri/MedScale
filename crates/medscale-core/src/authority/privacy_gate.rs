//! Privacy Gate Core authority paths (Spec 079).
//!
//! Core owns every classification, profile, transform, receipt, pseudonym
//! map, re-identification and egress decision. CLI and Desktop reach this
//! module only through `CoreFacade` requests.
//!
//! Invariants enforced here (`security.md`):
//! - an artifact without a classification row is `LocalPhi` (T1);
//! - a transform writes a new derived artifact; the source is never
//!   overwritten and its digest is re-verified (T2);
//! - receipts, decisions, audit rows and error messages never carry a
//!   detected plaintext value (T3);
//! - pseudonym map keys live only in the `KeyStore` (T4);
//! - re-identification is its own capability and is audited before any
//!   value is returned (T5);
//! - revocation, digest drift and unavailable recognizers all make egress
//!   deny (T6/T7/T8).
//!
//! This module decides egress; it never sends anything. It imports no
//! network code.

use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::{
    AuthorityScopeId, DerivedSourceArtifact, DigestSha256, LossClass, ObjectHeader, OpaqueId,
    RealmId, RepresentationKind, VaultId,
};
use medscale_contracts::privacy_gate::{
    ArtifactClassification, ClassificationBasis, DataClass, DeidReceipt, DeidReceiptStatus,
    EffectiveClassification, EgressBoundary, EgressDecision, MAX_SPANS_PER_TRANSFORM,
    PRIVACY_GATE_SCHEMA_VERSION, PrivacyPolicyProfile, PrivacyProfileStatus, ProfileRule,
    PseudonymMapRef, PseudonymMapStatus, ReceiptEvidence, ReceiptLimitation, RecognizerFamily,
    RecognizerIdentity, RecognizerResult, RecognizerStatus, ReidentificationAudit,
    ReidentificationOutcome, ResidualScanResult, SensitiveSpan, SensitiveSpanKind,
    TRANSFORM_INPUT_MAX_BYTES, TransformOp, TransformOpCount, decide_egress, is_pseudonym,
};
use medscale_keys::{KeyStore, KeyStoreError, WrappedBlob, generate_key32, seal, unseal};
use medscale_storage::{DeidTransformCommit, MetaError, PseudonymEntryRow, SqliteMetaStore};

use super::privacy_recognizers::{
    FhirOutcome, MODEL_RECOGNIZER_ID, MODEL_RECOGNIZER_VERSION, apply_transform, fhir_identity,
    fhir_recognize, is_transformed_value, pattern_identity, pattern_recognize, pseudonym_for,
    resolve_overlaps,
};
use super::store::{InMemoryAuthorityStore, ScopeError, StoredObject};
use crate::process::{LeaseRegistry, SessionRegistry};

/// `transform_id` recorded on every derived artifact this module writes.
pub const PRIVACY_TRANSFORM_ID: &str = "medscale.privacy_gate.deid";
pub const PRIVACY_TRANSFORM_VERSION: &str = "1";

fn meta_err(err: MetaError) -> AuthorityError {
    match err {
        MetaError::NotFound => AuthorityError::NotFound,
        MetaError::Conflict(message) => AuthorityError::Conflict { message },
        MetaError::UnsupportedSchema(message) => AuthorityError::UnsupportedSchema { message },
        MetaError::CorruptObjectBody(message) => AuthorityError::Corrupt { message },
        other => AuthorityError::Internal {
            message: other.to_string(),
        },
    }
}

fn scope_err(err: ScopeError) -> AuthorityError {
    match err {
        ScopeError::NotFound => AuthorityError::NotFound,
        ScopeError::WrongScope => AuthorityError::WrongScope,
    }
}

fn invalid(message: impl Into<String>) -> AuthorityError {
    AuthorityError::InvalidArgument {
        message: message.into(),
    }
}

/// Local model recognizer request for one transform.
#[derive(Debug, Clone)]
pub struct ModelRecognizerRequest {
    pub pack_id: OpaqueId,
    pub local_path: String,
}

/// Parameters of one privacy transform.
#[derive(Debug, Clone)]
pub struct TransformRequest {
    pub project_id: OpaqueId,
    pub source_artifact_id: OpaqueId,
    pub profile_id: OpaqueId,
    pub pseudonym_map_id: Option<OpaqueId>,
    pub model: Option<ModelRecognizerRequest>,
    pub synthetic_only: bool,
}

/// Bytes and identity of an artifact read for a transform or egress check.
struct ArtifactBytes {
    bytes: Vec<u8>,
    digest: DigestSha256,
    declared_fhir: bool,
}

/// One recognizer run over a text.
struct Recognition {
    results: Vec<RecognizerResult>,
    spans: Vec<SensitiveSpan>,
}

/// Authenticated Privacy Gate authority view for one request.
pub struct PrivacyGate<'a> {
    pub store: &'a mut InMemoryAuthorityStore,
    pub meta: &'a SqliteMetaStore,
    pub packs: &'a medscale_pack::PackStore,
    pub keys: &'a dyn KeyStore,
    pub sessions: &'a SessionRegistry,
    pub leases: &'a LeaseRegistry,
    pub vault_id: &'a VaultId,
    pub realm: RealmId,
    pub scope: AuthorityScopeId,
    pub session_id: Option<OpaqueId>,
}

impl PrivacyGate<'_> {
    fn actor(&self) -> Result<OpaqueId, AuthorityError> {
        if let Some(holder) = self
            .session_id
            .as_ref()
            .and_then(|session_id| self.sessions.holder_of(session_id))
        {
            return Ok(holder);
        }
        self.leases
            .holder(self.vault_id)
            .ok_or(AuthorityError::LeaseRequired)
    }

    /// Appends one scope-level audit row (ids only, never content).
    fn audit(&mut self, action: &str, targets: Vec<OpaqueId>) -> Result<(), AuthorityError> {
        let actor = self.actor()?;
        let id = self.store.alloc_id("audit");
        let record = medscale_contracts::objects::ActionAuditRecord {
            header: ObjectHeader {
                id,
                schema_version: medscale_contracts::AUTHORITY_SCHEMA_VERSION,
                realm_id: self.realm.clone(),
                authority_scope_id: self.scope.clone(),
            },
            kind: medscale_contracts::objects::ActionAuditKind::Audit,
            actor,
            action: action.to_owned(),
            target_refs: targets,
            effect_state: None,
            payload_digest: None,
            detail: None,
        };
        self.store.insert(StoredObject::Audit(record));
        Ok(())
    }

    fn header(&self, id: OpaqueId) -> ObjectHeader {
        ObjectHeader {
            id,
            schema_version: PRIVACY_GATE_SCHEMA_VERSION,
            realm_id: self.realm.clone(),
            authority_scope_id: self.scope.clone(),
        }
    }

    fn in_scope(&self, header: &ObjectHeader) -> Result<(), AuthorityError> {
        if header.realm_id != self.realm || header.authority_scope_id != self.scope {
            return Err(AuthorityError::WrongScope);
        }
        Ok(())
    }

    fn scoped_project(&self, id: &OpaqueId) -> Result<(), AuthorityError> {
        let project = self.meta.get_project(id).map_err(meta_err)?;
        self.in_scope(&project.header)
    }

    fn alloc(&self, prefix: &str) -> Result<OpaqueId, AuthorityError> {
        self.meta.alloc_privacy_id(prefix).map_err(meta_err)
    }

    /// Reads a source or derived artifact in this scope.
    fn artifact_bytes(&self, id: &OpaqueId) -> Result<ArtifactBytes, AuthorityError> {
        match self
            .store
            .get_scoped(id, &self.realm, &self.scope)
            .map_err(scope_err)?
        {
            StoredObject::Source(s) => Ok(ArtifactBytes {
                bytes: s.bytes.clone(),
                digest: s.content_digest.clone(),
                declared_fhir: s.media_type.contains("fhir"),
            }),
            StoredObject::Derived(d) => Ok(ArtifactBytes {
                bytes: d.bytes.clone(),
                digest: d.content_digest.clone(),
                declared_fhir: false,
            }),
            _ => Err(invalid("artifact is not a source or derived artifact")),
        }
    }

    fn require_artifact(&self, id: &OpaqueId) -> Result<(), AuthorityError> {
        self.artifact_bytes(id).map(|_| ())
    }

    // ----- classification -----

    /// Declares an artifact's class in a Project. A receipt-backed row is
    /// owned by the transform path and cannot be re-declared.
    pub fn classify_artifact(
        &mut self,
        project_id: OpaqueId,
        artifact_id: OpaqueId,
        data_class: DataClass,
        expected_revision: Option<u64>,
    ) -> Result<ArtifactClassification, AuthorityError> {
        self.scoped_project(&project_id)?;
        self.require_artifact(&artifact_id)?;
        let existing = self
            .meta
            .get_classification(&project_id, &artifact_id)
            .map_err(meta_err)?;
        let row = match (existing, expected_revision) {
            (None, None) => {
                let id = self.alloc("classification")?;
                let row = ArtifactClassification::declared(
                    self.header(id),
                    project_id,
                    artifact_id.clone(),
                    data_class,
                );
                self.meta.insert_classification(&row).map_err(meta_err)?;
                row
            }
            (None, Some(_)) => return Err(AuthorityError::NotFound),
            (Some(_), None) => {
                return Err(AuthorityError::Conflict {
                    message: "artifact is already classified; pass expected_revision".to_owned(),
                });
            }
            (Some(current), Some(expected)) => {
                if current.basis == ClassificationBasis::DeidReceipt {
                    return Err(AuthorityError::Conflict {
                        message: "a receipt-backed classification cannot be re-declared".to_owned(),
                    });
                }
                if current.revision != expected {
                    return Err(AuthorityError::Conflict {
                        message: format!(
                            "stale revision: expected {expected}, current is {}",
                            current.revision
                        ),
                    });
                }
                let mut next = current;
                next.data_class = data_class;
                next.revision = expected + 1;
                self.meta
                    .update_classification(&next, expected)
                    .map_err(meta_err)?;
                next
            }
        };
        self.audit("privacy.classify", vec![row.header.id.clone(), artifact_id])?;
        Ok(row)
    }

    pub fn effective_classification(
        &self,
        project_id: &OpaqueId,
        artifact_id: &OpaqueId,
    ) -> Result<EffectiveClassification, AuthorityError> {
        self.scoped_project(project_id)?;
        self.require_artifact(artifact_id)?;
        let row = self
            .meta
            .get_classification(project_id, artifact_id)
            .map_err(meta_err)?;
        Ok(EffectiveClassification::of(
            project_id.clone(),
            artifact_id.clone(),
            row,
        ))
    }

    pub fn list_classifications(
        &self,
        project_id: &OpaqueId,
    ) -> Result<Vec<ArtifactClassification>, AuthorityError> {
        self.scoped_project(project_id)?;
        self.meta.list_classifications(project_id).map_err(meta_err)
    }

    // ----- profiles -----

    pub fn create_profile(
        &mut self,
        project_id: OpaqueId,
        name: String,
        target_class: DataClass,
        rules: Vec<ProfileRule>,
        use_model_recognizer: bool,
    ) -> Result<PrivacyPolicyProfile, AuthorityError> {
        self.scoped_project(&project_id)?;
        // Validate completely before allocating an id or writing anything.
        let id = OpaqueId::new("pending");
        PrivacyPolicyProfile::new(
            self.header(id),
            project_id.clone(),
            name.clone(),
            target_class,
            rules.clone(),
            use_model_recognizer,
        )
        .map_err(invalid)?;
        let id = self.alloc("profile")?;
        let profile = PrivacyPolicyProfile::new(
            self.header(id.clone()),
            project_id,
            name,
            target_class,
            rules,
            use_model_recognizer,
        )
        .map_err(invalid)?;
        self.meta
            .insert_privacy_profile(&profile)
            .map_err(meta_err)?;
        self.audit("privacy.profile.create", vec![id])?;
        Ok(profile)
    }

    fn scoped_profile(&self, id: &OpaqueId) -> Result<PrivacyPolicyProfile, AuthorityError> {
        let profile = self.meta.get_privacy_profile(id).map_err(meta_err)?;
        self.in_scope(&profile.header)?;
        Ok(profile)
    }

    pub fn get_profile(&self, id: &OpaqueId) -> Result<PrivacyPolicyProfile, AuthorityError> {
        self.scoped_profile(id)
    }

    pub fn list_profiles(
        &self,
        project_id: &OpaqueId,
    ) -> Result<Vec<PrivacyPolicyProfile>, AuthorityError> {
        self.scoped_project(project_id)?;
        self.meta
            .list_privacy_profiles(project_id)
            .map_err(meta_err)
    }

    pub fn revoke_profile(
        &mut self,
        id: &OpaqueId,
        expected_revision: u64,
    ) -> Result<PrivacyPolicyProfile, AuthorityError> {
        self.scoped_profile(id)?;
        let profile = self
            .meta
            .revoke_privacy_profile(id, expected_revision)
            .map_err(meta_err)?;
        self.audit("privacy.profile.revoke", vec![id.clone()])?;
        Ok(profile)
    }

    // ----- pseudonym maps -----

    /// A fresh `KeyStore` account per map. The random suffix keeps two
    /// vaults that reuse a vault id and map id from sharing (or
    /// overwriting) each other's key.
    fn key_account(&self, map_id: &OpaqueId) -> String {
        let nonce: String = generate_key32()[..8]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        format!(
            "medscale.privacy.pseudonym.{}.{}.{nonce}",
            self.vault_id.as_str(),
            map_id.as_str()
        )
    }

    fn scoped_map(&self, id: &OpaqueId) -> Result<PseudonymMapRef, AuthorityError> {
        let map = self.meta.get_pseudonym_map(id).map_err(meta_err)?;
        self.in_scope(&map.header)?;
        Ok(map)
    }

    /// Creates a map and its key. The key goes only to the `KeyStore`.
    pub fn create_pseudonym_map(
        &mut self,
        project_id: OpaqueId,
    ) -> Result<PseudonymMapRef, AuthorityError> {
        self.scoped_project(&project_id)?;
        let id = self.alloc("pseudonym-map")?;
        let key_account = self.key_account(&id);
        let mut key = generate_key32();
        let stored = self.keys.put(&key_account, &key);
        medscale_keys::zeroize_key(&mut key);
        stored.map_err(|_| AuthorityError::Internal {
            message: "pseudonym key store unavailable".to_owned(),
        })?;
        let map = PseudonymMapRef {
            header: self.header(id.clone()),
            revision: 1,
            project_id,
            key_account,
            entry_count: 0,
            status: PseudonymMapStatus::Active,
        };
        self.meta.insert_pseudonym_map(&map).map_err(meta_err)?;
        self.audit("privacy.pseudonym_map.create", vec![id])?;
        Ok(map)
    }

    pub fn list_pseudonym_maps(
        &self,
        project_id: &OpaqueId,
    ) -> Result<Vec<PseudonymMapRef>, AuthorityError> {
        self.scoped_project(project_id)?;
        self.meta.list_pseudonym_maps(project_id).map_err(meta_err)
    }

    /// Revokes a map and destroys its key. Re-identification then denies.
    pub fn revoke_pseudonym_map(
        &mut self,
        id: &OpaqueId,
        expected_revision: u64,
    ) -> Result<PseudonymMapRef, AuthorityError> {
        let current = self.scoped_map(id)?;
        let map = self
            .meta
            .revoke_pseudonym_map(id, expected_revision)
            .map_err(meta_err)?;
        match self.keys.delete(&current.key_account) {
            Ok(()) | Err(KeyStoreError::NotFound) => {}
            Err(_) => {
                return Err(AuthorityError::Internal {
                    message: "pseudonym key could not be destroyed".to_owned(),
                });
            }
        }
        self.audit("privacy.pseudonym_map.revoke", vec![id.clone()])?;
        Ok(map)
    }

    fn map_key(&self, map: &PseudonymMapRef) -> Option<Vec<u8>> {
        self.keys
            .get(&map.key_account)
            .ok()
            .filter(|k| k.len() == 32)
    }

    fn seal_aad(map_id: &OpaqueId, pseudonym: &str) -> Vec<u8> {
        format!(
            "medscale.privacy.v1\u{1f}{}\u{1f}{pseudonym}",
            map_id.as_str()
        )
        .into_bytes()
    }

    /// Resolves one pseudonym. The audit row is written first; a value is
    /// returned only when the audit write succeeded and the outcome is
    /// `Returned`.
    pub fn reidentify(
        &mut self,
        map_id: &OpaqueId,
        pseudonym: String,
        reason: String,
    ) -> Result<(ReidentificationAudit, Option<String>), AuthorityError> {
        let map = self.scoped_map(map_id)?;
        if !is_pseudonym(&pseudonym) {
            return Err(invalid("not a pseudonym"));
        }
        let requested_by = self.actor()?;
        let (outcome, value) = if map.status == PseudonymMapStatus::Revoked {
            (ReidentificationOutcome::DeniedRevoked, None)
        } else {
            match self
                .meta
                .get_pseudonym_entry(map_id, &pseudonym)
                .map_err(meta_err)?
            {
                None => (ReidentificationOutcome::DeniedUnknownPseudonym, None),
                Some(entry) => match self.map_key(&map) {
                    None => (ReidentificationOutcome::DeniedKeyUnavailable, None),
                    Some(key) => {
                        match unseal(&key, &entry.sealed, &Self::seal_aad(map_id, &pseudonym))
                            .ok()
                            .and_then(|bytes| String::from_utf8(bytes).ok())
                        {
                            Some(value) => (ReidentificationOutcome::Returned, Some(value)),
                            None => (ReidentificationOutcome::DeniedKeyUnavailable, None),
                        }
                    }
                },
            }
        };
        let id = self.alloc("reid-audit")?;
        let audit = ReidentificationAudit {
            header: self.header(id.clone()),
            project_id: map.project_id.clone(),
            map_id: map_id.clone(),
            pseudonym,
            requested_by,
            reason,
            outcome,
        };
        audit.validate().map_err(invalid)?;
        self.meta.insert_reid_audit(&audit).map_err(meta_err)?;
        self.audit("privacy.reidentify", vec![id, map_id.clone()])?;
        Ok((audit, value))
    }

    pub fn list_reid_audit(
        &self,
        map_id: &OpaqueId,
    ) -> Result<Vec<ReidentificationAudit>, AuthorityError> {
        self.scoped_map(map_id)?;
        self.meta.list_reid_audit(map_id).map_err(meta_err)
    }

    // ----- recognition -----

    /// Runs the local model recognizer. Any admission or runtime problem is
    /// reported as `Unavailable`/`Failed`, never as an empty success.
    fn model_recognize(
        &self,
        text: &str,
        request: &ModelRecognizerRequest,
    ) -> (RecognizerResult, Vec<SensitiveSpan>) {
        let identity = RecognizerIdentity {
            recognizer_id: MODEL_RECOGNIZER_ID.to_owned(),
            version: MODEL_RECOGNIZER_VERSION.to_owned(),
            family: RecognizerFamily::LocalModel,
            model_pack_id: Some(request.pack_id.clone()),
        };
        let result = |status, span_count| RecognizerResult {
            recognizer: identity.clone(),
            status,
            span_count,
        };
        let path = std::path::Path::new(&request.local_path);
        let Ok(manifest) = medscale_pack::admit_pack_dir(path) else {
            return (result(RecognizerStatus::Unavailable, 0), Vec::new());
        };
        let admitted = self.packs.get(&manifest.pack_id);
        let admitted_ok = manifest.pack_id == request.pack_id
            && admitted.as_ref().is_some_and(|a| {
                a.content_digest == manifest.content_digest
                    && a.pack_epoch == manifest.pack_epoch
                    && a.version == manifest.version
            });
        if !admitted_ok {
            return (result(RecognizerStatus::Unavailable, 0), Vec::new());
        }
        let Ok(runtime) = medscale_pack::OnnxTokenClassifierRuntime::new(512) else {
            return (result(RecognizerStatus::Unavailable, 0), Vec::new());
        };
        let Ok(prepared) = runtime.prepare(path, &manifest) else {
            return (result(RecognizerStatus::Unavailable, 0), Vec::new());
        };
        // Word windows sized well under the model's fixed sequence length.
        let window = (prepared.fixed_sequence_length() / 4).max(1);
        let mut spans = Vec::new();
        for (chunk_start, chunk) in word_windows(text, window) {
            let Ok(evaluation) = prepared.run(&manifest.pack_id, chunk) else {
                return (result(RecognizerStatus::Failed, 0), Vec::new());
            };
            let predictions = evaluation
                .output
                .proposal_payload
                .get("predictions")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default();
            let lower = chunk.to_lowercase();
            let mut cursor = 0;
            for p in predictions {
                let token = p
                    .get("token")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                let label = p
                    .get("label")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                let piece = token.trim_start_matches("##").to_lowercase();
                if piece.is_empty() || token.starts_with('[') {
                    continue;
                }
                let Some(found) = lower.get(cursor..).and_then(|rest| rest.find(&piece)) else {
                    continue;
                };
                let start = cursor + found;
                let end = start + piece.len();
                cursor = end;
                if label == "O" || !chunk.is_char_boundary(start) || !chunk.is_char_boundary(end) {
                    continue;
                }
                spans.push(SensitiveSpan {
                    kind: SensitiveSpanKind::ModelEntity,
                    start: chunk_start + start,
                    end: chunk_start + end,
                    recognizer_id: MODEL_RECOGNIZER_ID.to_owned(),
                });
            }
        }
        let spans = merge_adjacent(text, spans);
        let count = u32::try_from(spans.len()).unwrap_or(u32::MAX);
        (result(RecognizerStatus::Completed, count), spans)
    }

    /// Runs every applicable recognizer. `transformed` masks spans that fall
    /// inside transform output (placeholders, pseudonyms, generalizations).
    fn recognize(
        &self,
        text: &str,
        declared_fhir: bool,
        model: Option<&ModelRecognizerRequest>,
        use_model: bool,
        transformed: bool,
    ) -> Recognition {
        let mut results = Vec::new();
        let mut spans = pattern_recognize(text);
        results.push(RecognizerResult {
            recognizer: pattern_identity(),
            status: RecognizerStatus::Completed,
            span_count: 0,
        });
        match fhir_recognize(text, declared_fhir) {
            FhirOutcome::NotApplicable => {}
            FhirOutcome::Malformed => results.push(RecognizerResult {
                recognizer: fhir_identity(),
                status: RecognizerStatus::Failed,
                span_count: 0,
            }),
            FhirOutcome::Spans(found) => {
                results.push(RecognizerResult {
                    recognizer: fhir_identity(),
                    status: RecognizerStatus::Completed,
                    span_count: 0,
                });
                spans.extend(found);
            }
        }
        if use_model {
            match model {
                Some(request) => {
                    let (result, found) = self.model_recognize(text, request);
                    results.push(result);
                    spans.extend(found);
                }
                None => results.push(RecognizerResult {
                    recognizer: RecognizerIdentity {
                        recognizer_id: MODEL_RECOGNIZER_ID.to_owned(),
                        version: MODEL_RECOGNIZER_VERSION.to_owned(),
                        family: RecognizerFamily::LocalModel,
                        model_pack_id: Some(OpaqueId::new("not-supplied")),
                    },
                    status: RecognizerStatus::Unavailable,
                    span_count: 0,
                }),
            }
        }
        if transformed {
            let masked = transformed_regions(text);
            spans.retain(|s| {
                !masked.iter().any(|(a, b)| s.start < *b && s.end > *a)
                    && !is_transformed_value(s.kind, &text[s.start..s.end])
            });
        }
        let spans = resolve_overlaps(spans);
        for result in &mut results {
            let id = &result.recognizer.recognizer_id;
            if result.status == RecognizerStatus::Completed {
                result.span_count =
                    u32::try_from(spans.iter().filter(|s| &s.recognizer_id == id).count())
                        .unwrap_or(u32::MAX);
            }
        }
        Recognition { results, spans }
    }

    // ----- transform -----

    /// Transforms a source artifact into a new de-identified derived
    /// artifact under an active profile, and records a `DeidReceipt`.
    pub fn transform(&mut self, request: TransformRequest) -> Result<DeidReceipt, AuthorityError> {
        if !request.synthetic_only {
            return Err(AuthorityError::ExternalGateRequired {
                gate: "REAL_PHI_AUTHORIZATION".to_owned(),
            });
        }
        self.scoped_project(&request.project_id)?;
        let profile = self.scoped_profile(&request.profile_id)?;
        if profile.project_id != request.project_id {
            return Err(invalid("profile does not belong to this project"));
        }
        if profile.status != PrivacyProfileStatus::Active {
            return Err(AuthorityError::Conflict {
                message: "profile is revoked".to_owned(),
            });
        }
        let (map, key) = if profile.uses_pseudonyms() {
            let map_id = request
                .pseudonym_map_id
                .as_ref()
                .ok_or_else(|| invalid("profile pseudonymizes; a pseudonym map is required"))?;
            let map = self.scoped_map(map_id)?;
            if map.project_id != request.project_id {
                return Err(invalid("pseudonym map does not belong to this project"));
            }
            if map.status != PseudonymMapStatus::Active {
                return Err(AuthorityError::Conflict {
                    message: "pseudonym map is revoked".to_owned(),
                });
            }
            let key = self.map_key(&map).ok_or(AuthorityError::Conflict {
                message: "pseudonym map key is unavailable".to_owned(),
            })?;
            (Some(map), Some(key))
        } else {
            (None, None)
        };

        let source = self.artifact_bytes(&request.source_artifact_id)?;
        if source.digest != DigestSha256::of(&source.bytes) {
            return Err(AuthorityError::DigestMismatch);
        }
        if source.bytes.len() > TRANSFORM_INPUT_MAX_BYTES {
            return Err(invalid("source exceeds the transform input bound"));
        }
        let text =
            std::str::from_utf8(&source.bytes).map_err(|_| invalid("source is not UTF-8 text"))?;

        let found = self.recognize(
            text,
            source.declared_fhir,
            request.model.as_ref(),
            profile.use_model_recognizer,
            false,
        );
        if found.spans.len() > MAX_SPANS_PER_TRANSFORM {
            return Err(invalid("too many sensitive spans for one transform"));
        }
        let (output, replacements) = apply_transform(
            text,
            &found.spans,
            |kind| profile.op_for(kind).unwrap_or(TransformOp::Redact),
            |kind, value| key.as_deref().map(|k| pseudonym_for(k, kind, value)),
        )
        .map_err(invalid)?;

        let mut new_entries: Vec<PseudonymEntryRow> = Vec::new();
        if let (Some(map), Some(key)) = (&map, &key) {
            for r in &replacements {
                let Some(pseudonym) = &r.pseudonym else {
                    continue;
                };
                if new_entries.iter().any(|e| &e.pseudonym == pseudonym) {
                    continue;
                }
                let sealed: WrappedBlob = seal(
                    key,
                    r.original.as_bytes(),
                    &Self::seal_aad(&map.header.id, pseudonym),
                )
                .map_err(|_| AuthorityError::Internal {
                    message: "pseudonym sealing failed".to_owned(),
                })?;
                new_entries.push(PseudonymEntryRow {
                    map_id: map.header.id.clone(),
                    pseudonym: pseudonym.clone(),
                    sealed,
                });
            }
        }

        let mut op_counts: Vec<TransformOpCount> = Vec::new();
        for r in &replacements {
            match op_counts
                .iter_mut()
                .find(|c| c.kind == r.kind && c.op == r.op)
            {
                Some(c) => c.count += 1,
                None => op_counts.push(TransformOpCount {
                    kind: r.kind,
                    op: r.op,
                    count: 1,
                }),
            }
        }
        op_counts.sort_by_key(|c| (c.kind, c.op.as_str()));

        let residual_scan = self.recognize(
            &output,
            false,
            request.model.as_ref(),
            profile.use_model_recognizer,
            true,
        );
        let mut residual = ResidualScanResult::from_rescan(residual_scan.results);
        // A recognizer that did not complete on the source (malformed FHIR,
        // missing model) means spans may have gone unseen: never report the
        // output as scanned-clean in that case.
        if found
            .results
            .iter()
            .any(|r| r.status != RecognizerStatus::Completed)
        {
            residual.status = medscale_contracts::privacy_gate::ResidualScanStatus::Unavailable;
        }

        // Write the new derived artifact; the source is never touched.
        let output_bytes = output.into_bytes();
        let output_digest = DigestSha256::of(&output_bytes);
        let output_id = self.store.alloc_id("derived");
        self.store
            .insert(StoredObject::Derived(DerivedSourceArtifact {
                header: ObjectHeader {
                    id: output_id.clone(),
                    schema_version: medscale_contracts::AUTHORITY_SCHEMA_VERSION,
                    realm_id: self.realm.clone(),
                    authority_scope_id: self.scope.clone(),
                },
                source_id: request.source_artifact_id.clone(),
                transform_id: PRIVACY_TRANSFORM_ID.to_owned(),
                transform_version: PRIVACY_TRANSFORM_VERSION.to_owned(),
                loss_class: LossClass::Lossy,
                representation: RepresentationKind::NormalizedText,
                bytes: output_bytes,
                content_digest: output_digest.clone(),
                parent_span_map_ref: None,
            }));

        let mut limitations = vec![
            ReceiptLimitation::AutomatedRecognitionIsIncomplete,
            ReceiptLimitation::SyntheticEvaluationOnly,
            ReceiptLimitation::NonLatinScriptsNotEvaluated,
        ];
        if profile.use_model_recognizer {
            limitations.push(ReceiptLimitation::ModelRecognizerNotQualified);
        }
        if source.declared_fhir {
            limitations.push(ReceiptLimitation::StructuredNarrativePatternOnly);
        }

        let receipt_id = self.alloc("deid-receipt")?;
        let receipt = DeidReceipt {
            header: self.header(receipt_id.clone()),
            revision: 1,
            project_id: request.project_id.clone(),
            source_artifact_id: request.source_artifact_id.clone(),
            source_digest: source.digest.clone(),
            output_artifact_id: output_id.clone(),
            output_digest,
            output_class: profile.target_class,
            profile_id: profile.header.id.clone(),
            profile_revision: profile.revision,
            recognizers: found.results,
            op_counts,
            pseudonym_map_id: map.as_ref().map(|m| m.header.id.clone()),
            residual,
            limitations,
            status: DeidReceiptStatus::Valid,
        };
        receipt.validate().map_err(invalid)?;
        let classification_id = self.alloc("classification")?;
        let output_classification = ArtifactClassification::from_receipt(
            self.header(classification_id),
            request.project_id.clone(),
            output_id.clone(),
            profile.target_class,
            receipt_id.clone(),
        );
        self.meta
            .commit_deid_transform(&DeidTransformCommit {
                receipt: receipt.clone(),
                output_classification,
                new_entries,
            })
            .map_err(meta_err)?;

        // T2: the source is byte-identical after the transform.
        let after = self.artifact_bytes(&request.source_artifact_id)?;
        if after.digest != source.digest || after.bytes != source.bytes {
            return Err(AuthorityError::Corrupt {
                message: "source changed during transform".to_owned(),
            });
        }
        self.audit(
            "privacy.transform",
            vec![receipt_id, request.source_artifact_id, output_id],
        )?;
        Ok(receipt)
    }

    fn scoped_receipt(&self, id: &OpaqueId) -> Result<DeidReceipt, AuthorityError> {
        let receipt = self.meta.get_deid_receipt(id).map_err(meta_err)?;
        self.in_scope(&receipt.header)?;
        Ok(receipt)
    }

    pub fn get_receipt(&self, id: &OpaqueId) -> Result<DeidReceipt, AuthorityError> {
        self.scoped_receipt(id)
    }

    pub fn list_receipts(&self, project_id: &OpaqueId) -> Result<Vec<DeidReceipt>, AuthorityError> {
        self.scoped_project(project_id)?;
        self.meta.list_deid_receipts(project_id).map_err(meta_err)
    }

    pub fn revoke_receipt(
        &mut self,
        id: &OpaqueId,
        expected_revision: u64,
    ) -> Result<DeidReceipt, AuthorityError> {
        self.scoped_receipt(id)?;
        let receipt = self
            .meta
            .revoke_deid_receipt(id, expected_revision)
            .map_err(meta_err)?;
        self.audit("privacy.receipt.revoke", vec![id.clone()])?;
        Ok(receipt)
    }

    // ----- egress -----

    /// The single egress decision every boundary must request before
    /// sending anything. Always persisted; fails closed.
    pub fn evaluate_egress(
        &mut self,
        project_id: OpaqueId,
        artifact_id: OpaqueId,
        boundary: EgressBoundary,
    ) -> Result<EgressDecision, AuthorityError> {
        let effective = self.effective_classification(&project_id, &artifact_id)?;
        let receipt = match effective
            .classification
            .as_ref()
            .and_then(|c| c.deid_receipt_id.clone())
        {
            Some(receipt_id) => {
                let receipt = self.scoped_receipt(&receipt_id)?;
                let profile_status = self
                    .meta
                    .get_privacy_profile(&receipt.profile_id)
                    .map(|p| p.status)
                    .unwrap_or(PrivacyProfileStatus::Revoked);
                let output_digest_matches = receipt.output_artifact_id == artifact_id
                    && self.artifact_bytes(&artifact_id).is_ok_and(|a| {
                        a.digest == receipt.output_digest
                            && DigestSha256::of(&a.bytes) == receipt.output_digest
                    });
                Some((
                    receipt_id,
                    ReceiptEvidence {
                        receipt_status: receipt.status,
                        profile_status,
                        residual: receipt.residual.status,
                        output_digest_matches,
                    },
                ))
            }
            None => None,
        };
        let reason = decide_egress(
            boundary,
            effective.data_class,
            effective.basis,
            receipt.as_ref().map(|(_, e)| e),
        );
        let id = self.alloc("egress-decision")?;
        let decision = EgressDecision {
            header: self.header(id.clone()),
            project_id,
            artifact_id: artifact_id.clone(),
            boundary,
            data_class: effective.data_class,
            basis: effective.basis,
            outcome: reason.outcome(),
            reason,
            deid_receipt_id: receipt.map(|(id, _)| id),
        };
        self.meta
            .insert_egress_decision(&decision)
            .map_err(meta_err)?;
        self.audit("privacy.egress.evaluate", vec![id, artifact_id])?;
        Ok(decision)
    }

    pub fn list_egress_decisions(
        &self,
        project_id: &OpaqueId,
        artifact_id: Option<&OpaqueId>,
    ) -> Result<Vec<EgressDecision>, AuthorityError> {
        self.scoped_project(project_id)?;
        self.meta
            .list_egress_decisions(project_id, artifact_id)
            .map_err(meta_err)
    }
}

/// Splits `text` into windows of at most `max_words` words; returns each
/// window's byte offset and slice.
fn word_windows(text: &str, max_words: usize) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut words = 0;
    let mut start: Option<usize> = None;
    let mut in_word = false;
    for (i, c) in text.char_indices() {
        if c.is_whitespace() {
            if in_word {
                in_word = false;
                if words >= max_words {
                    if let Some(s) = start.take() {
                        out.push((s, &text[s..i]));
                    }
                    words = 0;
                }
            }
        } else if !in_word {
            in_word = true;
            words += 1;
            if start.is_none() {
                start = Some(i);
            }
        }
    }
    if let Some(s) = start {
        let end = text.len();
        if s < end {
            out.push((s, &text[s..end]));
        }
    }
    out
}

/// Merges model spans separated only by whitespace into one entity.
fn merge_adjacent(text: &str, mut spans: Vec<SensitiveSpan>) -> Vec<SensitiveSpan> {
    spans.sort_by_key(|s| s.start);
    let mut out: Vec<SensitiveSpan> = Vec::new();
    for s in spans {
        if let Some(last) = out.last_mut()
            && s.start >= last.end
            && text[last.end..s.start].chars().all(char::is_whitespace)
            && s.start - last.end <= 1
        {
            last.end = s.end;
            continue;
        }
        out.push(s);
    }
    out
}

/// Byte ranges of transform output markers: `[...]` placeholders and
/// pseudonyms. Spans inside them are not residual sensitive values.
fn transformed_regions(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'['
            && let Some(close) = text[i..].find(']')
        {
            let inner = &text[i + 1..i + close];
            let looks_like_marker = inner.bytes().all(|b| {
                b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_' || b == b':' || b == b'-'
            }) && !inner.is_empty();
            if looks_like_marker {
                out.push((i, i + close + 1));
                i += close + 1;
                continue;
            }
        }
        if text[i..].starts_with("PSN-")
            && text.len() >= i + 16
            && text.is_char_boundary(i + 16)
            && is_pseudonym(&text[i..i + 16])
        {
            out.push((i, i + 16));
            i += 16;
            continue;
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_windows_cover_the_text_in_order() {
        let text = "a b c d e f g";
        let windows = word_windows(text, 3);
        let joined: Vec<&str> = windows.iter().map(|(_, w)| *w).collect();
        assert_eq!(joined, vec!["a b c", "d e f", "g"]);
        for (start, w) in windows {
            assert_eq!(&text[start..start + w.len()], w);
        }
    }

    #[test]
    fn transformed_regions_mask_placeholders_and_pseudonyms() {
        let text = "x [REDACTED:PERSON_NAME] y PSN-0123456789ab z [not a marker]";
        let regions = transformed_regions(text);
        assert_eq!(regions.len(), 2);
        assert_eq!(&text[regions[0].0..regions[0].1], "[REDACTED:PERSON_NAME]");
        assert_eq!(&text[regions[1].0..regions[1].1], "PSN-0123456789ab");
    }
}
