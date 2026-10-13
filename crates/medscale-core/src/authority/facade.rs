//! In-process Core Host authority facade (logical IPC API).

use std::sync::{Arc, Mutex};

use medscale_contracts::AUTHORITY_SCHEMA_VERSION;
use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, AuthorityResponse, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::objects::{
    ActionAuditKind, ActionAuditRecord, DerivedSourceArtifact, DigestSha256, EffectState,
    EvaluationRecord, LossClass, MedicalTime, ObjectHeader, OpaqueId, ProducerKind, Projection,
    Proposal, RepresentationKind, TimePrecision,
};

use crate::effects;
use crate::process::{LeaseError, LeaseRegistry, SessionEnforcement, SessionRegistry};

use super::identity::{create_identity_assertion, decide_identity_merge};
use super::ingest_ops;
use super::presentation;
use super::promote::{PromoteError, promote_proposal};
use super::source_ops::create_source_record;
use super::store::{InMemoryAuthorityStore, ScopeError, StoredObject};
use medscale_contracts::network::EgressAllowlistEntry;
use medscale_network::FixtureTransport;
use medscale_storage::{EncryptedVault, SyntheticVault};

/// Spec 103: estimated resident budget for prepared ONNX models (weights size).
/// Least-recently-used, deterministic eviction (`medscale_pack::ResidencyPool`).
const PREPARED_MODEL_BUDGET_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// In-process facade owning lease registry + in-memory store + optional vaults.
#[derive(Debug)]
pub struct CoreFacade {
    leases: LeaseRegistry,
    sessions: SessionRegistry,
    session_enforcement: SessionEnforcement,
    store: Mutex<InMemoryAuthorityStore>,
    vault: Mutex<Option<SyntheticVault>>,
    encrypted: Mutex<Option<EncryptedVault>>,
    allowlist: Mutex<Vec<EgressAllowlistEntry>>,
    packs: Mutex<medscale_pack::PackStore>,
    prepared_models:
        Mutex<medscale_pack::ResidencyPool<Arc<medscale_pack::PreparedOnnxTokenClassifier>>>,
    /// Spec 104: prepared candle safetensors models, with the same budget.
    prepared_candle_models:
        Mutex<medscale_pack::ResidencyPool<Arc<medscale_pack::PreparedCandleTokenClassifier>>>,
    /// Spec 104: prepared GLiNER zero-shot models, with the same budget.
    prepared_gliner_models:
        Mutex<medscale_pack::ResidencyPool<Arc<medscale_pack::PreparedGlinerSpanModel>>>,
    /// Spec 105: prepared text generators, with the same budget.
    prepared_generators:
        Mutex<medscale_pack::ResidencyPool<Arc<medscale_pack::PreparedTextGenerator>>>,
    mesc_epochs: Mutex<medscale_pack::MescEpochStore>,
    /// Spec 079: custody of pseudonym map keys. Never vault metadata.
    privacy_keys: PrivacyKeys,
    /// Spec 080: the only Browse egress path (public-only resolver, no
    /// redirects followed by the transport).
    browse_transport: BrowseTransportBox,
    /// Spec 081: the installed local speech engine, if any (none by
    /// default; hermetic tests and demos install the labelled fixture).
    asr_engine: AsrEngineBox,
    /// Spec 081: capture sessions started by this process.
    live_captures: Mutex<std::collections::HashSet<String>>,
    /// Spec 085: the Compute worker (default: the MedScale worker next to
    /// this executable; never configurable through a request).
    compute_runtime: crate::compute_supervisor::ComputeRuntime,
    /// Spec 085: cancels the compute run in progress, if any.
    compute_cancel: crate::compute_supervisor::CancelSlot,
    /// Spec 086: staging directory and external programs (host
    /// configuration; never set through a request).
    r_host: crate::r_workspace_host::RWorkspaceHost,
    /// Spec 090: institutional transport (product default: unavailable;
    /// never set through a request).
    institutional_transport:
        std::sync::Arc<dyn crate::institutional_transport::InstitutionalTransport>,
}

/// Speech engine holder. `Debug` prints no configuration.
struct AsrEngineBox(Option<Box<dyn super::audio::AsrEngine>>);

impl std::fmt::Debug for AsrEngineBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AsrEngine(..)")
    }
}

/// Browse transport holder. `Debug` prints no configuration.
struct BrowseTransportBox(Box<dyn medscale_network::BrowseTransport>);

impl std::fmt::Debug for BrowseTransportBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BrowseTransport(..)")
    }
}

/// Key custody for Spec 079 pseudonym maps. `Debug` never prints keys.
struct PrivacyKeys(Box<dyn medscale_keys::KeyStore>);

impl std::fmt::Debug for PrivacyKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PrivacyKeys(..)")
    }
}

impl Default for CoreFacade {
    fn default() -> Self {
        Self::new()
    }
}

impl CoreFacade {
    /// Creates a new facade with Spec 024 Strict session enforcement (fail-closed).
    #[must_use]
    pub fn new() -> Self {
        Self {
            leases: LeaseRegistry::new(),
            sessions: SessionRegistry::new(),
            session_enforcement: SessionEnforcement::Strict,
            store: Mutex::new(InMemoryAuthorityStore::default()),
            vault: Mutex::new(None),
            encrypted: Mutex::new(None),
            allowlist: Mutex::new(Vec::new()),
            packs: Mutex::new(medscale_pack::PackStore::default()),
            prepared_models: Mutex::new(medscale_pack::ResidencyPool::new(
                PREPARED_MODEL_BUDGET_BYTES,
            )),
            prepared_candle_models: Mutex::new(medscale_pack::ResidencyPool::new(
                PREPARED_MODEL_BUDGET_BYTES,
            )),
            prepared_gliner_models: Mutex::new(medscale_pack::ResidencyPool::new(
                PREPARED_MODEL_BUDGET_BYTES,
            )),
            prepared_generators: Mutex::new(medscale_pack::ResidencyPool::new(
                PREPARED_MODEL_BUDGET_BYTES,
            )),
            mesc_epochs: Mutex::new(medscale_pack::MescEpochStore::new()),
            // Process-lifetime by default; hosts install a durable store
            // with `set_privacy_key_store`.
            privacy_keys: PrivacyKeys(Box::new(medscale_keys::MemoryKeyStore::new())),
            browse_transport: BrowseTransportBox(Box::new(medscale_network::UreqBrowseTransport)),
            asr_engine: AsrEngineBox(None),
            live_captures: Mutex::new(std::collections::HashSet::new()),
            compute_runtime: crate::compute_supervisor::ComputeRuntime::resolve(),
            compute_cancel: crate::compute_supervisor::CancelSlot::default(),
            r_host: crate::r_workspace_host::RWorkspaceHost::platform_default(),
            institutional_transport: std::sync::Arc::new(
                crate::institutional_transport::UnavailableTransport,
            ),
        }
    }

    /// Engineering/test escape: Spec 018 legacy lease-only mutation without `session_id`.
    ///
    /// Not used by the qualified OS IPC host path. Prefer Strict (`CoreFacade::new()`).
    #[must_use]
    pub fn new_legacy_lease_only_engineering() -> Self {
        Self {
            session_enforcement: SessionEnforcement::LegacyLeaseOnlyEngineering,
            ..Self::new()
        }
    }

    /// In-process session registry (Spec 018 READY_BASE).
    #[must_use]
    pub fn sessions(&self) -> &SessionRegistry {
        &self.sessions
    }

    /// Installs the key store that holds Spec 079 pseudonym map keys (for
    /// example the OS keyring). Keys created before the swap stay in the
    /// previous store and become unavailable, which fails closed.
    pub fn set_privacy_key_store(&mut self, keys: Box<dyn medscale_keys::KeyStore>) {
        self.privacy_keys = PrivacyKeys(keys);
    }

    /// Replaces the Browse transport (hermetic tests and offline demos use
    /// `medscale_network::ScriptedBrowseTransport`). Policy stays in Core.
    pub fn set_browse_transport(&mut self, transport: Box<dyn medscale_network::BrowseTransport>) {
        self.browse_transport = BrowseTransportBox(transport);
    }

    /// Installs a local speech engine for Spec 081 transcription. Route
    /// policy stays in Core; there is never a remote engine.
    pub fn set_asr_engine(&mut self, engine: Box<dyn super::audio::AsrEngine>) {
        self.asr_engine = AsrEngineBox(Some(engine));
    }

    /// Replaces the Spec 085 Compute worker (qualification harnesses name
    /// a fault-injecting worker here). Admission, validation and commit
    /// stay in Core.
    pub fn set_compute_runtime(&mut self, runtime: crate::compute_supervisor::ComputeRuntime) {
        self.compute_runtime = runtime;
    }

    /// Replaces the Spec 090 institutional transport (qualification
    /// harnesses name an in-process store; the product has none).
    pub fn set_institutional_transport(
        &mut self,
        transport: std::sync::Arc<dyn crate::institutional_transport::InstitutionalTransport>,
    ) {
        self.institutional_transport = transport;
    }

    /// Replaces the Spec 086 R Workspace host configuration (the CLI sets
    /// it from its flags and environment; tests name temporary paths).
    pub fn set_r_workspace_host(&mut self, host: crate::r_workspace_host::RWorkspaceHost) {
        self.r_host = host;
    }

    /// Cancels whichever Spec 085 compute run is in progress (each run
    /// installs a fresh handle when it starts). In-process only; usable
    /// from another thread while the run holds the facade.
    #[must_use]
    pub fn compute_canceller(&self) -> crate::compute_supervisor::CancelSlot {
        self.compute_cancel.clone()
    }

    /// Active session enforcement mode (Spec 024).
    #[must_use]
    pub fn session_enforcement(&self) -> SessionEnforcement {
        self.session_enforcement
    }

    fn store(&self) -> std::sync::MutexGuard<'_, InMemoryAuthorityStore> {
        self.store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn vault(&self) -> std::sync::MutexGuard<'_, Option<SyntheticVault>> {
        self.vault
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn encrypted(&self) -> std::sync::MutexGuard<'_, Option<EncryptedVault>> {
        self.encrypted
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Runs one Spec 074 project-graph operation with lease enforcement and
    /// vault-meta access (synthetic or encrypted). Surfaces never touch
    /// storage: this is the only path from request to project rows.
    fn pg<R>(
        &self,
        vault_id: &medscale_contracts::objects::VaultId,
        realm: medscale_contracts::objects::RealmId,
        scope: medscale_contracts::objects::AuthorityScopeId,
        session_id: Option<medscale_contracts::objects::OpaqueId>,
        op: impl FnOnce(super::project_graph::ProjectGraph<'_>) -> Result<R, AuthorityError>,
    ) -> Result<R, AuthorityError> {
        self.require_lease(vault_id)?;
        // Lock order vault -> store matches the existing multi-lock arms and
        // keeps no new lock ordering in the lane.
        let vault_guard = self.vault();
        let enc_guard = self.encrypted();
        let meta = if let Some(enc) = enc_guard.as_ref() {
            &enc.meta
        } else if let Some(vault) = vault_guard.as_ref() {
            &vault.meta
        } else {
            return Err(AuthorityError::VaultRequired);
        };
        let mut store = self.store();
        let packs_guard = self.packs();
        let store_ref: &mut InMemoryAuthorityStore = &mut store;
        let packs_ref: &medscale_pack::PackStore = &packs_guard;
        op(super::project_graph::ProjectGraph {
            store: store_ref,
            meta,
            packs: packs_ref,
            sessions: &self.sessions,
            leases: &self.leases,
            vault_id,
            realm,
            scope,
            session_id,
        })
    }

    /// Runs one Spec 075 data-source operation with lease enforcement and
    /// vault-meta/blob access (synthetic or encrypted). Surfaces never touch
    /// storage, drivers, or transports: this is the only path from request
    /// to data-source rows. Remote acquisition is brokered through the
    /// fixture transport; live hosts stay external-gate refused.
    fn ds<R>(
        &self,
        vault_id: &medscale_contracts::objects::VaultId,
        realm: medscale_contracts::objects::RealmId,
        scope: medscale_contracts::objects::AuthorityScopeId,
        session_id: Option<medscale_contracts::objects::OpaqueId>,
        op: impl FnOnce(super::data_sources::DataSources<'_>) -> Result<R, AuthorityError>,
    ) -> Result<R, AuthorityError> {
        self.require_lease(vault_id)?;
        // Lock order vault -> store matches the existing multi-lock arms and
        // keeps no new lock ordering in the lane.
        let vault_guard = self.vault();
        let enc_guard = self.encrypted();
        let backend = if let Some(enc) = enc_guard.as_ref() {
            super::data_sources::SnapshotBlobBackend::Encrypted(enc)
        } else if let Some(vault) = vault_guard.as_ref() {
            super::data_sources::SnapshotBlobBackend::Synthetic(vault)
        } else {
            return Err(AuthorityError::VaultRequired);
        };
        let mut store = self.store();
        let allow_guard = self.allowlist();
        let transport = FixtureTransport;
        op(super::data_sources::DataSources {
            store: &mut store,
            backend,
            allowlist: &allow_guard,
            transport: &transport,
            sessions: &self.sessions,
            leases: &self.leases,
            vault_id,
            realm,
            scope,
            session_id,
        })
    }

    /// Runs one Spec 084 Hub operation (Hub side) with lease enforcement and
    /// vault-meta access. Device submissions reach collaboration rows only
    /// through the `Collab` view this builds, under the device's session.
    fn hub<R>(
        &self,
        vault_id: &medscale_contracts::objects::VaultId,
        realm: medscale_contracts::objects::RealmId,
        scope: medscale_contracts::objects::AuthorityScopeId,
        session_id: Option<medscale_contracts::objects::OpaqueId>,
        op: impl FnOnce(super::hub::Hub<'_>) -> Result<R, AuthorityError>,
    ) -> Result<R, AuthorityError> {
        self.require_lease(vault_id)?;
        let vault_guard = self.vault();
        let enc_guard = self.encrypted();
        let meta = if let Some(enc) = enc_guard.as_ref() {
            &enc.meta
        } else if let Some(vault) = vault_guard.as_ref() {
            &vault.meta
        } else {
            return Err(AuthorityError::VaultRequired);
        };
        let mut store = self.store();
        let store_ref: &mut InMemoryAuthorityStore = &mut store;
        let packs_guard = self.packs();
        let packs_ref: &medscale_pack::PackStore = &packs_guard;
        op(super::hub::Hub {
            store: store_ref,
            meta,
            packs: packs_ref,
            sessions: &self.sessions,
            leases: &self.leases,
            vault_id,
            realm,
            scope,
            session_id,
        })
    }

    /// Runs one Spec 084 client-side operation over this vault's own links,
    /// outbox and mirror.
    fn hub_client<R>(
        &self,
        vault_id: &medscale_contracts::objects::VaultId,
        realm: medscale_contracts::objects::RealmId,
        scope: medscale_contracts::objects::AuthorityScopeId,
        op: impl FnOnce(super::hub::HubClient<'_>) -> Result<R, AuthorityError>,
    ) -> Result<R, AuthorityError> {
        self.require_lease(vault_id)?;
        let vault_guard = self.vault();
        let enc_guard = self.encrypted();
        let meta = if let Some(enc) = enc_guard.as_ref() {
            &enc.meta
        } else if let Some(vault) = vault_guard.as_ref() {
            &vault.meta
        } else {
            return Err(AuthorityError::VaultRequired);
        };
        op(super::hub::HubClient { meta, realm, scope })
    }

    /// Runs one Spec 076 collaboration operation with lease enforcement and
    /// vault-meta access (synthetic or encrypted). Surfaces never touch
    /// storage: this is the only path from request to collaboration rows.
    fn collab<R>(
        &self,
        vault_id: &medscale_contracts::objects::VaultId,
        realm: medscale_contracts::objects::RealmId,
        scope: medscale_contracts::objects::AuthorityScopeId,
        session_id: Option<medscale_contracts::objects::OpaqueId>,
        op: impl FnOnce(super::collaboration::Collab<'_>) -> Result<R, AuthorityError>,
    ) -> Result<R, AuthorityError> {
        self.require_lease(vault_id)?;
        // Lock order vault -> store matches the existing multi-lock arms and
        // keeps no new lock ordering in the lane.
        let vault_guard = self.vault();
        let enc_guard = self.encrypted();
        let meta = if let Some(enc) = enc_guard.as_ref() {
            &enc.meta
        } else if let Some(vault) = vault_guard.as_ref() {
            &vault.meta
        } else {
            return Err(AuthorityError::VaultRequired);
        };
        let mut store = self.store();
        let store_ref: &mut InMemoryAuthorityStore = &mut store;
        let packs_guard = self.packs();
        let packs_ref: &medscale_pack::PackStore = &packs_guard;
        op(super::collaboration::Collab {
            store: store_ref,
            meta,
            packs: packs_ref,
            sessions: &self.sessions,
            leases: &self.leases,
            vault_id,
            realm,
            scope,
            session_id,
        })
    }

    /// Runs one Spec 077 MedAgent Workbench operation with lease enforcement
    /// and vault-meta/pack-store access (synthetic or encrypted). Surfaces
    /// never touch storage: this is the only path from request to medagent
    /// rows.
    fn medagent<R>(
        &self,
        vault_id: &medscale_contracts::objects::VaultId,
        realm: medscale_contracts::objects::RealmId,
        scope: medscale_contracts::objects::AuthorityScopeId,
        session_id: Option<medscale_contracts::objects::OpaqueId>,
        op: impl FnOnce(super::medagent::MedAgent<'_>) -> Result<R, AuthorityError>,
    ) -> Result<R, AuthorityError> {
        self.require_lease(vault_id)?;
        // Lock order vault -> store matches the existing multi-lock arms and
        // keeps no new lock ordering in the lane.
        let vault_guard = self.vault();
        let enc_guard = self.encrypted();
        let meta = if let Some(enc) = enc_guard.as_ref() {
            &enc.meta
        } else if let Some(vault) = vault_guard.as_ref() {
            &vault.meta
        } else {
            return Err(AuthorityError::VaultRequired);
        };
        let mut store = self.store();
        let store_ref: &mut InMemoryAuthorityStore = &mut store;
        let packs_guard = self.packs();
        let packs_ref: &medscale_pack::PackStore = &packs_guard;
        op(super::medagent::MedAgent {
            store: store_ref,
            meta,
            packs: packs_ref,
            sessions: &self.sessions,
            leases: &self.leases,
            vault_id,
            realm,
            scope,
            session_id,
        })
    }

    /// Runs one Spec 078 Model Fleet operation with lease enforcement and
    /// vault-meta/pack-store access, exactly like `medagent`. Surfaces never
    /// touch storage: this is the only path from request to model_fleet rows.
    fn model_fleet<R>(
        &self,
        vault_id: &medscale_contracts::objects::VaultId,
        realm: medscale_contracts::objects::RealmId,
        scope: medscale_contracts::objects::AuthorityScopeId,
        session_id: Option<medscale_contracts::objects::OpaqueId>,
        op: impl FnOnce(super::model_fleet::ModelFleet<'_>) -> Result<R, AuthorityError>,
    ) -> Result<R, AuthorityError> {
        self.require_lease(vault_id)?;
        let vault_guard = self.vault();
        let enc_guard = self.encrypted();
        let meta = if let Some(enc) = enc_guard.as_ref() {
            &enc.meta
        } else if let Some(vault) = vault_guard.as_ref() {
            &vault.meta
        } else {
            return Err(AuthorityError::VaultRequired);
        };
        let mut store = self.store();
        let store_ref: &mut InMemoryAuthorityStore = &mut store;
        let packs_guard = self.packs();
        let packs_ref: &medscale_pack::PackStore = &packs_guard;
        op(super::model_fleet::ModelFleet {
            store: store_ref,
            meta,
            packs: packs_ref,
            sessions: &self.sessions,
            leases: &self.leases,
            vault_id,
            realm,
            scope,
            session_id,
        })
    }

    /// Runs one Spec 079 Privacy Gate operation with lease enforcement,
    /// vault-meta, pack-store and key-store access. Surfaces never touch
    /// privacy storage or keys: this is the only path.
    fn privacy<R>(
        &self,
        vault_id: &medscale_contracts::objects::VaultId,
        realm: medscale_contracts::objects::RealmId,
        scope: medscale_contracts::objects::AuthorityScopeId,
        session_id: Option<medscale_contracts::objects::OpaqueId>,
        op: impl FnOnce(super::privacy_gate::PrivacyGate<'_>) -> Result<R, AuthorityError>,
    ) -> Result<R, AuthorityError> {
        self.require_lease(vault_id)?;
        let vault_guard = self.vault();
        let enc_guard = self.encrypted();
        let meta = if let Some(enc) = enc_guard.as_ref() {
            &enc.meta
        } else if let Some(vault) = vault_guard.as_ref() {
            &vault.meta
        } else {
            return Err(AuthorityError::VaultRequired);
        };
        let mut store = self.store();
        let store_ref: &mut InMemoryAuthorityStore = &mut store;
        let packs_guard = self.packs();
        let packs_ref: &medscale_pack::PackStore = &packs_guard;
        op(super::privacy_gate::PrivacyGate {
            store: store_ref,
            meta,
            packs: packs_ref,
            keys: self.privacy_keys.0.as_ref(),
            sessions: &self.sessions,
            leases: &self.leases,
            vault_id,
            realm,
            scope,
            session_id,
        })
    }

    /// Runs one Spec 080 Browse operation with lease enforcement and
    /// vault-meta access. Surfaces never reach the transport directly.
    fn browse<R>(
        &self,
        vault_id: &medscale_contracts::objects::VaultId,
        realm: medscale_contracts::objects::RealmId,
        scope: medscale_contracts::objects::AuthorityScopeId,
        session_id: Option<medscale_contracts::objects::OpaqueId>,
        op: impl FnOnce(super::browse::Browse<'_>) -> Result<R, AuthorityError>,
    ) -> Result<R, AuthorityError> {
        self.require_lease(vault_id)?;
        let vault_guard = self.vault();
        let enc_guard = self.encrypted();
        let meta = if let Some(enc) = enc_guard.as_ref() {
            &enc.meta
        } else if let Some(vault) = vault_guard.as_ref() {
            &vault.meta
        } else {
            return Err(AuthorityError::VaultRequired);
        };
        let mut store = self.store();
        let store_ref: &mut InMemoryAuthorityStore = &mut store;
        op(super::browse::Browse {
            store: store_ref,
            meta,
            transport: self.browse_transport.0.as_ref(),
            sessions: &self.sessions,
            leases: &self.leases,
            vault_id,
            realm,
            scope,
            session_id,
        })
    }

    /// Runs one Spec 081 AudioFlow operation with lease enforcement and
    /// vault-meta access. Surfaces never reach storage or engines directly.
    fn audio<R>(
        &self,
        vault_id: &medscale_contracts::objects::VaultId,
        realm: medscale_contracts::objects::RealmId,
        scope: medscale_contracts::objects::AuthorityScopeId,
        session_id: Option<medscale_contracts::objects::OpaqueId>,
        op: impl FnOnce(super::audio::Audio<'_>) -> Result<R, AuthorityError>,
    ) -> Result<R, AuthorityError> {
        self.require_lease(vault_id)?;
        let vault_guard = self.vault();
        let enc_guard = self.encrypted();
        let meta = if let Some(enc) = enc_guard.as_ref() {
            &enc.meta
        } else if let Some(vault) = vault_guard.as_ref() {
            &vault.meta
        } else {
            return Err(AuthorityError::VaultRequired);
        };
        let mut store = self.store();
        let store_ref: &mut InMemoryAuthorityStore = &mut store;
        op(super::audio::Audio {
            store: store_ref,
            meta,
            engine: self.asr_engine.0.as_deref(),
            live_captures: &self.live_captures,
            sessions: &self.sessions,
            leases: &self.leases,
            vault_id,
            realm,
            scope,
            session_id,
        })
    }

    fn allowlist(&self) -> std::sync::MutexGuard<'_, Vec<EgressAllowlistEntry>> {
        self.allowlist
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn packs(&self) -> std::sync::MutexGuard<'_, medscale_pack::PackStore> {
        self.packs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Spec 103: every prepared model in the three runtime pools.
    fn residency_view(
        &self,
        unloaded: Vec<medscale_contracts::objects::DigestSha256>,
    ) -> medscale_contracts::packs::PackResidencyView {
        let mut rows: Vec<(String, String, u64, bool)> = Vec::new();
        for (runtime, entries) in [
            ("tract_onnx", resident_rows(&self.prepared_models())),
            (
                "candle_token_classifier",
                resident_rows(&self.prepared_candle_models()),
            ),
            (
                "candle_gliner",
                resident_rows(&self.prepared_gliner_models()),
            ),
            (
                "candle_text_generation",
                resident_rows(&self.prepared_generators()),
            ),
        ] {
            rows.extend(
                entries
                    .into_iter()
                    .map(|(key, bytes, pinned)| (runtime.to_owned(), key, bytes, pinned)),
            );
        }
        let manifests = self.packs().list();
        let models = rows
            .into_iter()
            .filter_map(|(runtime, key, resident_bytes, pinned)| {
                let content_digest = digest_from_hex(&key)?;
                let pack_ids = manifests
                    .iter()
                    .filter(|m| m.content_digest == content_digest)
                    .map(|m| m.pack_id.clone())
                    .collect();
                Some(medscale_contracts::packs::ResidentModelView {
                    runtime,
                    content_digest,
                    pack_ids,
                    resident_bytes,
                    pinned,
                })
            })
            .collect();
        medscale_contracts::packs::PackResidencyView {
            budget_bytes_per_pool: PREPARED_MODEL_BUDGET_BYTES,
            models,
            unloaded,
        }
    }

    fn prepared_models(
        &self,
    ) -> std::sync::MutexGuard<
        '_,
        medscale_pack::ResidencyPool<Arc<medscale_pack::PreparedOnnxTokenClassifier>>,
    > {
        self.prepared_models
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn prepared_candle_models(
        &self,
    ) -> std::sync::MutexGuard<
        '_,
        medscale_pack::ResidencyPool<Arc<medscale_pack::PreparedCandleTokenClassifier>>,
    > {
        self.prepared_candle_models
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn prepared_gliner_models(
        &self,
    ) -> std::sync::MutexGuard<
        '_,
        medscale_pack::ResidencyPool<Arc<medscale_pack::PreparedGlinerSpanModel>>,
    > {
        self.prepared_gliner_models
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn prepared_generators(
        &self,
    ) -> std::sync::MutexGuard<
        '_,
        medscale_pack::ResidencyPool<Arc<medscale_pack::PreparedTextGenerator>>,
    > {
        self.prepared_generators
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Spec 104: `PacksEvaluateLocal` for a Pack that declares the candle
    /// safetensors runtime. Same admission checks as the ONNX path (done by
    /// the caller); prepared models are kept in their own residency pool,
    /// keyed by content digest, and preparation never holds the pool lock.
    fn evaluate_candle_pack(
        &self,
        path: &std::path::Path,
        manifest: &medscale_contracts::packs::PackManifestV0,
        input: &str,
        realm_id: medscale_contracts::objects::RealmId,
        authority_scope_id: medscale_contracts::objects::AuthorityScopeId,
    ) -> Result<ResponseBody, AuthorityError> {
        let key = manifest.content_digest.to_hex();
        let cached = self.prepared_candle_models().get(&key).cloned();
        let (prepared, prepared_cache_hit) = if let Some(prepared) = cached {
            (prepared, true)
        } else {
            let candidate = Arc::new(
                medscale_pack::prepare_candle_token_classifier(path, manifest).map_err(|err| {
                    AuthorityError::InvalidArgument {
                        message: format!("pack runtime preparation failed: {err}"),
                    }
                })?,
            );
            let resident_bytes = manifest
                .artifacts
                .iter()
                .filter(|a| a.kind == medscale_contracts::packs::PackArtifactKind::SafetensorsModel)
                .filter_map(|a| std::fs::metadata(path.join(&a.relative_path)).ok())
                .map(|m| m.len())
                .sum::<u64>()
                .max(1);
            let mut cache = self.prepared_candle_models();
            if let Some(existing) = cache.get(&key).cloned() {
                (existing, true)
            } else {
                let _ = cache.load(&key, resident_bytes, Arc::clone(&candidate));
                (candidate, false)
            }
        };
        let output = prepared
            .run_document(
                &manifest.pack_id,
                input,
                medscale_pack::CandleRunOptions::default(),
            )
            .map_err(|err| AuthorityError::InvalidArgument {
                message: format!("pack runtime evaluation failed: {err}"),
            })?;
        let mut store = self.store();
        let audit_id = store.alloc_id("audit");
        store.insert(StoredObject::Audit(ActionAuditRecord {
            header: ObjectHeader {
                id: audit_id.clone(),
                schema_version: AUTHORITY_SCHEMA_VERSION,
                realm_id,
                authority_scope_id,
            },
            kind: ActionAuditKind::Audit,
            actor: OpaqueId::new("pack-runtime"),
            action: "packs.evaluate_local".to_owned(),
            target_refs: vec![manifest.pack_id.clone()],
            effect_state: None,
            payload_digest: None,
            detail: Some(serde_json::json!({
                "runtime": medscale_pack::CANDLE_TOKEN_CLASSIFIER_RUNTIME_ID,
                "input_bytes": input.len(),
                "evidence_only": true,
                "synthetic_only": true,
                "prepared_cache_hit": prepared_cache_hit,
            })),
        }));
        Ok(ResponseBody::PackEvaluation {
            result: medscale_contracts::packs::PackEvaluationResult {
                pack_id: output.pack_id,
                content_digest: manifest.content_digest.clone(),
                runtime_id: medscale_pack::CANDLE_TOKEN_CLASSIFIER_RUNTIME_ID.to_owned(),
                prepared_cache_hit,
                evidence_only: output.evidence_only,
                proposal_payload: output.proposal_payload,
                provenance: prepared.provenance().clone(),
                audit_id,
            },
        })
    }

    /// Spec 104: `PacksEvaluateLocal` for a candle Pack holding a GLiNER
    /// zero-shot export, run for the requested entity types at OpenMed's
    /// default threshold with flat (non-overlapping) decoding. Same admission
    /// checks as the other paths (done by the caller); prepared models have
    /// their own residency pool and preparation never holds the pool lock.
    fn evaluate_gliner_pack(
        &self,
        path: &std::path::Path,
        manifest: &medscale_contracts::packs::PackManifestV0,
        input: &str,
        labels: &[String],
        realm_id: medscale_contracts::objects::RealmId,
        authority_scope_id: medscale_contracts::objects::AuthorityScopeId,
    ) -> Result<ResponseBody, AuthorityError> {
        let key = manifest.content_digest.to_hex();
        let cached = self.prepared_gliner_models().get(&key).cloned();
        let (prepared, prepared_cache_hit) = if let Some(prepared) = cached {
            (prepared, true)
        } else {
            let candidate = Arc::new(
                medscale_pack::prepare_gliner_span_model(path, manifest).map_err(|err| {
                    AuthorityError::InvalidArgument {
                        message: format!("pack runtime preparation failed: {err}"),
                    }
                })?,
            );
            let resident_bytes = manifest
                .artifacts
                .iter()
                .filter(|a| a.kind == medscale_contracts::packs::PackArtifactKind::SafetensorsModel)
                .filter_map(|a| std::fs::metadata(path.join(&a.relative_path)).ok())
                .map(|m| m.len())
                .sum::<u64>()
                .max(1);
            let mut cache = self.prepared_gliner_models();
            if let Some(existing) = cache.get(&key).cloned() {
                (existing, true)
            } else {
                let _ = cache.load(&key, resident_bytes, Arc::clone(&candidate));
                (candidate, false)
            }
        };
        let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
        let output = prepared
            .run(
                &manifest.pack_id,
                input,
                &labels,
                medscale_pack::GLINER_DEFAULT_THRESHOLD,
            )
            .map_err(|err| AuthorityError::InvalidArgument {
                message: format!("pack runtime evaluation failed: {err}"),
            })?;
        let mut store = self.store();
        let audit_id = store.alloc_id("audit");
        store.insert(StoredObject::Audit(ActionAuditRecord {
            header: ObjectHeader {
                id: audit_id.clone(),
                schema_version: AUTHORITY_SCHEMA_VERSION,
                realm_id,
                authority_scope_id,
            },
            kind: ActionAuditKind::Audit,
            actor: OpaqueId::new("pack-runtime"),
            action: "packs.evaluate_local".to_owned(),
            target_refs: vec![manifest.pack_id.clone()],
            effect_state: None,
            payload_digest: None,
            detail: Some(serde_json::json!({
                "runtime": medscale_pack::GLINER_ZERO_SHOT_RUNTIME_ID,
                "input_bytes": input.len(),
                "label_count": labels.len(),
                "evidence_only": true,
                "synthetic_only": true,
                "prepared_cache_hit": prepared_cache_hit,
            })),
        }));
        Ok(ResponseBody::PackEvaluation {
            result: medscale_contracts::packs::PackEvaluationResult {
                pack_id: output.pack_id,
                content_digest: manifest.content_digest.clone(),
                runtime_id: medscale_pack::GLINER_ZERO_SHOT_RUNTIME_ID.to_owned(),
                prepared_cache_hit,
                evidence_only: output.evidence_only,
                proposal_payload: output.proposal_payload,
                provenance: prepared.provenance().clone(),
                audit_id,
            },
        })
    }

    /// Spec 105: `PacksGenerateLocal` with an admitted generation Pack, after
    /// the caller's admission checks. Prepared generators have their own
    /// residency pool; the prompt text is never written to the audit.
    fn generate_with_pack(
        &self,
        path: &std::path::Path,
        manifest: &medscale_contracts::packs::PackManifestV0,
        input: &str,
        max_new_tokens: usize,
        realm_id: medscale_contracts::objects::RealmId,
        authority_scope_id: medscale_contracts::objects::AuthorityScopeId,
    ) -> Result<ResponseBody, AuthorityError> {
        let key = manifest.content_digest.to_hex();
        let cached = self.prepared_generators().get(&key).cloned();
        let (prepared, prepared_cache_hit) = if let Some(prepared) = cached {
            (prepared, true)
        } else {
            let candidate = Arc::new(
                medscale_pack::prepare_text_generator(path, manifest).map_err(|err| {
                    AuthorityError::InvalidArgument {
                        message: format!("pack runtime preparation failed: {err}"),
                    }
                })?,
            );
            let resident_bytes = manifest
                .artifacts
                .iter()
                .filter(|a| a.kind == medscale_contracts::packs::PackArtifactKind::SafetensorsModel)
                .filter_map(|a| std::fs::metadata(path.join(&a.relative_path)).ok())
                .map(|m| m.len())
                .sum::<u64>()
                .max(1);
            let mut cache = self.prepared_generators();
            if let Some(existing) = cache.get(&key).cloned() {
                (existing, true)
            } else {
                let _ = cache.load(&key, resident_bytes, Arc::clone(&candidate));
                (candidate, false)
            }
        };
        let output = prepared
            .generate(&manifest.pack_id, input, max_new_tokens)
            .map_err(|err| AuthorityError::InvalidArgument {
                message: format!("pack generation failed: {err}"),
            })?;
        let mut store = self.store();
        let audit_id = store.alloc_id("audit");
        store.insert(StoredObject::Audit(ActionAuditRecord {
            header: ObjectHeader {
                id: audit_id.clone(),
                schema_version: AUTHORITY_SCHEMA_VERSION,
                realm_id,
                authority_scope_id,
            },
            kind: ActionAuditKind::Audit,
            actor: OpaqueId::new("pack-runtime"),
            action: "packs.generate_local".to_owned(),
            target_refs: vec![manifest.pack_id.clone()],
            effect_state: None,
            payload_digest: None,
            detail: Some(serde_json::json!({
                "runtime": medscale_pack::CANDLE_TEXT_GENERATION_RUNTIME_ID,
                "input_bytes": input.len(),
                "max_new_tokens": max_new_tokens,
                "evidence_only": true,
                "synthetic_only": true,
                "prepared_cache_hit": prepared_cache_hit,
            })),
        }));
        Ok(ResponseBody::PackEvaluation {
            result: medscale_contracts::packs::PackEvaluationResult {
                pack_id: output.pack_id,
                content_digest: manifest.content_digest.clone(),
                runtime_id: medscale_pack::CANDLE_TEXT_GENERATION_RUNTIME_ID.to_owned(),
                prepared_cache_hit,
                evidence_only: output.evidence_only,
                proposal_payload: output.proposal_payload,
                provenance: prepared.provenance().clone(),
                audit_id,
            },
        })
    }

    fn mesc_epochs(&self) -> std::sync::MutexGuard<'_, medscale_pack::MescEpochStore> {
        self.mesc_epochs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn require_lease(
        &self,
        vault_id: &medscale_contracts::objects::VaultId,
    ) -> Result<(), AuthorityError> {
        if self.leases.holder(vault_id).is_none() {
            Err(AuthorityError::LeaseRequired)
        } else {
            Ok(())
        }
    }

    /// Dispatches a versioned authority request.
    pub fn dispatch(&self, req: AuthorityRequest) -> AuthorityResponse {
        let request_id = req.request_id.clone();
        // Spec 074 high-frequency graph ops provably leave the in-memory
        // snapshot untouched (see RequestBody::preserves_memory_snapshot);
        // skipping the rewrite is output-identical and keeps per-op cost
        // constant instead of O(full history).
        let skip_snapshot = req.body.preserves_memory_snapshot();
        let result = self.dispatch_inner(req).and_then(|body| {
            if skip_snapshot {
                Ok(body)
            } else {
                self.persist_open_vault()?;
                Ok(body)
            }
        });
        AuthorityResponse {
            schema_version: AUTHORITY_SCHEMA_VERSION,
            request_id,
            result,
        }
    }

    fn persist_open_vault(&self) -> Result<(), AuthorityError> {
        let store = self.store();
        if let Some(enc) = self.encrypted().as_ref() {
            return super::durable::sync_store_to_encrypted(enc, &store);
        }
        let vault = self.vault();
        let Some(vault) = vault.as_ref() else {
            return Ok(());
        };
        super::durable::sync_store_to_vault(vault, &store)
    }

    fn dispatch_inner(&self, req: AuthorityRequest) -> Result<ResponseBody, AuthorityError> {
        if req.schema_version != AUTHORITY_SCHEMA_VERSION {
            return Err(AuthorityError::InvalidArgument {
                message: "unsupported schema_version".to_owned(),
            });
        }
        if !capability_matches(&req.capability, &req.body) {
            return Err(AuthorityError::Unauthorized);
        }

        // Spec 024: Strict requires session_id for mutating capabilities.
        // LegacyLeaseOnlyEngineering preserves Spec 018 opt-in validation-only-when-present.
        match self.session_enforcement {
            SessionEnforcement::Strict => {
                if req.capability.requires_client_session() {
                    let Some(session_id) = &req.session_id else {
                        return Err(AuthorityError::SessionRequired);
                    };
                    self.sessions
                        .validate(session_id, &req.vault_id, req.capability)?;
                } else if let Some(session_id) = &req.session_id {
                    self.sessions
                        .validate(session_id, &req.vault_id, req.capability)?;
                }
            }
            SessionEnforcement::LegacyLeaseOnlyEngineering => {
                if let Some(session_id) = &req.session_id {
                    self.sessions
                        .validate(session_id, &req.vault_id, req.capability)?;
                }
            }
        }

        match req.body {
            RequestBody::AcquireLease {
                client_id,
                holder_id_hint,
            } => {
                let holder = self
                    .leases
                    .acquire(&req.vault_id, client_id, holder_id_hint)
                    .map_err(lease_err)?;
                Ok(ResponseBody::Lease {
                    holder_id: holder,
                    vault_id: req.vault_id,
                })
            }
            RequestBody::ReleaseLease { holder_id } => {
                self.leases
                    .release(&req.vault_id, &holder_id)
                    .map_err(lease_err)?;
                Ok(ResponseBody::Released)
            }
            RequestBody::OpenSession {
                holder_id,
                granted,
                ttl_ticks,
            } => {
                let Some(lease_holder) = self.leases.holder(&req.vault_id) else {
                    return Err(AuthorityError::LeaseRequired);
                };
                if lease_holder != holder_id {
                    return Err(AuthorityError::NotHolder);
                }
                let (session_id, expires_at_tick) =
                    self.sessions
                        .open(&req.vault_id, holder_id, granted, ttl_ticks);
                Ok(ResponseBody::Session {
                    session_id,
                    expires_at_tick,
                })
            }
            RequestBody::RevokeSession { session_id } => {
                self.sessions.revoke(&session_id);
                Ok(ResponseBody::Released)
            }
            RequestBody::Ping => Ok(ResponseBody::Pong {
                schema_version: AUTHORITY_SCHEMA_VERSION,
            }),
            RequestBody::CreateSourceRecord { media_type, bytes } => {
                let mut store = self.store();
                let record = create_source_record(
                    &mut store,
                    req.realm_id,
                    req.authority_scope_id,
                    media_type,
                    bytes,
                );
                Ok(ResponseBody::Created {
                    object_id: record.header.id,
                })
            }
            RequestBody::CreateDerivedArtifact {
                source_id,
                transform_id,
                transform_version,
                bytes,
            } => {
                let mut store = self.store();
                store
                    .get_source(&source_id, &req.realm_id, &req.authority_scope_id)
                    .map_err(scope_err)?;
                let id = store.alloc_id("derived");
                let digest = DigestSha256::of(&bytes);
                let artifact = DerivedSourceArtifact {
                    header: ObjectHeader {
                        id: id.clone(),
                        schema_version: AUTHORITY_SCHEMA_VERSION,
                        realm_id: req.realm_id,
                        authority_scope_id: req.authority_scope_id,
                    },
                    source_id,
                    transform_id,
                    transform_version,
                    loss_class: LossClass::Unknown,
                    representation: RepresentationKind::NormalizedText,
                    bytes,
                    content_digest: digest,
                    parent_span_map_ref: None,
                };
                store.insert(StoredObject::Derived(artifact));
                Ok(ResponseBody::Created { object_id: id })
            }
            RequestBody::CreateProposal {
                subject_ref,
                claim_kind,
                payload,
                evidence_refs,
            } => {
                let mut store = self.store();
                let id = store.alloc_id("proposal");
                let proposal = Proposal {
                    header: ObjectHeader {
                        id: id.clone(),
                        schema_version: AUTHORITY_SCHEMA_VERSION,
                        realm_id: req.realm_id,
                        authority_scope_id: req.authority_scope_id,
                    },
                    subject_ref,
                    claim_kind,
                    payload,
                    confidence: None,
                    evidence_refs,
                    producer: ProducerKind::Rule,
                };
                store.insert(StoredObject::Proposal(proposal));
                Ok(ResponseBody::Created { object_id: id })
            }
            RequestBody::PromoteProposal {
                proposal_id,
                authorized_by,
                subject_ref,
            } => {
                let mut store = self.store();
                let (assertion, audit) = promote_proposal(
                    &mut store,
                    &proposal_id,
                    authorized_by,
                    subject_ref,
                    &req.realm_id,
                    &req.authority_scope_id,
                )
                .map_err(promote_err)?;
                Ok(ResponseBody::Promoted {
                    assertion_id: assertion.header.id,
                    audit_id: audit.header.id,
                })
            }
            RequestBody::AmendAssertion {
                prior_assertion_id,
                authorized_by,
                payload,
                effective_time,
                kind,
                rationale,
            } => {
                let mut store = self.store();
                let (assertion, amendment, audit) = super::amend::amend_assertion(
                    &mut store,
                    super::amend::AmendAssertionInput {
                        prior_assertion_id: &prior_assertion_id,
                        authorized_by,
                        payload,
                        effective_time,
                        kind,
                        rationale,
                        realm_id: &req.realm_id,
                        scope_id: &req.authority_scope_id,
                    },
                )
                .map_err(amend_err)?;
                Ok(ResponseBody::Amended {
                    assertion_id: assertion.header.id,
                    amendment_id: amendment.header.id,
                    audit_id: audit.header.id,
                })
            }
            RequestBody::CreateIdentityAssertion {
                subject_id,
                identifier_system,
                identifier_value,
            } => {
                let mut store = self.store();
                let record = create_identity_assertion(
                    &mut store,
                    req.realm_id,
                    req.authority_scope_id,
                    subject_id,
                    identifier_system,
                    identifier_value,
                );
                Ok(ResponseBody::Created {
                    object_id: record.header.id,
                })
            }
            RequestBody::DecideIdentityMerge {
                surviving_subject_id,
                merged_subject_ids,
                authorized_by,
                rationale,
            } => {
                let mut store = self.store();
                let record = decide_identity_merge(
                    &mut store,
                    req.realm_id,
                    req.authority_scope_id,
                    surviving_subject_id,
                    merged_subject_ids,
                    authorized_by,
                    rationale,
                )
                .map_err(|message| AuthorityError::InvalidArgument {
                    message: message.to_owned(),
                })?;
                Ok(ResponseBody::Created {
                    object_id: record.header.id,
                })
            }
            RequestBody::AppendAudit {
                actor,
                action,
                target_refs,
                detail,
            } => {
                let mut store = self.store();
                let id = store.alloc_id("audit");
                let audit = ActionAuditRecord {
                    header: ObjectHeader {
                        id: id.clone(),
                        schema_version: AUTHORITY_SCHEMA_VERSION,
                        realm_id: req.realm_id,
                        authority_scope_id: req.authority_scope_id,
                    },
                    kind: ActionAuditKind::Audit,
                    actor,
                    action,
                    target_refs,
                    effect_state: None,
                    payload_digest: None,
                    detail,
                };
                store.insert(StoredObject::Audit(audit));
                Ok(ResponseBody::Created { object_id: id })
            }
            RequestBody::TransitionEffect {
                action_id,
                to,
                reconcile_token,
            } => {
                let mut store = self.store();
                let audit = store
                    .get_audit_mut(&action_id)
                    .ok_or(AuthorityError::NotFound)?;
                if audit.kind != ActionAuditKind::ExternalActionIntent {
                    // Promote plain audit into effect-bearing intent if needed.
                    audit.kind = ActionAuditKind::ExternalActionIntent;
                    if audit.effect_state.is_none() {
                        audit.effect_state = Some(EffectState::Pending);
                    }
                }
                let from = audit.effect_state.unwrap_or(EffectState::Pending);
                // READY_BASE: cannot leave Pending toward Sent without bound payload digest.
                if from == EffectState::Pending
                    && to == EffectState::Sent
                    && audit.payload_digest.is_none()
                {
                    return Err(AuthorityError::InvalidArgument {
                        message: "payload_digest required before Pending→Sent".to_owned(),
                    });
                }
                let next = effects::transition(from, to, reconcile_token.as_deref()).ok_or(
                    if from == EffectState::Unknown
                        && reconcile_token.as_deref().unwrap_or("").is_empty()
                    {
                        AuthorityError::UnknownRequiresReconcile
                    } else {
                        AuthorityError::IllegalTransition
                    },
                )?;
                audit.effect_state = Some(next);
                Ok(ResponseBody::Effect {
                    action_id,
                    state: next,
                })
            }
            RequestBody::ReadObject { object_id } => {
                let store = self.store();
                let obj = store
                    .get_scoped(&object_id, &req.realm_id, &req.authority_scope_id)
                    .map_err(scope_err)?;
                Ok(ResponseBody::Object {
                    value: obj.to_json(),
                })
            }
            RequestBody::OpenSyntheticVault { vault_root } => {
                self.require_lease(&req.vault_id)?;
                if self.encrypted().is_some() {
                    return Err(AuthorityError::InvalidArgument {
                        message: "encrypted vault already open; close before opening synthetic"
                            .to_owned(),
                    });
                }
                let mut slot = self.vault();
                let mut store = self.store();
                ingest_ops::open_vault(&mut slot, &mut store, req.vault_id.as_str(), &vault_root)
            }
            RequestBody::CloseVault => {
                self.require_lease(&req.vault_id)?;
                let mut slot = self.vault();
                let store = self.store();
                ingest_ops::close_vault(&mut slot, &store)
            }
            RequestBody::IngestFhirSynthetic {
                media_type,
                bytes,
                fhir_version_hint,
                attach_validator_fixture_id,
            } => {
                self.require_lease(&req.vault_id)?;
                let vault_guard = self.vault();
                let vault = vault_guard.as_ref().ok_or(AuthorityError::VaultRequired)?;
                let mut store = self.store();
                ingest_ops::ingest_fhir(
                    vault,
                    &mut store,
                    req.realm_id,
                    req.authority_scope_id,
                    media_type,
                    bytes,
                    fhir_version_hint,
                    attach_validator_fixture_id,
                )
            }
            RequestBody::AttachValidatorEvidence {
                source_id,
                evaluator,
                outcome,
                issue_codes,
            } => {
                self.require_lease(&req.vault_id)?;
                let mut store = self.store();
                store
                    .get_scoped(&source_id, &req.realm_id, &req.authority_scope_id)
                    .map_err(scope_err)?;
                let eval_id = store.alloc_id("eval");
                store.insert(StoredObject::Evaluation(EvaluationRecord {
                    header: ObjectHeader {
                        id: eval_id.clone(),
                        schema_version: AUTHORITY_SCHEMA_VERSION,
                        realm_id: req.realm_id,
                        authority_scope_id: req.authority_scope_id,
                    },
                    target_refs: vec![source_id],
                    evaluator,
                    result: serde_json::json!({ "outcome": outcome, "issues": issue_codes, "evidence_only": true }),
                    evidence_only: true,
                }));
                Ok(ResponseBody::Created { object_id: eval_id })
            }
            RequestBody::RebuildProjection { kind, built_from } => {
                self.require_lease(&req.vault_id)?;
                let mut store = self.store();
                let vault_guard = self.vault();
                let body = if presentation::is_presentation_kind(&kind) {
                    // Prefer subject from first assertion in built_from, else treat first id as subject.
                    let subject_ref =
                        built_from
                            .first()
                            .cloned()
                            .ok_or(AuthorityError::InvalidArgument {
                                message: "built_from required".to_owned(),
                            })?;
                    // If first id is an assertion, use its subject; else use as subject_ref directly.
                    let subject = match store.get_scoped(
                        &subject_ref,
                        &req.realm_id,
                        &req.authority_scope_id,
                    ) {
                        Ok(StoredObject::Assertion(a)) => a.subject_ref.clone(),
                        _ => subject_ref.clone(),
                    };
                    let lookup = |d: &medscale_contracts::objects::DigestSha256| {
                        blob_lookup(vault_guard.as_ref(), d)
                    };
                    match kind.as_str() {
                        medscale_contracts::presentation::KIND_SUBJECT_TIMELINE_V1 => {
                            presentation::body_timeline(&presentation::build_timeline(
                                &store, &subject, &lookup,
                            ))
                        }
                        medscale_contracts::presentation::KIND_SUBJECT_BRIEF_V1 => {
                            presentation::body_brief(&presentation::build_brief(
                                &store, &subject, &lookup,
                            ))
                        }
                        medscale_contracts::presentation::KIND_SUBJECT_COVERAGE_V1 => {
                            presentation::body_coverage(&presentation::build_coverage(
                                &store, &subject, &lookup,
                            ))
                        }
                        _ => serde_json::json!({}),
                    }
                } else {
                    serde_json::json!({})
                };
                let id = store.alloc_id("proj");
                let proj = Projection {
                    header: ObjectHeader {
                        id: id.clone(),
                        schema_version: AUTHORITY_SCHEMA_VERSION,
                        realm_id: req.realm_id,
                        authority_scope_id: req.authority_scope_id,
                    },
                    projection_kind: kind,
                    built_from,
                    built_at: MedicalTime::new(
                        "1970-01-01T00:00:00Z",
                        TimePrecision::Instant,
                        false,
                    ),
                    body,
                    authoritative: false,
                };
                store.insert(StoredObject::Projection(proj));
                Ok(ResponseBody::Created { object_id: id })
            }
            RequestBody::ReadCanonicalVisibility { source_id } => {
                self.require_lease(&req.vault_id)?;
                let vault_guard = self.vault();
                let vault = vault_guard.as_ref().ok_or(AuthorityError::VaultRequired)?;
                let meta = vault
                    .meta
                    .get_source(&source_id)
                    .map_err(|_| AuthorityError::NotFound)?;
                if meta.realm_id != req.realm_id
                    || meta.authority_scope_id != req.authority_scope_id
                {
                    return Err(AuthorityError::WrongScope);
                }
                let blob_ok = vault.blobs.verify(&meta.digest, meta.byte_length).is_ok();
                Ok(ResponseBody::Visibility {
                    source_id,
                    visible: meta.visible && blob_ok,
                    content_digest: meta.digest,
                    byte_length: meta.byte_length,
                    blob_ok,
                })
            }
            RequestBody::VerifyBlob { digest } => {
                self.require_lease(&req.vault_id)?;
                let vault_guard = self.vault();
                let vault = vault_guard.as_ref().ok_or(AuthorityError::VaultRequired)?;
                let state = vault.blobs.state(&digest);
                let bytes = vault
                    .blobs
                    .get_blob(&digest)
                    .map_err(|_| AuthorityError::NotFound)?;
                Ok(ResponseBody::BlobVerified {
                    digest,
                    byte_length: bytes.len() as u64,
                    state,
                })
            }
            RequestBody::BackupVault { destination } => {
                self.require_lease(&req.vault_id)?;
                let vault_guard = self.vault();
                let vault = vault_guard.as_ref().ok_or(AuthorityError::VaultRequired)?;
                ingest_ops::backup(vault, &destination)
            }
            RequestBody::RestoreVault {
                source,
                destination,
            } => {
                self.require_lease(&req.vault_id)?;
                ingest_ops::restore(&source, &destination)
            }
            RequestBody::RunBlobGc => {
                self.require_lease(&req.vault_id)?;
                let vault_guard = self.vault();
                let vault = vault_guard.as_ref().ok_or(AuthorityError::VaultRequired)?;
                ingest_ops::gc(vault)
            }
            RequestBody::GetTimeline { subject_ref } => {
                self.require_lease(&req.vault_id)?;
                let store = self.store();
                let vault_guard = self.vault();
                let body = presentation::build_timeline(&store, &subject_ref, &|d| {
                    blob_lookup(vault_guard.as_ref(), d)
                });
                Ok(ResponseBody::Timeline {
                    body,
                    projection_id: None,
                })
            }
            RequestBody::GetBrief { subject_ref } => {
                self.require_lease(&req.vault_id)?;
                let store = self.store();
                let vault_guard = self.vault();
                let body = presentation::build_brief(&store, &subject_ref, &|d| {
                    blob_lookup(vault_guard.as_ref(), d)
                });
                Ok(ResponseBody::Brief {
                    body,
                    projection_id: None,
                })
            }
            RequestBody::GetCoverage { subject_ref } => {
                self.require_lease(&req.vault_id)?;
                let store = self.store();
                let vault_guard = self.vault();
                let body = presentation::build_coverage(&store, &subject_ref, &|d| {
                    blob_lookup(vault_guard.as_ref(), d)
                });
                Ok(ResponseBody::Coverage {
                    body,
                    projection_id: None,
                })
            }
            RequestBody::DrillDownPresentation {
                subject_ref,
                field_key,
                assertion_id,
            } => {
                self.require_lease(&req.vault_id)?;
                let store = self.store();
                let vault_guard = self.vault();
                let result = presentation::drill_down(
                    &store,
                    &subject_ref,
                    &field_key,
                    assertion_id.as_ref(),
                    &|d| blob_lookup(vault_guard.as_ref(), d),
                )
                .map_err(|e| match e {
                    presentation::DrillDownError::NotFound => AuthorityError::NotFound,
                    presentation::DrillDownError::UnhealthyEvidence => {
                        AuthorityError::InvalidArgument {
                            message: "unhealthy_evidence".to_owned(),
                        }
                    }
                })?;
                Ok(ResponseBody::DrillDown { result })
            }
            RequestBody::CreateEncryptedVault {
                vault_root,
                passphrase,
            } => {
                self.require_lease(&req.vault_id)?;
                if self.vault().is_some() {
                    return Err(AuthorityError::InvalidArgument {
                        message: "synthetic vault already open; close before creating encrypted"
                            .to_owned(),
                    });
                }
                let mut slot = self.encrypted();
                if slot.is_some() {
                    return Err(AuthorityError::InvalidArgument {
                        message: "encrypted vault already open".to_owned(),
                    });
                }
                let holder = self
                    .leases
                    .holder(&req.vault_id)
                    .ok_or(AuthorityError::LeaseRequired)?;
                let (vault, codes) = EncryptedVault::create(
                    req.vault_id.as_str(),
                    std::path::Path::new(&vault_root),
                    &passphrase,
                    holder.as_str(),
                    None,
                )
                .map_err(enc_err)?;
                // Spec 035: empty authority snapshot for new encrypted vault.
                {
                    let store = self.store();
                    super::durable::sync_store_to_encrypted(&vault, &store)?;
                }
                *slot = Some(vault);
                Ok(ResponseBody::EncryptedVaultReady {
                    vault_root,
                    recovery_codes: Some(codes.codes),
                })
            }
            RequestBody::OpenEncryptedVault {
                vault_root,
                passphrase,
                recovery_code,
            } => {
                self.require_lease(&req.vault_id)?;
                if self.vault().is_some() {
                    return Err(AuthorityError::InvalidArgument {
                        message: "synthetic vault already open; close before opening encrypted"
                            .to_owned(),
                    });
                }
                let mut slot = self.encrypted();
                if slot.is_some() {
                    return Err(AuthorityError::InvalidArgument {
                        message: "encrypted vault already open".to_owned(),
                    });
                }
                let holder = self
                    .leases
                    .holder(&req.vault_id)
                    .ok_or(AuthorityError::LeaseRequired)?;
                let path = std::path::Path::new(&vault_root);
                let vault = if let Some(code) = recovery_code {
                    EncryptedVault::open_with_recovery(path, &code, holder.as_str())
                } else if let Some(pw) = passphrase {
                    EncryptedVault::open_with_passphrase(path, &pw, holder.as_str())
                } else {
                    return Err(AuthorityError::MissingKeyMaterial);
                }
                .map_err(enc_err)?;
                {
                    let mut store = self.store();
                    super::durable::load_store_from_encrypted(&vault, &mut store)?;
                }
                *slot = Some(vault);
                Ok(ResponseBody::EncryptedVaultReady {
                    vault_root,
                    recovery_codes: None,
                })
            }
            RequestBody::CloseEncryptedVault => {
                self.require_lease(&req.vault_id)?;
                {
                    let enc = self.encrypted();
                    if let Some(vault) = enc.as_ref() {
                        let store = self.store();
                        super::durable::sync_store_to_encrypted(vault, &store)?;
                    }
                }
                let mut slot = self.encrypted();
                if let Some(vault) = slot.take() {
                    vault.close().map_err(enc_err)?;
                }
                Ok(ResponseBody::VaultClosed)
            }
            RequestBody::SetEgressAllowlist { entries } => {
                self.require_lease(&req.vault_id)?;
                let count = entries.len() as u32;
                *self.allowlist() = entries;
                Ok(ResponseBody::AllowlistSet { entries: count })
            }
            RequestBody::NetworkBrokerInvoke { request } => {
                self.require_lease(&req.vault_id)?;
                let allowlist = self.allowlist().clone();
                let outcome =
                    medscale_network::broker_invoke(&allowlist, &request, &FixtureTransport);
                let mut store = self.store();
                let audit_id = store.alloc_id("audit");
                let action = match outcome.decision {
                    medscale_contracts::network::BrokerDecision::Allow => "network_broker.attempt",
                    medscale_contracts::network::BrokerDecision::Deny => "network_broker.deny",
                };
                store.insert(StoredObject::Audit(ActionAuditRecord {
                    header: ObjectHeader {
                        id: audit_id.clone(),
                        schema_version: AUTHORITY_SCHEMA_VERSION,
                        realm_id: req.realm_id.clone(),
                        authority_scope_id: req.authority_scope_id.clone(),
                    },
                    kind: ActionAuditKind::Audit,
                    actor: OpaqueId::new("network-broker"),
                    action: action.to_owned(),
                    target_refs: vec![],
                    effect_state: None,
                    payload_digest: request.body_digest.clone(),
                    detail: Some(serde_json::json!({
                        "host": request.destination_host,
                        "path": request.destination_path,
                        "purpose": request.purpose,
                        "data_class": request.data_class,
                        "decision": outcome.decision,
                        "reason": outcome.reason,
                        "transport_sent": outcome.transport_sent,
                    })),
                }));
                let evaluation_id = if matches!(
                    request.purpose,
                    medscale_contracts::network::EgressPurpose::ProfileOracleFixture
                        | medscale_contracts::network::EgressPurpose::ConformanceEvidenceAttach
                        | medscale_contracts::network::EgressPurpose::IntegrityCheck
                ) {
                    let eval_id = store.alloc_id("eval");
                    store.insert(StoredObject::Evaluation(EvaluationRecord {
                        header: ObjectHeader {
                            id: eval_id.clone(),
                            schema_version: AUTHORITY_SCHEMA_VERSION,
                            realm_id: req.realm_id,
                            authority_scope_id: req.authority_scope_id,
                        },
                        target_refs: vec![audit_id.clone()],
                        evaluator: "medscale.network.fixture_oracle.v1".to_owned(),
                        result: serde_json::json!({
                            "evidence_only": true,
                            "reason": outcome.reason,
                            "fixture": outcome.fixture_body,
                        }),
                        evidence_only: true,
                    }));
                    Some(eval_id)
                } else {
                    None
                };
                Ok(ResponseBody::NetworkBroker {
                    result: medscale_contracts::network::NetworkBrokerResult {
                        decision: outcome.decision,
                        reason: outcome.reason,
                        audit_id,
                        evaluation_id,
                        transport_sent: outcome.transport_sent,
                        fixture_body: outcome.fixture_body,
                    },
                })
            }
            RequestBody::PacksInstallLocal { local_path } => {
                self.require_lease(&req.vault_id)?;
                let mut store = self.store();
                let audit_id = store.alloc_id("audit");
                match medscale_pack::admit_pack_dir(std::path::Path::new(&local_path)) {
                    Ok(manifest) => {
                        let pack_id = manifest.pack_id.clone();
                        if let Err(err) = self.packs().admit(manifest) {
                            let reason = err.reason();
                            store.insert(StoredObject::Audit(ActionAuditRecord {
                                header: ObjectHeader {
                                    id: audit_id.clone(),
                                    schema_version: AUTHORITY_SCHEMA_VERSION,
                                    realm_id: req.realm_id,
                                    authority_scope_id: req.authority_scope_id,
                                },
                                kind: ActionAuditKind::Audit,
                                actor: OpaqueId::new("pack-admit"),
                                action: "packs.install_local.deny".to_owned(),
                                target_refs: vec![],
                                effect_state: None,
                                payload_digest: None,
                                detail: Some(serde_json::json!({
                                    "local_path": local_path,
                                    "reason": reason,
                                })),
                            }));
                            return Ok(ResponseBody::PackAdmit {
                                result: medscale_contracts::packs::PackAdmitResult {
                                    admitted: false,
                                    reason,
                                    pack_id: None,
                                    audit_id,
                                },
                            });
                        }
                        store.insert(StoredObject::Audit(ActionAuditRecord {
                            header: ObjectHeader {
                                id: audit_id.clone(),
                                schema_version: AUTHORITY_SCHEMA_VERSION,
                                realm_id: req.realm_id,
                                authority_scope_id: req.authority_scope_id,
                            },
                            kind: ActionAuditKind::Audit,
                            actor: OpaqueId::new("pack-admit"),
                            action: "packs.install_local".to_owned(),
                            target_refs: vec![pack_id.clone()],
                            effect_state: None,
                            payload_digest: None,
                            detail: Some(serde_json::json!({
                                "local_path": local_path,
                                "admitted": true
                            })),
                        }));
                        Ok(ResponseBody::PackAdmit {
                            result: medscale_contracts::packs::PackAdmitResult {
                                admitted: true,
                                reason: medscale_contracts::packs::PackAdmitReason::Ok,
                                pack_id: Some(pack_id),
                                audit_id,
                            },
                        })
                    }
                    Err(err) => {
                        let reason = err.reason();
                        store.insert(StoredObject::Audit(ActionAuditRecord {
                            header: ObjectHeader {
                                id: audit_id.clone(),
                                schema_version: AUTHORITY_SCHEMA_VERSION,
                                realm_id: req.realm_id,
                                authority_scope_id: req.authority_scope_id,
                            },
                            kind: ActionAuditKind::Audit,
                            actor: OpaqueId::new("pack-admit"),
                            action: "packs.install_local.deny".to_owned(),
                            target_refs: vec![],
                            effect_state: None,
                            payload_digest: None,
                            detail: Some(serde_json::json!({
                                "local_path": local_path,
                                "reason": reason,
                            })),
                        }));
                        Ok(ResponseBody::PackAdmit {
                            result: medscale_contracts::packs::PackAdmitResult {
                                admitted: false,
                                reason,
                                pack_id: None,
                                audit_id,
                            },
                        })
                    }
                }
            }
            RequestBody::PacksList => {
                self.require_lease(&req.vault_id)?;
                Ok(ResponseBody::PackList {
                    packs: self.packs().list(),
                })
            }
            RequestBody::PacksPromote { pack_id, to } => {
                self.require_lease(&req.vault_id)?;
                match self.packs().promote(&pack_id, to) {
                    Ok(m) => Ok(ResponseBody::PackPromoted {
                        pack_id: m.pack_id,
                        state: m.promotion_state,
                    }),
                    Err(msg) => Err(AuthorityError::InvalidArgument {
                        message: msg.to_owned(),
                    }),
                }
            }
            RequestBody::PacksGenerateLocal { request } => {
                self.require_lease(&req.vault_id)?;
                if !request.synthetic_only {
                    return Err(AuthorityError::ExternalGateRequired {
                        gate: "REAL_PHI_MODEL_RUNTIME".to_owned(),
                    });
                }
                let path = std::path::Path::new(&request.local_path);
                let manifest = medscale_pack::admit_pack_dir(path).map_err(|err| {
                    AuthorityError::InvalidArgument {
                        message: format!("pack generation admission failed: {err}"),
                    }
                })?;
                if manifest.pack_id != request.pack_id {
                    return Err(AuthorityError::DigestMismatch);
                }
                let stored = self
                    .packs()
                    .get(&request.pack_id)
                    .ok_or(AuthorityError::NotFound)?;
                if stored.content_digest != manifest.content_digest
                    || stored.pack_epoch != manifest.pack_epoch
                    || stored.version != manifest.version
                {
                    return Err(AuthorityError::DigestMismatch);
                }
                let max_new_tokens = usize::try_from(request.max_new_tokens).map_err(|_| {
                    AuthorityError::InvalidArgument {
                        message: "max_new_tokens is not representable".to_owned(),
                    }
                })?;
                self.generate_with_pack(
                    path,
                    &manifest,
                    &request.prompt,
                    max_new_tokens,
                    req.realm_id.clone(),
                    req.authority_scope_id.clone(),
                )
            }
            RequestBody::PacksResidency => Ok(ResponseBody::PackResidency {
                view: Box::new(self.residency_view(Vec::new())),
            }),
            RequestBody::PacksUnloadLocal { pack_id } => {
                self.require_lease(&req.vault_id)?;
                let target = match pack_id.as_ref() {
                    Some(id) => Some(
                        self.packs()
                            .get(id)
                            .ok_or(AuthorityError::NotFound)?
                            .content_digest
                            .to_hex(),
                    ),
                    None => None,
                };
                let mut unloaded = std::collections::BTreeSet::new();
                {
                    let mut onnx = self.prepared_models();
                    unloaded.extend(unload_from(&mut onnx, target.as_deref()));
                }
                {
                    let mut candle = self.prepared_candle_models();
                    unloaded.extend(unload_from(&mut candle, target.as_deref()));
                }
                {
                    let mut gliner = self.prepared_gliner_models();
                    unloaded.extend(unload_from(&mut gliner, target.as_deref()));
                }
                {
                    let mut generators = self.prepared_generators();
                    unloaded.extend(unload_from(&mut generators, target.as_deref()));
                }
                let unloaded: Vec<medscale_contracts::objects::DigestSha256> = unloaded
                    .into_iter()
                    .filter_map(|hex| digest_from_hex(&hex))
                    .collect();
                let mut store = self.store();
                let audit_id = store.alloc_id("audit");
                store.insert(StoredObject::Audit(ActionAuditRecord {
                    header: ObjectHeader {
                        id: audit_id,
                        schema_version: AUTHORITY_SCHEMA_VERSION,
                        realm_id: req.realm_id.clone(),
                        authority_scope_id: req.authority_scope_id.clone(),
                    },
                    kind: ActionAuditKind::Audit,
                    actor: OpaqueId::new("pack-runtime"),
                    action: "packs.unload_local".to_owned(),
                    target_refs: pack_id.iter().cloned().collect(),
                    effect_state: None,
                    payload_digest: None,
                    detail: Some(serde_json::json!({ "unloaded": unloaded.len() })),
                }));
                drop(store);
                Ok(ResponseBody::PackResidency {
                    view: Box::new(self.residency_view(unloaded)),
                })
            }
            RequestBody::PacksEvaluateLocal { request } => {
                self.require_lease(&req.vault_id)?;
                if !request.synthetic_only {
                    return Err(AuthorityError::ExternalGateRequired {
                        gate: "REAL_PHI_MODEL_RUNTIME".to_owned(),
                    });
                }
                let path = std::path::Path::new(&request.local_path);
                let manifest = medscale_pack::admit_pack_dir(path).map_err(|err| {
                    AuthorityError::InvalidArgument {
                        message: format!("pack evaluation admission failed: {err}"),
                    }
                })?;
                if manifest.pack_id != request.pack_id {
                    return Err(AuthorityError::DigestMismatch);
                }
                let stored = self
                    .packs()
                    .get(&request.pack_id)
                    .ok_or(AuthorityError::NotFound)?;
                if stored.content_digest != manifest.content_digest
                    || stored.pack_epoch != manifest.pack_epoch
                    || stored.version != manifest.version
                {
                    return Err(AuthorityError::DigestMismatch);
                }
                let max_tokens = usize::try_from(request.max_tokens).map_err(|_| {
                    AuthorityError::InvalidArgument {
                        message: "max_tokens is not representable".to_owned(),
                    }
                })?;
                let candle = medscale_pack::pack_runtime_kind(&manifest)
                    == Some(medscale_pack::PackRuntimeKind::CandleSafetensors);
                // A resident prepared model already answers which head the Pack
                // holds; only a cold Pack re-reads its verified config.json.
                // Each pool lock is released before any file is read.
                let resident_key = manifest.content_digest.to_hex();
                let resident_gliner = self.prepared_gliner_models().get(&resident_key).is_some();
                let resident_candle = self.prepared_candle_models().get(&resident_key).is_some();
                let zero_shot = candle
                    && (resident_gliner
                        || (!resident_candle
                            && medscale_pack::pack_is_gliner_span(path, &manifest).map_err(
                                |err| AuthorityError::InvalidArgument {
                                    message: format!("pack runtime preparation failed: {err}"),
                                },
                            )?));
                match (zero_shot, &request.zero_shot_labels) {
                    (true, Some(labels)) => {
                        return self.evaluate_gliner_pack(
                            path,
                            &manifest,
                            &request.input,
                            labels,
                            req.realm_id,
                            req.authority_scope_id,
                        );
                    }
                    (true, None) => {
                        return Err(AuthorityError::InvalidArgument {
                            message: "zero-shot Pack requires zero_shot_labels".to_owned(),
                        });
                    }
                    (false, Some(_)) => {
                        return Err(AuthorityError::InvalidArgument {
                            message: "zero_shot_labels apply only to zero-shot Packs".to_owned(),
                        });
                    }
                    (false, None) => {}
                }
                if candle {
                    return self.evaluate_candle_pack(
                        path,
                        &manifest,
                        &request.input,
                        req.realm_id,
                        req.authority_scope_id,
                    );
                }
                let runtime =
                    medscale_pack::OnnxTokenClassifierRuntime::new(max_tokens).map_err(|err| {
                        AuthorityError::InvalidArgument {
                            message: format!("pack runtime configuration denied: {err}"),
                        }
                    })?;
                let key = manifest.content_digest.to_hex();
                let cached = self
                    .prepared_models()
                    .get(&key)
                    .filter(|prepared| prepared.matches_runtime_contract(&manifest))
                    .cloned();
                let (prepared, prepared_cache_hit) = if let Some(prepared) = cached {
                    if prepared.fixed_sequence_length() > max_tokens {
                        return Err(AuthorityError::InvalidArgument {
                            message:
                                "pack runtime token ceiling is below the prepared model requirement"
                                    .to_owned(),
                        });
                    }
                    (prepared, true)
                } else {
                    // Preparation can be expensive. Never hold the shared cache mutex while
                    // parsing/optimizing model bytes. Re-check after preparation to handle a
                    // concurrent request that may have populated the same digest.
                    let candidate = Arc::new(runtime.prepare(path, &manifest).map_err(|err| {
                        AuthorityError::InvalidArgument {
                            message: format!("pack runtime preparation failed: {err}"),
                        }
                    })?);
                    let resident_bytes = manifest
                        .artifacts
                        .iter()
                        .filter(|a| {
                            a.kind == medscale_contracts::packs::PackArtifactKind::OnnxModel
                        })
                        .filter_map(|a| std::fs::metadata(path.join(&a.relative_path)).ok())
                        .map(|m| m.len())
                        .sum::<u64>()
                        .max(1);
                    let mut cache = self.prepared_models();
                    if let Some(existing) = cache
                        .get(&key)
                        .filter(|prepared| prepared.matches_runtime_contract(&manifest))
                        .cloned()
                    {
                        (existing, true)
                    } else {
                        // A model larger than the whole budget still runs; it is
                        // simply not kept resident.
                        let _ = cache.load(&key, resident_bytes, Arc::clone(&candidate));
                        (candidate, false)
                    }
                };
                let evaluation =
                    prepared
                        .run(&manifest.pack_id, &request.input)
                        .map_err(|err| AuthorityError::InvalidArgument {
                            message: format!("pack runtime evaluation failed: {err}"),
                        })?;
                let mut store = self.store();
                let audit_id = store.alloc_id("audit");
                store.insert(StoredObject::Audit(ActionAuditRecord {
                    header: ObjectHeader {
                        id: audit_id.clone(),
                        schema_version: AUTHORITY_SCHEMA_VERSION,
                        realm_id: req.realm_id,
                        authority_scope_id: req.authority_scope_id,
                    },
                    kind: ActionAuditKind::Audit,
                    actor: OpaqueId::new("pack-runtime"),
                    action: "packs.evaluate_local".to_owned(),
                    target_refs: vec![manifest.pack_id.clone()],
                    effect_state: None,
                    payload_digest: None,
                    detail: Some(serde_json::json!({
                        "runtime": medscale_pack::ONNX_TOKEN_CLASSIFIER_RUNTIME_ID,
                        "input_bytes": request.input.len(),
                        "evidence_only": true,
                        "synthetic_only": true,
                        "prepared_cache_hit": prepared_cache_hit,
                    })),
                }));
                Ok(ResponseBody::PackEvaluation {
                    result: medscale_contracts::packs::PackEvaluationResult {
                        pack_id: evaluation.output.pack_id,
                        content_digest: manifest.content_digest,
                        runtime_id: medscale_pack::ONNX_TOKEN_CLASSIFIER_RUNTIME_ID.to_owned(),
                        prepared_cache_hit,
                        evidence_only: evaluation.output.evidence_only,
                        proposal_payload: evaluation.output.proposal_payload,
                        provenance: evaluation.provenance,
                        audit_id,
                    },
                })
            }
            RequestBody::DocumentIntake { request } => {
                self.require_lease(&req.vault_id)?;
                let mut store = self.store();
                Ok(ResponseBody::DocumentIntake {
                    result: super::document_ops::intake(
                        &mut store,
                        req.realm_id,
                        req.authority_scope_id,
                        request,
                    )?,
                })
            }
            RequestBody::OcrStub { request } => {
                self.require_lease(&req.vault_id)?;
                let mut store = self.store();
                Ok(ResponseBody::MediaStub {
                    result: super::document_ops::ocr_stub(
                        &mut store,
                        req.realm_id,
                        req.authority_scope_id,
                        request,
                    )?,
                })
            }
            RequestBody::AsrStub { request } => {
                self.require_lease(&req.vault_id)?;
                let mut store = self.store();
                Ok(ResponseBody::MediaStub {
                    result: super::document_ops::asr_stub(
                        &mut store,
                        req.realm_id,
                        req.authority_scope_id,
                        request,
                    )?,
                })
            }
            RequestBody::RetrieveLexical { request } => {
                self.require_lease(&req.vault_id)?;
                let mut store = self.store();
                Ok(ResponseBody::LexicalRetrieve {
                    result: super::retrieval::retrieve_lexical(
                        &mut store,
                        req.realm_id,
                        req.authority_scope_id,
                        request,
                    )?,
                })
            }
            RequestBody::CreateExternalActionIntent { request } => {
                let mut store = self.store();
                let id = store.alloc_id("intent");
                let audit = ActionAuditRecord {
                    header: ObjectHeader {
                        id: id.clone(),
                        schema_version: AUTHORITY_SCHEMA_VERSION,
                        realm_id: req.realm_id,
                        authority_scope_id: req.authority_scope_id,
                    },
                    kind: ActionAuditKind::ExternalActionIntent,
                    actor: request.actor,
                    action: request.action,
                    target_refs: request.target_refs,
                    effect_state: Some(EffectState::Pending),
                    payload_digest: Some(request.payload_digest),
                    detail: None,
                };
                store.insert(StoredObject::Audit(audit));
                Ok(ResponseBody::Created { object_id: id })
            }
            RequestBody::ListOutbox => {
                let store = self.store();
                let entries = store
                    .list_external_action_intents(&req.realm_id, &req.authority_scope_id)
                    .into_iter()
                    .filter_map(|a| {
                        let digest = a.payload_digest.clone()?;
                        Some(medscale_contracts::actions::OutboxEntry {
                            action_id: a.header.id.clone(),
                            action: a.action.clone(),
                            effect_state: a.effect_state.unwrap_or(EffectState::Pending),
                            payload_digest: digest,
                        })
                    })
                    .collect();
                Ok(ResponseBody::Outbox { entries })
            }
            RequestBody::NphiesInvoke { request: _ } => Err(AuthorityError::ExternalGateRequired {
                gate: "SPEC_014_WORKFLOW_EVIDENCE".to_owned(),
            }),
            RequestBody::OnlinePackAcquire { request } => {
                if !request.broker_required {
                    return Err(AuthorityError::InvalidArgument {
                        message: "online pack acquire requires broker_required=true".to_owned(),
                    });
                }
                Err(AuthorityError::ExternalGateRequired {
                    gate: "HF_ONLINE_PACK_DISTRIBUTION".to_owned(),
                })
            }
            RequestBody::MescArtifactAdmit { request } => {
                if !request.pack_path_required {
                    return Err(AuthorityError::InvalidArgument {
                        message: "MESC admit requires pack_path_required=true (ARTIFACT_IMPORT)"
                            .to_owned(),
                    });
                }
                Err(AuthorityError::ExternalGateRequired {
                    gate: "MESC_RELEASED_ARTIFACT".to_owned(),
                })
            }
            RequestBody::MescArtifactVerify { request } => {
                if request.release_dir.trim().is_empty() {
                    return Err(AuthorityError::InvalidArgument {
                        message: "MESC verify requires non-empty release_dir".to_owned(),
                    });
                }
                let path = std::path::Path::new(&request.release_dir);
                match medscale_pack::verify_mesc_release_dir(path) {
                    Ok(report) => {
                        let producer = report
                            .producer_id
                            .clone()
                            .unwrap_or_else(|| "unknown".to_owned());
                        let epoch = report.epoch.unwrap_or(0);
                        if let Err(e) = self.mesc_epochs().admit_epoch(&producer, epoch) {
                            return Ok(ResponseBody::MescVerify {
                                report: e.report().clone(),
                            });
                        }
                        debug_assert!(!report.product_admit_authorized);
                        Ok(ResponseBody::MescVerify { report })
                    }
                    Err(e) => Ok(ResponseBody::MescVerify {
                        report: e.report().clone(),
                    }),
                }
            }
            RequestBody::GetFhirSupportMatrix => Ok(ResponseBody::FhirSupportMatrix {
                matrix: medscale_contracts::fhir::FhirSupportMatrix::trusted_v1_ready_base(),
            }),
            RequestBody::ExportFhirLossAware { resource } => {
                Ok(ResponseBody::FhirLossAwareExport {
                    export: medscale_contracts::fhir::loss_aware_export(&resource),
                })
            }
            RequestBody::RejectProposal {
                proposal_id,
                actor,
                rationale,
            } => {
                self.require_lease(&req.vault_id)?;
                let mut store = self.store();
                let _proposal = store
                    .get_proposal(&proposal_id, &req.realm_id, &req.authority_scope_id)
                    .map_err(scope_err)?;
                let audit_id = store.alloc_id("audit");
                let audit = ActionAuditRecord {
                    header: ObjectHeader {
                        id: audit_id.clone(),
                        schema_version: AUTHORITY_SCHEMA_VERSION,
                        realm_id: req.realm_id,
                        authority_scope_id: req.authority_scope_id,
                    },
                    kind: ActionAuditKind::Audit,
                    actor,
                    action: medscale_contracts::workflow::PROPOSAL_REJECT_ACTION.to_owned(),
                    target_refs: vec![proposal_id.clone()],
                    effect_state: None,
                    payload_digest: None,
                    detail: Some(serde_json::json!({ "rationale": rationale })),
                };
                store.insert(StoredObject::Audit(audit));
                Ok(ResponseBody::Rejected {
                    proposal_id,
                    audit_id,
                })
            }
            RequestBody::AppendDisclosure {
                purpose,
                scope,
                subject_ref,
                artifact_refs,
                export_digest,
                note,
            } => {
                self.require_lease(&req.vault_id)?;
                let mut store = self.store();
                let disclosure_id = store.alloc_id("disclosure");
                let record = medscale_contracts::workflow::DisclosureRecord {
                    disclosure_id: disclosure_id.clone(),
                    purpose: purpose.clone(),
                    scope: scope.clone(),
                    subject_ref: subject_ref.clone(),
                    artifact_refs: artifact_refs.clone(),
                    export_digest: export_digest.clone(),
                    synthetic_only: true,
                    release_ready_claimed: false,
                    note: note.clone(),
                };
                let audit = ActionAuditRecord {
                    header: ObjectHeader {
                        id: disclosure_id,
                        schema_version: AUTHORITY_SCHEMA_VERSION,
                        realm_id: req.realm_id,
                        authority_scope_id: req.authority_scope_id,
                    },
                    kind: ActionAuditKind::Audit,
                    actor: OpaqueId::new("workflow-operator"),
                    action: medscale_contracts::workflow::DISCLOSURE_APPEND_ACTION.to_owned(),
                    target_refs: artifact_refs,
                    effect_state: None,
                    payload_digest: export_digest,
                    detail: Some(serde_json::to_value(&record).unwrap_or(serde_json::Value::Null)),
                };
                store.insert(StoredObject::Audit(audit));
                Ok(ResponseBody::DisclosureAppended { record })
            }
            RequestBody::ListDisclosures => {
                self.require_lease(&req.vault_id)?;
                let store = self.store();
                let records = store.list_disclosures(&req.realm_id, &req.authority_scope_id);
                Ok(ResponseBody::DisclosureList { records })
            }
            // Spec 074 project graph: every arm runs through `pg` (lease +
            // session gate already enforced above + meta + audit). No arm
            // touches storage except through `ProjectGraph` methods.
            RequestBody::ProjectCreate { name, description } => {
                let project = self.pg(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut pg| pg.create_project(name, description),
                )?;
                Ok(ResponseBody::Project { project })
            }
            RequestBody::ProjectGet { project_id } => {
                let project = self.pg(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |pg| pg.scoped_project_pub(&project_id),
                )?;
                Ok(ResponseBody::Project { project })
            }
            RequestBody::ProjectList {
                status,
                limit,
                cursor,
            } => {
                let (projects, next_cursor) = self.pg(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |pg| pg.list_projects(status, limit, cursor),
                )?;
                Ok(ResponseBody::ProjectList {
                    projects,
                    next_cursor,
                })
            }
            RequestBody::ProjectUpdate {
                project_id,
                expected_revision,
                name,
                description,
            } => {
                let project = self.pg(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut pg| pg.update_project(&project_id, expected_revision, name, description),
                )?;
                Ok(ResponseBody::Project { project })
            }
            RequestBody::ProjectArchive {
                project_id,
                expected_revision,
            } => {
                let project = self.pg(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut pg| pg.archive_project(&project_id, expected_revision),
                )?;
                Ok(ResponseBody::Project { project })
            }
            RequestBody::ProjectRestore {
                project_id,
                expected_revision,
            } => {
                let project = self.pg(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut pg| pg.restore_project(&project_id, expected_revision),
                )?;
                Ok(ResponseBody::Project { project })
            }
            RequestBody::ExperimentCreate {
                project_id,
                name,
                description,
            } => {
                let experiment = self.pg(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut pg| pg.create_experiment(&project_id, name, description),
                )?;
                Ok(ResponseBody::Experiment { experiment })
            }
            RequestBody::ExperimentGet { experiment_id } => {
                let experiment = self.pg(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |pg| pg.scoped_experiment_pub(&experiment_id),
                )?;
                Ok(ResponseBody::Experiment { experiment })
            }
            RequestBody::ExperimentList {
                project_id,
                limit,
                cursor,
            } => {
                let (experiments, next_cursor) = self.pg(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |pg| pg.list_experiments(&project_id, limit, cursor),
                )?;
                Ok(ResponseBody::ExperimentList {
                    experiments,
                    next_cursor,
                })
            }
            RequestBody::ExperimentUpdate {
                experiment_id,
                expected_revision,
                name,
                description,
            } => {
                let experiment = self.pg(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut pg| {
                        pg.update_experiment(&experiment_id, expected_revision, name, description)
                    },
                )?;
                Ok(ResponseBody::Experiment { experiment })
            }
            RequestBody::ExperimentArchive {
                experiment_id,
                expected_revision,
            } => {
                let experiment = self.pg(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut pg| pg.archive_experiment(&experiment_id, expected_revision),
                )?;
                Ok(ResponseBody::Experiment { experiment })
            }
            RequestBody::ProjectAttach {
                project_id,
                experiment_id,
                artifact,
            } => {
                let reference = self.pg(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut pg| pg.attach(&project_id, experiment_id, artifact),
                )?;
                Ok(ResponseBody::ProjectRef { reference })
            }
            RequestBody::ProjectDetach {
                ref_id,
                expected_revision,
            } => {
                let reference = self.pg(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut pg| pg.detach(&ref_id, expected_revision),
                )?;
                Ok(ResponseBody::ProjectRef { reference })
            }
            RequestBody::ProjectListRefs {
                project_id,
                experiment_id,
                active_only,
                limit,
                cursor,
            } => {
                let (refs, next_cursor) = self.pg(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |pg| pg.list_refs(&project_id, experiment_id, active_only, limit, cursor),
                )?;
                Ok(ResponseBody::ProjectRefList { refs, next_cursor })
            }
            RequestBody::GraphEdgeCreate {
                project_id,
                subject,
                predicate,
                object,
            } => {
                let edge = self.pg(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut pg| pg.create_edge(&project_id, subject, predicate, object),
                )?;
                Ok(ResponseBody::GraphEdge { edge })
            }
            RequestBody::GraphEdgeRemove {
                edge_id,
                expected_revision,
            } => {
                let edge = self.pg(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut pg| pg.remove_edge(&edge_id, expected_revision),
                )?;
                Ok(ResponseBody::GraphEdge { edge })
            }
            RequestBody::GraphNeighbors {
                project_id,
                start,
                predicates,
                direction,
                limit,
                cursor,
            } => {
                let page = self.pg(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |pg| pg.neighbors(&project_id, start, predicates, direction, limit, cursor),
                )?;
                Ok(ResponseBody::GraphNeighbors { page })
            }
            RequestBody::ProjectContextResolve {
                project_id,
                experiment_id,
                refs_limit,
                graph_limit,
            } => {
                let context = self.pg(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |pg| pg.context(&project_id, experiment_id, refs_limit, graph_limit),
                )?;
                Ok(ResponseBody::ProjectContext { context })
            }
            RequestBody::ProjectSummaryQuery { project_id } => {
                let summary = self.pg(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |pg| pg.project_summary(&project_id),
                )?;
                Ok(ResponseBody::ProjectSummary { summary })
            }
            // Spec 075 data source fabric: every arm runs through `ds`
            // (lease + session gate already enforced above + meta + blobs).
            // No arm touches storage, drivers, or transports except through
            // `DataSources` methods.
            RequestBody::DataSourceCreate {
                project_id,
                display_name,
                locator,
                credential_ref,
            } => {
                let source = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut ds| ds.create_source(project_id, display_name, locator, credential_ref),
                )?;
                Ok(ResponseBody::DataSource {
                    source: Box::new(source),
                })
            }
            RequestBody::DataSourceGet { source_id } => {
                let source = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |ds| ds.get_source(&source_id),
                )?;
                Ok(ResponseBody::DataSource {
                    source: Box::new(source),
                })
            }
            RequestBody::DataSourceList {
                project_id,
                limit,
                cursor,
            } => {
                let (sources, next_cursor) = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |ds| ds.list_sources(&project_id, limit, &cursor),
                )?;
                Ok(ResponseBody::DataSourceList {
                    sources,
                    next_cursor,
                })
            }
            RequestBody::DataSourceUpdate {
                source_id,
                expected_revision,
                display_name,
                credential_ref,
            } => {
                let source = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut ds| {
                        ds.update_source(
                            &source_id,
                            expected_revision,
                            display_name,
                            credential_ref,
                        )
                    },
                )?;
                Ok(ResponseBody::DataSource {
                    source: Box::new(source),
                })
            }
            RequestBody::DataSourceArchive {
                source_id,
                expected_revision,
            } => {
                let source = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut ds| ds.archive_source(&source_id, expected_revision),
                )?;
                Ok(ResponseBody::DataSource {
                    source: Box::new(source),
                })
            }
            RequestBody::SnapshotImport { source_id } => {
                let (snapshot, receipt) = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut ds| ds.import_source(&source_id),
                )?;
                Ok(ResponseBody::SnapshotImported {
                    snapshot: Box::new(snapshot.snapshot),
                    receipt,
                })
            }
            RequestBody::SnapshotPreview {
                source_id,
                max_rows,
            } => {
                let (schema, rows) = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |ds| ds.preview_source(&source_id, max_rows),
                )?;
                Ok(ResponseBody::SnapshotPreview { schema, rows })
            }
            RequestBody::SnapshotGet { snapshot_id } => {
                let record = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |ds| ds.get_snapshot(&snapshot_id),
                )?;
                Ok(ResponseBody::Snapshot {
                    snapshot: Box::new(record.snapshot),
                })
            }
            RequestBody::SnapshotList {
                source_id,
                limit,
                cursor,
            } => {
                let (snapshots, next_cursor) = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |ds| ds.list_snapshots(&source_id, limit, &cursor),
                )?;
                Ok(ResponseBody::SnapshotList {
                    snapshots,
                    next_cursor,
                })
            }
            RequestBody::SnapshotRows {
                snapshot_id,
                limit,
                cursor,
                filters,
                sort,
            } => {
                let page = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |ds| ds.snapshot_rows(&snapshot_id, limit, &cursor, &filters, &sort),
                )?;
                Ok(ResponseBody::SnapshotRows { page })
            }
            RequestBody::SnapshotRefresh {
                source_id,
                allow_schema_change,
            } => {
                let (receipt, record) = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut ds| ds.refresh_source(&source_id, allow_schema_change),
                )?;
                Ok(ResponseBody::SnapshotRefreshed {
                    receipt,
                    snapshot: record.map(|record| Box::new(record.snapshot)),
                })
            }
            RequestBody::SavedViewCreate {
                snapshot_id,
                view_kind,
                state,
            } => {
                let view = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut ds| ds.create_view(snapshot_id, view_kind, state),
                )?;
                Ok(ResponseBody::SavedView {
                    view: Box::new(view),
                })
            }
            RequestBody::SavedViewGet { view_id } => {
                let view = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |ds| ds.get_view(&view_id),
                )?;
                Ok(ResponseBody::SavedView {
                    view: Box::new(view),
                })
            }
            RequestBody::SavedViewList {
                snapshot_id,
                limit,
                cursor,
            } => {
                let (views, next_cursor) = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |ds| ds.list_views(&snapshot_id, limit, &cursor),
                )?;
                Ok(ResponseBody::SavedViewList { views, next_cursor })
            }
            RequestBody::SavedViewUpdate {
                view_id,
                expected_revision,
                state,
            } => {
                let view = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut ds| ds.update_view(&view_id, expected_revision, state),
                )?;
                Ok(ResponseBody::SavedView {
                    view: Box::new(view),
                })
            }
            RequestBody::TransformExecute {
                input_snapshot_ids,
                ops,
            } => {
                let (snapshot, receipt) = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut ds| ds.execute_transformation(input_snapshot_ids, ops),
                )?;
                Ok(ResponseBody::Transformed {
                    snapshot: Box::new(snapshot.snapshot),
                    receipt,
                })
            }
            RequestBody::DatasetReleaseCreate {
                snapshot_id,
                version,
                split_group,
                annotation_schema_ref,
                rights_state,
            } => {
                let release = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut ds| {
                        ds.create_release(
                            snapshot_id,
                            version,
                            split_group,
                            annotation_schema_ref,
                            rights_state,
                        )
                    },
                )?;
                Ok(ResponseBody::DatasetRelease {
                    release: Box::new(release),
                })
            }
            RequestBody::DatasetReleaseGet { release_id } => {
                let release = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |ds| ds.get_release(&release_id),
                )?;
                Ok(ResponseBody::DatasetRelease {
                    release: Box::new(release),
                })
            }
            RequestBody::DatasetReleaseList {
                project_id,
                limit,
                cursor,
            } => {
                let (releases, next_cursor) = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |ds| ds.list_releases(&project_id, limit, &cursor),
                )?;
                Ok(ResponseBody::DatasetReleaseList {
                    releases,
                    next_cursor,
                })
            }
            // Spec 076 Collaboration Substrate: every mutation flows through
            // Core authority paths. Surfaces never write collaboration
            // storage directly. T076-03 slice only (participant/room/
            // membership); thread/message/task/note/approval/activity land
            // in later slices.
            RequestBody::ParticipantRegister {
                holder_id,
                kind,
                display_name,
                agent_profile_ref,
            } => {
                let participant = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut collab| {
                        collab.register_participant(
                            holder_id,
                            kind,
                            display_name,
                            agent_profile_ref,
                        )
                    },
                )?;
                Ok(ResponseBody::Participant {
                    participant: Box::new(participant),
                })
            }
            RequestBody::ParticipantGet { participant_id } => {
                let participant = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.get_participant(&participant_id),
                )?;
                Ok(ResponseBody::Participant {
                    participant: Box::new(participant),
                })
            }
            RequestBody::ParticipantRevoke {
                participant_id,
                expected_revision,
            } => {
                let participant = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut collab| collab.revoke_participant(&participant_id, expected_revision),
                )?;
                Ok(ResponseBody::Participant {
                    participant: Box::new(participant),
                })
            }
            RequestBody::RoomCreate {
                project_id,
                experiment_id,
                name,
            } => {
                let room = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut collab| collab.create_room(project_id, experiment_id, name),
                )?;
                Ok(ResponseBody::CollabRoom {
                    room: Box::new(room),
                })
            }
            RequestBody::RoomGet { room_id } => {
                let room = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.get_room(&room_id),
                )?;
                Ok(ResponseBody::CollabRoom {
                    room: Box::new(room),
                })
            }
            RequestBody::RoomList {
                project_id,
                status,
                limit,
                cursor,
            } => {
                let (rooms, next_cursor) = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.list_rooms(&project_id, status, limit.unwrap_or(25), cursor),
                )?;
                Ok(ResponseBody::CollabRoomList { rooms, next_cursor })
            }
            RequestBody::RoomRename {
                room_id,
                expected_revision,
                name,
            } => {
                let room = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.rename_room(&room_id, expected_revision, name),
                )?;
                Ok(ResponseBody::CollabRoom {
                    room: Box::new(room),
                })
            }
            RequestBody::RoomArchive {
                room_id,
                expected_revision,
            } => {
                let room = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.archive_room(&room_id, expected_revision),
                )?;
                Ok(ResponseBody::CollabRoom {
                    room: Box::new(room),
                })
            }
            RequestBody::RoomMembershipAdd {
                room_id,
                participant_id,
                role,
            } => {
                let membership = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.add_membership(&room_id, &participant_id, role),
                )?;
                Ok(ResponseBody::CollabMembership {
                    membership: Box::new(membership),
                })
            }
            RequestBody::RoomMembershipList { room_id } => {
                let memberships = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.list_memberships(&room_id),
                )?;
                Ok(ResponseBody::CollabMembershipList { memberships })
            }
            RequestBody::RoomMembershipRemove {
                room_id,
                membership_id,
                expected_revision,
            } => {
                let membership = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.remove_membership(&room_id, &membership_id, expected_revision),
                )?;
                Ok(ResponseBody::CollabMembership {
                    membership: Box::new(membership),
                })
            }
            // Spec 076 T076-04/05/06 slice: thread, message, task.
            RequestBody::ThreadOpen { room_id, anchor } => {
                let (thread, resolution) = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.open_thread(room_id, anchor),
                )?;
                Ok(ResponseBody::CollabThread {
                    thread: Box::new(thread),
                    resolution,
                })
            }
            RequestBody::ThreadGet { thread_id } => {
                let (thread, resolution) = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.get_thread(&thread_id),
                )?;
                Ok(ResponseBody::CollabThread {
                    thread: Box::new(thread),
                    resolution,
                })
            }
            RequestBody::ThreadList {
                room_id,
                limit,
                cursor,
            } => {
                let (resolved, next_cursor) = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.list_threads(&room_id, limit.unwrap_or(25), cursor),
                )?;
                let (threads, resolutions) = resolved.into_iter().unzip();
                Ok(ResponseBody::CollabThreadList {
                    threads,
                    resolutions,
                    next_cursor,
                })
            }
            RequestBody::ThreadSetStatus {
                thread_id,
                expected_revision,
                status,
            } => {
                let (thread, resolution) = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.set_thread_status(&thread_id, expected_revision, status),
                )?;
                Ok(ResponseBody::CollabThread {
                    thread: Box::new(thread),
                    resolution,
                })
            }
            RequestBody::MessagePost { thread_id, body } => {
                let message = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.post_message(&thread_id, body),
                )?;
                Ok(ResponseBody::CollabMessage {
                    message: Box::new(message),
                })
            }
            RequestBody::MessageList {
                thread_id,
                limit,
                after_seq,
            } => {
                let messages = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| {
                        collab.list_messages(
                            &thread_id,
                            limit.unwrap_or(50),
                            after_seq.unwrap_or(0),
                        )
                    },
                )?;
                Ok(ResponseBody::CollabMessageList { messages })
            }
            RequestBody::MessageEditBody {
                message_id,
                new_body,
            } => {
                let edit = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.edit_message(&message_id, new_body),
                )?;
                Ok(ResponseBody::CollabMessageEdit {
                    edit: Box::new(edit),
                })
            }
            RequestBody::MessageDelete { message_id } => {
                let edit = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.delete_message(&message_id),
                )?;
                Ok(ResponseBody::CollabMessageEdit {
                    edit: Box::new(edit),
                })
            }
            RequestBody::TaskCreate {
                room_id,
                anchor,
                title,
                description,
            } => {
                let task = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.create_task(room_id, anchor, title, description),
                )?;
                Ok(ResponseBody::CollabTask {
                    task: Box::new(task),
                })
            }
            RequestBody::TaskGet { task_id } => {
                let task = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.get_task(&task_id),
                )?;
                Ok(ResponseBody::CollabTask {
                    task: Box::new(task),
                })
            }
            RequestBody::TaskList {
                room_id,
                limit,
                cursor,
            } => {
                let (tasks, next_cursor) = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.list_tasks(&room_id, limit.unwrap_or(25), cursor),
                )?;
                Ok(ResponseBody::CollabTaskList { tasks, next_cursor })
            }
            RequestBody::TaskUpdate {
                task_id,
                expected_revision,
                status,
                assignee_participant_id,
            } => {
                let task = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| {
                        collab.update_task(
                            &task_id,
                            expected_revision,
                            status,
                            assignee_participant_id,
                        )
                    },
                )?;
                Ok(ResponseBody::CollabTask {
                    task: Box::new(task),
                })
            }
            // Spec 076 T076-07/08/09 slice: note, approval, activity.
            RequestBody::NoteCreate {
                room_id,
                title,
                body,
            } => {
                let note = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.create_note(room_id, title, body),
                )?;
                Ok(ResponseBody::CollabNote {
                    note: Box::new(note),
                })
            }
            RequestBody::NoteGet { note_id } => {
                let note = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.get_note(&note_id),
                )?;
                Ok(ResponseBody::CollabNote {
                    note: Box::new(note),
                })
            }
            RequestBody::NoteListRevisions { note_id } => {
                let revisions = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.list_note_revisions(&note_id),
                )?;
                Ok(ResponseBody::CollabNoteRevisionList { revisions })
            }
            RequestBody::NoteEdit {
                note_id,
                expected_revision,
                body,
            } => {
                let (note, new_revision, is_conflict_copy) = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.edit_note(&note_id, expected_revision, body),
                )?;
                Ok(ResponseBody::CollabNoteEdit {
                    note: Box::new(note),
                    new_revision: Box::new(new_revision),
                    is_conflict_copy,
                })
            }
            RequestBody::ApprovalRequestCreate {
                room_id,
                anchor,
                kind,
                assignee_participant_ids,
                blind_until_closed,
            } => {
                let request = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| {
                        collab.create_approval_request(
                            room_id,
                            anchor,
                            kind,
                            assignee_participant_ids,
                            blind_until_closed,
                        )
                    },
                )?;
                Ok(ResponseBody::CollabApprovalRequest {
                    request: Box::new(request),
                })
            }
            RequestBody::ApprovalRequestGet { request_id } => {
                let request = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.get_approval_request(&request_id),
                )?;
                Ok(ResponseBody::CollabApprovalRequest {
                    request: Box::new(request),
                })
            }
            RequestBody::ApprovalRequestWithdraw {
                request_id,
                expected_revision,
            } => {
                let request = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.withdraw_approval_request(&request_id, expected_revision),
                )?;
                Ok(ResponseBody::CollabApprovalRequest {
                    request: Box::new(request),
                })
            }
            RequestBody::ApprovalDecide {
                request_id,
                outcome,
                rationale,
            } => {
                let decision = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.decide_approval_request(&request_id, outcome, rationale),
                )?;
                Ok(ResponseBody::CollabApprovalDecision {
                    decision: Box::new(decision),
                })
            }
            RequestBody::ApprovalDecisionList { request_id } => {
                let decisions = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| collab.list_approval_decisions(&request_id),
                )?;
                Ok(ResponseBody::CollabApprovalDecisionList { decisions })
            }
            RequestBody::ActivityList {
                room_id,
                limit,
                after_seq,
            } => {
                let records = self.collab(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |collab| {
                        collab.list_activity(&room_id, limit.unwrap_or(100), after_seq.unwrap_or(0))
                    },
                )?;
                Ok(ResponseBody::CollabActivityList { records })
            }
            // Spec 077 MedAgent Workbench: every mutation flows through
            // Core authority paths. Surfaces never write medagent storage
            // directly. T077-03 slice only (AgentIdentity +
            // AgentCapabilityManifest).
            // Spec 078 Model Fleet + Compare: every mutation flows through
            // Core authority paths. T078-03 slice (AgentLane).
            // Spec 080 Governed Browse: policy, transport and persistence
            // all run in Core; Project content first needs a Spec 079
            // egress decision for the browse boundary.
            RequestBody::BrowseAllowlistAdd {
                project_id,
                host,
                path_prefix,
            } => {
                let entry = self.browse(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut b| b.allowlist_add(project_id, host, path_prefix),
                )?;
                Ok(ResponseBody::BrowseAllowlistEntry {
                    entry: Box::new(entry),
                })
            }
            RequestBody::BrowseAllowlistList { project_id } => {
                let entries = self.browse(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |b| b.allowlist_list(&project_id),
                )?;
                Ok(ResponseBody::BrowseAllowlist { entries })
            }
            RequestBody::BrowseAllowlistDisable {
                entry_id,
                expected_revision,
            } => {
                let entry = self.browse(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut b| b.allowlist_disable(&entry_id, expected_revision),
                )?;
                Ok(ResponseBody::BrowseAllowlistEntry {
                    entry: Box::new(entry),
                })
            }
            RequestBody::BrowseRouteList => Ok(ResponseBody::BrowseRoutes {
                routes: super::browse::Browse::routes(),
            }),
            RequestBody::BrowseRun { request } => {
                let context = match request.context_artifact_id.clone() {
                    Some(artifact_id) => {
                        let decision = self.privacy(
                            &req.vault_id,
                            req.realm_id.clone(),
                            req.authority_scope_id.clone(),
                            req.session_id.clone(),
                            |mut gate| {
                                gate.evaluate_egress(
                                    request.project_id.clone(),
                                    artifact_id,
                                    medscale_contracts::privacy_gate::EgressBoundary::Browse,
                                )
                            },
                        )?;
                        Some(super::browse::ContextEgress {
                            decision_id: decision.header.id,
                            allowed: decision.outcome
                                == medscale_contracts::privacy_gate::EgressOutcome::Allow,
                        })
                    }
                    None => None,
                };
                let view = self.browse(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut b| b.browse(request, context),
                )?;
                Ok(ResponseBody::BrowseSession {
                    view: Box::new(view),
                })
            }
            RequestBody::BrowseSessionGet { session_id } => {
                let view = self.browse(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |b| b.get_session(&session_id),
                )?;
                Ok(ResponseBody::BrowseSession {
                    view: Box::new(view),
                })
            }
            RequestBody::BrowseSessionList { project_id } => {
                let sessions = self.browse(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |b| b.list_sessions(&project_id),
                )?;
                Ok(ResponseBody::BrowseSessionList { sessions })
            }
            RequestBody::BrowseSessionCancel {
                session_id,
                expected_revision,
            } => {
                self.browse(
                    &req.vault_id,
                    req.realm_id.clone(),
                    req.authority_scope_id.clone(),
                    req.session_id.clone(),
                    |mut b| b.cancel_session(&session_id, expected_revision),
                )?;
                let view = self.browse(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |b| b.get_session(&session_id),
                )?;
                Ok(ResponseBody::BrowseSession {
                    view: Box::new(view),
                })
            }
            // Spec 082 Analytics Gate: runs on the Spec 075 data-source
            // authority; snapshots are read, never written.
            RequestBody::AnalyticsQuery { request } => {
                let value = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| {
                        d.analytics_query(
                            request,
                            medscale_contracts::analytics::QueryOrigin::SqlEditor,
                            None,
                        )
                    },
                )?;
                Ok(ResponseBody::AnalyticsQuery {
                    view: Box::new(value),
                })
            }
            RequestBody::AnalyticsReplay { receipt_id } => {
                let value = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.analytics_replay(&receipt_id),
                )?;
                Ok(ResponseBody::AnalyticsReplay { report: value })
            }
            RequestBody::AnalyticsReceiptGet { receipt_id } => {
                let value = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.analytics_receipt(&receipt_id),
                )?;
                Ok(ResponseBody::AnalyticsReceipt {
                    receipt: Box::new(value),
                })
            }
            RequestBody::AnalyticsReceiptList { project_id } => {
                let value = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.analytics_receipts(&project_id),
                )?;
                Ok(ResponseBody::AnalyticsReceipts { receipts: value })
            }
            RequestBody::AnalyticsResultGet { result_id } => {
                let value = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.analytics_result(&result_id),
                )?;
                Ok(ResponseBody::AnalyticsResult {
                    table: Box::new(value.0),
                    doc: Box::new(value.1),
                })
            }
            RequestBody::AnalyticsStatistics {
                result_id,
                column,
                kinds,
            } => {
                let value = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.analytics_statistics(&result_id, &column, &kinds),
                )?;
                Ok(ResponseBody::AnalyticsStatistics { results: value })
            }
            RequestBody::AnalyticsCohortCreate {
                project_id,
                label,
                snapshot_id,
                criteria,
            } => {
                let value = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.analytics_cohort_create(project_id, label, snapshot_id, criteria),
                )?;
                Ok(ResponseBody::AnalyticsCohort {
                    cohort: Box::new(value),
                })
            }
            RequestBody::AnalyticsCohortList { project_id } => {
                let value = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.analytics_cohorts(&project_id),
                )?;
                Ok(ResponseBody::AnalyticsCohorts { cohorts: value })
            }
            RequestBody::AnalyticsCohortRun {
                cohort_id,
                max_rows,
            } => {
                let value = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.analytics_cohort_run(&cohort_id, max_rows),
                )?;
                Ok(ResponseBody::AnalyticsQuery {
                    view: Box::new(value),
                })
            }
            // Spec 083 Knowledge + Research Canvas: a rebuildable lexical
            // projection over sources Core already governs; sources are read,
            // never written.
            RequestBody::KnowledgeIndexBuild { project_id } => {
                let value = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.knowledge_index_build(&project_id),
                )?;
                Ok(ResponseBody::KnowledgeIndexBuilt {
                    manifest: Box::new(value.0),
                    status: value.1,
                })
            }
            RequestBody::KnowledgeIndexStatus { project_id } => {
                let value = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.knowledge_index_status(&project_id),
                )?;
                Ok(ResponseBody::KnowledgeIndexStatus { status: value })
            }
            RequestBody::KnowledgeSearch { request } => {
                let value = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.knowledge_search(request),
                )?;
                Ok(ResponseBody::KnowledgeSearch {
                    result: Box::new(value),
                })
            }
            RequestBody::KnowledgeReceiptGet { receipt_id } => {
                let value = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.knowledge_receipt(&receipt_id),
                )?;
                Ok(ResponseBody::KnowledgeReceipt {
                    receipt: Box::new(value),
                })
            }
            RequestBody::KnowledgeReceiptList { project_id } => {
                let value = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.knowledge_receipts(&project_id),
                )?;
                Ok(ResponseBody::KnowledgeReceipts { receipts: value })
            }
            RequestBody::CanvasCreate { project_id, title } => {
                let value = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.canvas_create(project_id, title),
                )?;
                Ok(ResponseBody::Canvas {
                    canvas: Box::new(value),
                })
            }
            RequestBody::CanvasEdit {
                canvas_id,
                expected_revision,
                ops,
            } => {
                let value = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.canvas_edit(&canvas_id, expected_revision, ops),
                )?;
                Ok(ResponseBody::Canvas {
                    canvas: Box::new(value),
                })
            }
            RequestBody::CanvasGet {
                canvas_id,
                revision,
            } => {
                let value = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.canvas_view(&canvas_id, revision),
                )?;
                Ok(ResponseBody::CanvasView {
                    view: Box::new(value),
                })
            }
            RequestBody::CanvasList { project_id } => {
                let value = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.canvas_list(&project_id),
                )?;
                Ok(ResponseBody::Canvases { canvases: value })
            }
            // Spec 085 MedScale Compute: runs on the Spec 075 data-source
            // authority; the worker receives staged bytes, never the vault.
            RequestBody::ComputeSubmit { request } => {
                let value = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.compute_submit(*request),
                )?;
                Ok(ResponseBody::ComputeJob {
                    view: Box::new(value),
                })
            }
            RequestBody::ComputeRun { job_id } => {
                let cancel = self.compute_cancel.fresh();
                let runtime = &self.compute_runtime;
                let value = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.compute_run(&job_id, runtime, &cancel),
                )?;
                Ok(ResponseBody::ComputeJob {
                    view: Box::new(value),
                })
            }
            RequestBody::ComputeCancel { job_id } => {
                let value = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.compute_cancel(&job_id),
                )?;
                Ok(ResponseBody::ComputeJob {
                    view: Box::new(value),
                })
            }
            RequestBody::ComputeRecover => {
                let views = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.compute_recover(),
                )?;
                Ok(ResponseBody::ComputeRecovered { views })
            }
            RequestBody::ComputeJobGet { job_id } => {
                let value = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.compute_job(&job_id),
                )?;
                Ok(ResponseBody::ComputeJob {
                    view: Box::new(value),
                })
            }
            RequestBody::ComputeJobList { project_id } => {
                let jobs = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.compute_jobs(&project_id),
                )?;
                Ok(ResponseBody::ComputeJobs { jobs })
            }
            RequestBody::ComputeStatus => {
                let runtime = &self.compute_runtime;
                let status = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.compute_status(runtime),
                )?;
                Ok(ResponseBody::ComputeStatus { status })
            }
            // Spec 086 R Workspace: staging, external launch, recorded run
            // refusal and explicit publication on the Spec 075 authority.
            RequestBody::RWorkspaceStage { request } => {
                let host = &self.r_host;
                let workspace = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.r_stage(host, *request),
                )?;
                Ok(ResponseBody::RWorkspace {
                    workspace: Box::new(workspace),
                })
            }
            RequestBody::RWorkspaceInspect { workspace_id } => {
                let inspection = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.r_inspect(&workspace_id),
                )?;
                Ok(ResponseBody::RWorkspaceInspection { inspection })
            }
            RequestBody::RWorkspaceLaunch { request } => {
                let host = &self.r_host;
                let receipt = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.r_launch(host, request),
                )?;
                Ok(ResponseBody::RLaunch {
                    receipt: Box::new(receipt),
                })
            }
            RequestBody::RWorkspaceRun { request } => {
                let host = &self.r_host;
                let receipt = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.r_run(host, request),
                )?;
                Ok(ResponseBody::RRun {
                    receipt: Box::new(receipt),
                })
            }
            RequestBody::RWorkspacePublish { request } => {
                let view = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.r_publish(*request),
                )?;
                Ok(ResponseBody::RPublish {
                    view: Box::new(view),
                })
            }
            RequestBody::RWorkspaceGet { workspace_id } => {
                let history = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.r_workspace(&workspace_id),
                )?;
                Ok(ResponseBody::RWorkspaceHistory {
                    history: Box::new(history),
                })
            }
            RequestBody::RWorkspaceList { project_id } => {
                let workspaces = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.r_workspaces(&project_id),
                )?;
                Ok(ResponseBody::RWorkspaces { workspaces })
            }
            RequestBody::RWorkspacePublished { table_id } => {
                let view = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.r_published(&table_id),
                )?;
                Ok(ResponseBody::RPublish {
                    view: Box::new(view),
                })
            }
            RequestBody::RWorkspaceStatus => {
                let host = &self.r_host;
                let status = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| Ok(d.r_status(host)),
                )?;
                Ok(ResponseBody::RWorkspaceStatus { status })
            }
            // Spec 091 federation on the Spec 075 authority.
            RequestBody::FederationAct { act } => {
                let result = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.fed_act(*act),
                )?;
                Ok(ResponseBody::FederationActed {
                    result: Box::new(result),
                })
            }
            RequestBody::FederationGet => {
                let view = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.fed_view(),
                )?;
                Ok(ResponseBody::Federation {
                    view: Box::new(view),
                })
            }
            // Spec 090 institutional adapters on the Spec 075 authority.
            RequestBody::AdapterAct { act } => {
                let transport = self.institutional_transport.clone();
                let result = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.adapter_act(*act, transport.as_ref()),
                )?;
                Ok(ResponseBody::AdapterActed {
                    result: Box::new(result),
                })
            }
            RequestBody::AdapterList { project_id } => {
                let adapters = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.adapter_list(&project_id),
                )?;
                Ok(ResponseBody::Adapters { adapters })
            }
            RequestBody::AdapterGet { adapter_id } => {
                let view = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.adapter_view(&adapter_id),
                )?;
                Ok(ResponseBody::Adapter {
                    view: Box::new(view),
                })
            }
            // Spec 089 Research Packs on the Spec 075 authority.
            RequestBody::PackAct { act } => {
                let result = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.pack_act(*act),
                )?;
                Ok(ResponseBody::PackActed {
                    result: Box::new(result),
                })
            }
            RequestBody::ModelCatalogQuery { query } => {
                // Read-only metadata over a local snapshot: no vault, no network.
                let page = crate::model_catalog::catalog_page(&query).map_err(|e| {
                    AuthorityError::InvalidArgument {
                        message: e.to_string(),
                    }
                })?;
                Ok(ResponseBody::ModelCatalogPage {
                    page: Box::new(page),
                })
            }
            RequestBody::PackCatalog => Ok(ResponseBody::PackCatalog {
                packs: super::data_sources::DataSources::pack_catalog(),
            }),
            RequestBody::PackInstallGet {
                project_id,
                pack_id,
            } => {
                let install = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.pack_install_get(&project_id, &pack_id),
                )?;
                Ok(ResponseBody::PackInstall {
                    install: Box::new(install),
                })
            }
            RequestBody::PackArtifactGet { artifact_id } => {
                let artifact = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.artifact_get(&artifact_id),
                )?;
                Ok(ResponseBody::PackArtifact {
                    artifact: Box::new(artifact),
                })
            }
            RequestBody::PackArtifactList {
                project_id,
                pack_id,
            } => {
                let artifacts = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.artifact_list(&project_id, &pack_id),
                )?;
                Ok(ResponseBody::PackArtifacts { artifacts })
            }
            // Spec 087 Community Extensions (declarative only) on the Spec
            // 075 authority: verification, lifecycle, grants, invocation.
            RequestBody::ExtensionTrustPublisher {
                publisher_id,
                key_hex,
            } => {
                let publisher = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.ext_trust_publisher(publisher_id, key_hex),
                )?;
                Ok(ResponseBody::ExtensionPublisher {
                    publisher: Box::new(publisher),
                })
            }
            RequestBody::ExtensionRevokePublisher { publisher_id } => {
                let receipts = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.ext_revoke_publisher(&publisher_id),
                )?;
                Ok(ResponseBody::ExtensionLifecycles { receipts })
            }
            RequestBody::ExtensionRevokeRelease { digest } => {
                let receipts = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.ext_revoke_release(&digest),
                )?;
                Ok(ResponseBody::ExtensionLifecycles { receipts })
            }
            RequestBody::ExtensionInstall {
                project_id,
                pack_json,
                upgrade,
            } => {
                let receipt = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| {
                        if upgrade {
                            d.ext_upgrade(&project_id, &pack_json)
                        } else {
                            d.ext_install(&project_id, &pack_json)
                        }
                    },
                )?;
                Ok(ResponseBody::ExtensionLifecycle {
                    receipt: Box::new(receipt),
                })
            }
            RequestBody::ExtensionRollback {
                project_id,
                extension_id,
            } => {
                let receipt = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.ext_rollback(&project_id, &extension_id),
                )?;
                Ok(ResponseBody::ExtensionLifecycle {
                    receipt: Box::new(receipt),
                })
            }
            RequestBody::ExtensionSetEnabled {
                project_id,
                extension_id,
                enabled,
            } => {
                let receipt = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.ext_set_enabled(&project_id, &extension_id, enabled),
                )?;
                Ok(ResponseBody::ExtensionLifecycle {
                    receipt: Box::new(receipt),
                })
            }
            RequestBody::ExtensionUninstall {
                project_id,
                extension_id,
            } => {
                let receipt = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.ext_uninstall(&project_id, &extension_id),
                )?;
                Ok(ResponseBody::ExtensionLifecycle {
                    receipt: Box::new(receipt),
                })
            }
            RequestBody::ExtensionGrant {
                project_id,
                extension_id,
                capability,
                ceiling,
            } => {
                let receipt = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.ext_grant(&project_id, &extension_id, capability, ceiling),
                )?;
                Ok(ResponseBody::ExtensionLifecycle {
                    receipt: Box::new(receipt),
                })
            }
            RequestBody::ExtensionInvoke {
                project_id,
                extension_id,
                command,
                target,
            } => {
                let outcome = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut d| d.ext_invoke(&project_id, &extension_id, &command, target),
                )?;
                Ok(ResponseBody::ExtensionInvocation {
                    outcome: Box::new(outcome),
                })
            }
            RequestBody::ExtensionList { project_id } => {
                let view = self.ds(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |d| d.ext_list(&project_id),
                )?;
                Ok(ResponseBody::ExtensionProject {
                    view: Box::new(view),
                })
            }
            // Spec 084 MedScale Hub foundation: Hub side (operator and
            // device) and client side (links, outbox, mirror).
            RequestBody::HubInit => {
                let hub = self.hub(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |h| h.init(),
                )?;
                Ok(ResponseBody::HubIdentity { hub: Box::new(hub) })
            }
            RequestBody::HubInvite {
                project_id,
                display_name,
            } => {
                let (invitation, code) = self.hub(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |h| h.invite(project_id, display_name),
                )?;
                Ok(ResponseBody::HubInvited {
                    invitation: Box::new(invitation),
                    code: Box::new(code),
                })
            }
            RequestBody::HubInvitationRevoke { invitation_id } => {
                let invitation = self.hub(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |h| h.revoke_invitation(&invitation_id),
                )?;
                Ok(ResponseBody::HubInvitation {
                    invitation: Box::new(invitation),
                })
            }
            RequestBody::HubDeviceRevoke { device_id } => {
                let device = self.hub(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut h| h.revoke_device(&device_id),
                )?;
                Ok(ResponseBody::HubDevice {
                    device: Box::new(device),
                })
            }
            RequestBody::HubStatus => {
                let status = self.hub(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |h| h.status(),
                )?;
                Ok(ResponseBody::HubStatus {
                    status: Box::new(status),
                })
            }
            RequestBody::HubEnroll {
                token_hex,
                public_key_hex,
                signature_hex,
            } => {
                let device = self.hub(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    None,
                    |mut h| h.enroll(&token_hex, &public_key_hex, &signature_hex),
                )?;
                Ok(ResponseBody::HubDevice {
                    device: Box::new(device),
                })
            }
            RequestBody::HubChallenge { device_id } => {
                let challenge = self.hub(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    None,
                    |h| h.challenge(&device_id),
                )?;
                Ok(ResponseBody::HubChallenge {
                    challenge: Box::new(challenge),
                })
            }
            RequestBody::HubHandshake { handshake } => {
                let session = self.hub(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    None,
                    |h| h.handshake(&handshake),
                )?;
                Ok(ResponseBody::HubSession {
                    session: Box::new(session),
                })
            }
            RequestBody::HubSubmit { envelopes } => {
                let outcomes = self.hub(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut h| h.submit(envelopes),
                )?;
                Ok(ResponseBody::HubOutcomes { outcomes })
            }
            RequestBody::HubPull { after, limit } => {
                let page = self.hub(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |h| h.pull(after, limit),
                )?;
                Ok(ResponseBody::HubEvents {
                    page: Box::new(page),
                })
            }
            RequestBody::HubJoinPrepare { code } => {
                let prepared =
                    self.hub_client(&req.vault_id, req.realm_id, req.authority_scope_id, |c| {
                        c.join_prepare(&code)
                    })?;
                Ok(ResponseBody::HubJoinPrepared {
                    link_id: prepared.link_id,
                    public_key_hex: prepared.public_key_hex,
                    signature_hex: prepared.signature_hex,
                })
            }
            RequestBody::HubJoinComplete {
                link_id,
                endpoint,
                code,
                device,
            } => {
                let link =
                    self.hub_client(&req.vault_id, req.realm_id, req.authority_scope_id, |c| {
                        c.join_complete(&link_id, endpoint, &code, &device)
                    })?;
                Ok(ResponseBody::HubLink {
                    link: Box::new(link),
                })
            }
            RequestBody::HubQueue { link_id, intent } => {
                let entry =
                    self.hub_client(&req.vault_id, req.realm_id, req.authority_scope_id, |c| {
                        c.queue(&link_id, intent)
                    })?;
                Ok(ResponseBody::HubQueued {
                    entry: Box::new(entry),
                })
            }
            RequestBody::HubSignHandshake { link_id, challenge } => {
                let handshake =
                    self.hub_client(&req.vault_id, req.realm_id, req.authority_scope_id, |c| {
                        c.sign_handshake(&link_id, &challenge)
                    })?;
                Ok(ResponseBody::HubHandshakeSigned {
                    handshake: Box::new(handshake),
                })
            }
            RequestBody::HubRecordOutcomes { link_id, outcomes } => {
                self.hub_client(&req.vault_id, req.realm_id, req.authority_scope_id, |c| {
                    c.record_outcomes(&link_id, &outcomes)
                })?;
                Ok(ResponseBody::HubRecorded)
            }
            RequestBody::HubMirrorAppend { link_id, page } => {
                let link =
                    self.hub_client(&req.vault_id, req.realm_id, req.authority_scope_id, |c| {
                        c.mirror_append(&link_id, &page)
                    })?;
                Ok(ResponseBody::HubLink {
                    link: Box::new(link),
                })
            }
            RequestBody::HubLinkList => {
                let links =
                    self.hub_client(&req.vault_id, req.realm_id, req.authority_scope_id, |c| {
                        c.links()
                    })?;
                Ok(ResponseBody::HubLinks { links })
            }
            RequestBody::HubLinkGet { link_id } => {
                let link =
                    self.hub_client(&req.vault_id, req.realm_id, req.authority_scope_id, |c| {
                        c.link(&link_id)
                    })?;
                Ok(ResponseBody::HubLink {
                    link: Box::new(link),
                })
            }
            RequestBody::HubOutboxList {
                link_id,
                pending_only,
            } => {
                let entries =
                    self.hub_client(&req.vault_id, req.realm_id, req.authority_scope_id, |c| {
                        c.outbox(&link_id, pending_only)
                    })?;
                Ok(ResponseBody::HubOutbox { entries })
            }
            RequestBody::HubMirrorList {
                link_id,
                after,
                limit,
            } => {
                let events =
                    self.hub_client(&req.vault_id, req.realm_id, req.authority_scope_id, |c| {
                        c.mirror(&link_id, after, limit)
                    })?;
                Ok(ResponseBody::HubMirror { events })
            }
            // Spec 081 AudioFlow Foundation: local only; every operation
            // runs in Core over vault storage.
            RequestBody::AudioImport {
                project_id,
                label,
                wav,
            } => {
                let value = self.audio(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut a| a.import(project_id, label, wav),
                )?;
                Ok(ResponseBody::AudioSource {
                    source: Box::new(value),
                })
            }
            RequestBody::AudioSourceGet { source_id } => {
                let value = self.audio(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |a| a.get_source(&source_id),
                )?;
                Ok(ResponseBody::AudioSource {
                    source: Box::new(value),
                })
            }
            RequestBody::AudioSourceList { project_id } => {
                let value = self.audio(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |a| a.list_sources(&project_id),
                )?;
                Ok(ResponseBody::AudioSources { sources: value })
            }
            RequestBody::AudioCaptureStart {
                project_id,
                label,
                backend,
                format,
            } => {
                let value = self.audio(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut a| a.capture_start(project_id, label, backend, format),
                )?;
                Ok(ResponseBody::AudioCapture {
                    session: Box::new(value),
                })
            }
            RequestBody::AudioCaptureAppend {
                session_id,
                expected_revision,
                frames,
            } => {
                let value = self.audio(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut a| a.capture_append(&session_id, expected_revision, frames),
                )?;
                Ok(ResponseBody::AudioCapture {
                    session: Box::new(value),
                })
            }
            RequestBody::AudioCaptureTransition {
                session_id,
                expected_revision,
                to,
            } => {
                let value = self.audio(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut a| a.capture_transition(&session_id, expected_revision, to),
                )?;
                Ok(ResponseBody::AudioCapture {
                    session: Box::new(value),
                })
            }
            RequestBody::AudioCaptureStop {
                session_id,
                expected_revision,
            } => {
                let value = self.audio(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut a| a.capture_stop(&session_id, expected_revision),
                )?;
                Ok(ResponseBody::AudioCaptureStopped {
                    session: Box::new(value.0),
                    source: Box::new(value.1),
                })
            }
            RequestBody::AudioCaptureGet { session_id } => {
                let value = self.audio(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut a| a.capture_get(&session_id),
                )?;
                Ok(ResponseBody::AudioCapture {
                    session: Box::new(value),
                })
            }
            RequestBody::AudioCaptureList { project_id } => {
                let value = self.audio(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut a| a.capture_list(&project_id),
                )?;
                Ok(ResponseBody::AudioCaptures { sessions: value })
            }
            RequestBody::AudioRouteList => Ok(ResponseBody::AudioRoutes {
                routes: super::audio::route_statuses(self.asr_engine.0.is_some()),
            }),
            // Spec 088 AudioFlow Advanced huddles on the Spec 081 authority.
            RequestBody::HuddleAct { act } => {
                let result = self.audio(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut a| a.huddle_act(*act),
                )?;
                Ok(ResponseBody::HuddleActed {
                    result: Box::new(result),
                })
            }
            RequestBody::HuddleGet { huddle_id } => {
                let view = self.audio(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |a| a.huddle_view(&huddle_id),
                )?;
                Ok(ResponseBody::Huddle {
                    view: Box::new(view),
                })
            }
            RequestBody::HuddleList { project_id } => {
                let huddles = self.audio(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |a| a.huddle_list(&project_id),
                )?;
                Ok(ResponseBody::Huddles { huddles })
            }
            RequestBody::AudioTranscribe { request } => {
                let value = self.audio(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut a| a.transcribe(request),
                )?;
                Ok(ResponseBody::AudioTranscript {
                    view: Box::new(value),
                })
            }
            RequestBody::AudioTranscriptList { source_id } => {
                let value = self.audio(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |a| a.transcripts(&source_id),
                )?;
                Ok(ResponseBody::AudioTranscripts { revisions: value })
            }
            RequestBody::AudioTranscriptGet { revision_id } => {
                let value = self.audio(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |a| a.transcript(&revision_id),
                )?;
                Ok(ResponseBody::AudioTranscriptRevision {
                    revision: Box::new(value),
                })
            }
            RequestBody::AudioReceiptList { source_id } => {
                let value = self.audio(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |a| a.receipts(&source_id),
                )?;
                Ok(ResponseBody::AudioReceipts { receipts: value })
            }
            RequestBody::AudioTranscriptCorrect {
                revision_id,
                edits,
                reason,
            } => {
                let edits = edits
                    .into_iter()
                    .map(|(seq, text)| super::audio::SegmentEdit { seq, text })
                    .collect();
                let value = self.audio(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut a| a.correct(&revision_id, edits, reason),
                )?;
                Ok(ResponseBody::AudioTranscriptRevision {
                    revision: Box::new(value),
                })
            }
            RequestBody::AudioEvidenceGet {
                revision_id,
                segment_seq,
            } => {
                let value = self.audio(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |a| a.evidence(&revision_id, segment_seq),
                )?;
                Ok(ResponseBody::AudioEvidence { evidence: value })
            }
            // Spec 079 Privacy Gate: every classification, transform,
            // pseudonym and egress decision flows through Core.
            RequestBody::PrivacyClassify {
                project_id,
                artifact_id,
                data_class,
                expected_revision,
            } => {
                let result = self.privacy(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut gate| {
                        gate.classify_artifact(
                            project_id,
                            artifact_id,
                            data_class,
                            expected_revision,
                        )
                    },
                )?;
                Ok(ResponseBody::PrivacyClassification {
                    classification: Box::new(result),
                })
            }
            RequestBody::PrivacyClassificationGet {
                project_id,
                artifact_id,
            } => {
                let result = self.privacy(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |gate| gate.effective_classification(&project_id, &artifact_id),
                )?;
                Ok(ResponseBody::PrivacyEffectiveClassification {
                    effective: Box::new(result),
                })
            }
            RequestBody::PrivacyClassificationList { project_id } => {
                let result = self.privacy(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |gate| gate.list_classifications(&project_id),
                )?;
                Ok(ResponseBody::PrivacyClassificationList {
                    classifications: result,
                })
            }
            RequestBody::PrivacyProfileCreate {
                project_id,
                name,
                target_class,
                rules,
                use_model_recognizer,
            } => {
                let result = self.privacy(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut gate| {
                        gate.create_profile(
                            project_id,
                            name,
                            target_class,
                            rules,
                            use_model_recognizer,
                        )
                    },
                )?;
                Ok(ResponseBody::PrivacyProfile {
                    profile: Box::new(result),
                })
            }
            RequestBody::PrivacyProfileGet { profile_id } => {
                let result = self.privacy(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |gate| gate.get_profile(&profile_id),
                )?;
                Ok(ResponseBody::PrivacyProfile {
                    profile: Box::new(result),
                })
            }
            RequestBody::PrivacyProfileList { project_id } => {
                let result = self.privacy(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |gate| gate.list_profiles(&project_id),
                )?;
                Ok(ResponseBody::PrivacyProfileList { profiles: result })
            }
            RequestBody::PrivacyProfileRevoke {
                profile_id,
                expected_revision,
            } => {
                let result = self.privacy(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut gate| gate.revoke_profile(&profile_id, expected_revision),
                )?;
                Ok(ResponseBody::PrivacyProfile {
                    profile: Box::new(result),
                })
            }
            RequestBody::PrivacyMapCreate { project_id } => {
                let result = self.privacy(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut gate| gate.create_pseudonym_map(project_id),
                )?;
                Ok(ResponseBody::PrivacyMap {
                    map: Box::new(result),
                })
            }
            RequestBody::PrivacyMapList { project_id } => {
                let result = self.privacy(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |gate| gate.list_pseudonym_maps(&project_id),
                )?;
                Ok(ResponseBody::PrivacyMapList { maps: result })
            }
            RequestBody::PrivacyMapRevoke {
                map_id,
                expected_revision,
            } => {
                let result = self.privacy(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut gate| gate.revoke_pseudonym_map(&map_id, expected_revision),
                )?;
                Ok(ResponseBody::PrivacyMap {
                    map: Box::new(result),
                })
            }
            RequestBody::PrivacyTransform {
                project_id,
                source_artifact_id,
                profile_id,
                pseudonym_map_id,
                model_pack_id,
                model_pack_path,
                synthetic_only,
            } => {
                let model = match (model_pack_id, model_pack_path) {
                    (Some(pack_id), Some(local_path)) => {
                        Some(super::privacy_gate::ModelRecognizerRequest {
                            pack_id,
                            local_path,
                        })
                    }
                    (None, None) => None,
                    _ => {
                        return Err(AuthorityError::InvalidArgument {
                            message: "model_pack_id and model_pack_path go together".to_owned(),
                        });
                    }
                };
                let receipt = self.privacy(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut gate| {
                        gate.transform(super::privacy_gate::TransformRequest {
                            project_id,
                            source_artifact_id,
                            profile_id,
                            pseudonym_map_id,
                            model,
                            synthetic_only,
                        })
                    },
                )?;
                Ok(ResponseBody::PrivacyReceipt {
                    receipt: Box::new(receipt),
                })
            }
            RequestBody::PrivacyReceiptGet { receipt_id } => {
                let result = self.privacy(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |gate| gate.get_receipt(&receipt_id),
                )?;
                Ok(ResponseBody::PrivacyReceipt {
                    receipt: Box::new(result),
                })
            }
            RequestBody::PrivacyReceiptList { project_id } => {
                let result = self.privacy(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |gate| gate.list_receipts(&project_id),
                )?;
                Ok(ResponseBody::PrivacyReceiptList { receipts: result })
            }
            RequestBody::PrivacyReceiptRevoke {
                receipt_id,
                expected_revision,
            } => {
                let result = self.privacy(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut gate| gate.revoke_receipt(&receipt_id, expected_revision),
                )?;
                Ok(ResponseBody::PrivacyReceipt {
                    receipt: Box::new(result),
                })
            }
            RequestBody::PrivacyReidentify {
                map_id,
                pseudonym,
                reason,
            } => {
                let (audit, value) = self.privacy(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut gate| gate.reidentify(&map_id, pseudonym, reason),
                )?;
                Ok(ResponseBody::PrivacyReidentified {
                    audit: Box::new(audit),
                    value,
                })
            }
            RequestBody::PrivacyReidentificationAuditList { map_id } => {
                let result = self.privacy(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |gate| gate.list_reid_audit(&map_id),
                )?;
                Ok(ResponseBody::PrivacyReidentificationAuditList { audits: result })
            }
            RequestBody::PrivacyEgressEvaluate {
                project_id,
                artifact_id,
                boundary,
            } => {
                let result = self.privacy(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut gate| gate.evaluate_egress(project_id, artifact_id, boundary),
                )?;
                Ok(ResponseBody::PrivacyEgressDecision {
                    decision: Box::new(result),
                })
            }
            RequestBody::PrivacyEgressDecisionList {
                project_id,
                artifact_id,
            } => {
                let result = self.privacy(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |gate| gate.list_egress_decisions(&project_id, artifact_id.as_ref()),
                )?;
                Ok(ResponseBody::PrivacyEgressDecisionList { decisions: result })
            }
            RequestBody::ComparisonCompute { fleet_run_id } => {
                let report = self.model_fleet(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut fleet| fleet.compute_comparison(&fleet_run_id),
                )?;
                Ok(ResponseBody::ModelFleetComparisonReport {
                    report: Box::new(report),
                })
            }
            RequestBody::ComparisonReportList { fleet_run_id } => {
                let reports = self.model_fleet(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |fleet| fleet.list_comparison_reports(&fleet_run_id),
                )?;
                Ok(ResponseBody::ModelFleetComparisonReportList { reports })
            }
            RequestBody::FleetRunCreate {
                project_id,
                task_prompt,
            } => {
                let run = self.model_fleet(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut fleet| fleet.create_fleet_run(project_id, task_prompt),
                )?;
                Ok(ResponseBody::ModelFleetRun {
                    run: Box::new(run),
                    lane_run_refs: Vec::new(),
                })
            }
            RequestBody::FleetRunGet { fleet_run_id } => {
                let (run, lane_run_refs) = self.model_fleet(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |fleet| fleet.get_fleet_run(&fleet_run_id),
                )?;
                Ok(ResponseBody::ModelFleetRun {
                    run: Box::new(run),
                    lane_run_refs,
                })
            }
            RequestBody::FleetRunList {
                project_id,
                status,
                limit,
            } => {
                let runs = self.model_fleet(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |fleet| fleet.list_fleet_runs(&project_id, status, limit.unwrap_or(100)),
                )?;
                Ok(ResponseBody::ModelFleetRunList { runs })
            }
            RequestBody::FleetRunDispatch {
                fleet_run_id,
                expected_revision,
                lane_ids,
            } => {
                let (run, lane_run_refs) = self.model_fleet(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut fleet| {
                        fleet.dispatch_fleet_run(&fleet_run_id, expected_revision, lane_ids)
                    },
                )?;
                Ok(ResponseBody::ModelFleetRun {
                    run: Box::new(run),
                    lane_run_refs,
                })
            }
            RequestBody::FleetRunExecuteLane {
                fleet_run_id,
                lane_id,
                local_path,
                max_tokens,
                synthetic_only,
            } => {
                let (run, lane_run, proposal) = self.model_fleet(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut fleet| {
                        fleet.execute_fleet_lane(
                            &fleet_run_id,
                            &lane_id,
                            &local_path,
                            max_tokens,
                            synthetic_only,
                        )
                    },
                )?;
                Ok(ResponseBody::ModelFleetLaneExecuted {
                    run: Box::new(run),
                    lane_run: Box::new(lane_run),
                    proposal: proposal.map(Box::new),
                })
            }
            RequestBody::FleetRunCancel {
                fleet_run_id,
                expected_revision,
            } => {
                let (run, lane_run_refs) = self.model_fleet(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut fleet| fleet.cancel_fleet_run(&fleet_run_id, expected_revision),
                )?;
                Ok(ResponseBody::ModelFleetRun {
                    run: Box::new(run),
                    lane_run_refs,
                })
            }
            RequestBody::AgentLaneCreate {
                project_id,
                agent_identity_id,
                context_manifest_id,
                role_label,
                granted_tool_kinds,
                context_artifact_ids,
            } => {
                let lane = self.model_fleet(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut fleet| {
                        fleet.create_agent_lane(
                            project_id,
                            agent_identity_id,
                            context_manifest_id,
                            role_label,
                            granted_tool_kinds,
                            context_artifact_ids,
                        )
                    },
                )?;
                Ok(ResponseBody::ModelFleetLane {
                    lane: Box::new(lane),
                })
            }
            RequestBody::AgentLaneGet { lane_id } => {
                let lane = self.model_fleet(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |fleet| fleet.get_agent_lane(&lane_id),
                )?;
                Ok(ResponseBody::ModelFleetLane {
                    lane: Box::new(lane),
                })
            }
            RequestBody::AgentLaneList {
                project_id,
                status,
                limit,
            } => {
                let lanes = self.model_fleet(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |fleet| fleet.list_agent_lanes(&project_id, status, limit.unwrap_or(100)),
                )?;
                Ok(ResponseBody::ModelFleetLaneList { lanes })
            }
            RequestBody::AgentLaneRetire {
                lane_id,
                expected_revision,
            } => {
                let lane = self.model_fleet(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut fleet| fleet.retire_agent_lane(&lane_id, expected_revision),
                )?;
                Ok(ResponseBody::ModelFleetLane {
                    lane: Box::new(lane),
                })
            }
            RequestBody::AgentIdentityRegister {
                project_id,
                pack_id,
                display_name,
                granted_tool_kinds,
            } => {
                let (identity, capabilities) = self.medagent(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut medagent| {
                        medagent.register_agent_identity(
                            project_id,
                            pack_id,
                            display_name,
                            granted_tool_kinds,
                        )
                    },
                )?;
                Ok(ResponseBody::MedAgentIdentity {
                    identity: Box::new(identity),
                    capabilities: Box::new(capabilities),
                })
            }
            RequestBody::AgentIdentityGet { agent_id } => {
                let (identity, capabilities) = self.medagent(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |medagent| medagent.get_agent_identity(&agent_id),
                )?;
                Ok(ResponseBody::MedAgentIdentity {
                    identity: Box::new(identity),
                    capabilities: Box::new(capabilities),
                })
            }
            RequestBody::AgentIdentityList { project_id, limit } => {
                let identities = self.medagent(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |medagent| medagent.list_agent_identities(&project_id, limit.unwrap_or(100)),
                )?;
                Ok(ResponseBody::MedAgentIdentityList { identities })
            }
            RequestBody::AgentIdentityRevoke {
                agent_id,
                expected_revision,
            } => {
                let (identity, capabilities) = self.medagent(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut medagent| medagent.revoke_agent_identity(&agent_id, expected_revision),
                )?;
                Ok(ResponseBody::MedAgentIdentity {
                    identity: Box::new(identity),
                    capabilities: Box::new(capabilities),
                })
            }
            RequestBody::ContextManifestCreate {
                project_id,
                selected_artifacts,
            } => {
                let (manifest, resolutions) = self.medagent(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut medagent| {
                        let manifest =
                            medagent.create_context_manifest(project_id, selected_artifacts)?;
                        let (manifest, resolutions) =
                            medagent.get_context_manifest(&manifest.header.id)?;
                        Ok((manifest, resolutions))
                    },
                )?;
                Ok(ResponseBody::MedAgentContextManifest {
                    manifest: Box::new(manifest),
                    resolutions,
                })
            }
            RequestBody::ContextManifestGet { context_id } => {
                let (manifest, resolutions) = self.medagent(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |medagent| medagent.get_context_manifest(&context_id),
                )?;
                Ok(ResponseBody::MedAgentContextManifest {
                    manifest: Box::new(manifest),
                    resolutions,
                })
            }
            RequestBody::AgentRunCreate {
                project_id,
                agent_identity_id,
                context_manifest_id,
                prompt,
            } => {
                let run = self.medagent(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut medagent| {
                        medagent.create_agent_run(
                            project_id,
                            agent_identity_id,
                            context_manifest_id,
                            prompt,
                        )
                    },
                )?;
                Ok(ResponseBody::MedAgentRun { run: Box::new(run) })
            }
            RequestBody::AgentRunGet { run_id } => {
                let run = self.medagent(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |medagent| medagent.get_agent_run(&run_id),
                )?;
                Ok(ResponseBody::MedAgentRun { run: Box::new(run) })
            }
            RequestBody::AgentRunList {
                project_id,
                agent_id,
                status,
                limit,
            } => {
                let runs = self.medagent(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |medagent| {
                        medagent.list_agent_runs(
                            &project_id,
                            agent_id.as_ref(),
                            status,
                            limit.unwrap_or(100),
                        )
                    },
                )?;
                Ok(ResponseBody::MedAgentRunList { runs })
            }
            RequestBody::AgentRunStart {
                run_id,
                expected_revision,
            } => {
                let (run, turn) = self.medagent(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut medagent| medagent.start_agent_run(&run_id, expected_revision),
                )?;
                Ok(ResponseBody::MedAgentRunStarted {
                    run: Box::new(run),
                    turn: Box::new(turn),
                })
            }
            RequestBody::AgentRunCancel {
                run_id,
                expected_revision,
            } => {
                let (run, receipt) = self.medagent(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut medagent| medagent.cancel_agent_run(&run_id, expected_revision),
                )?;
                Ok(ResponseBody::MedAgentRunTerminal {
                    run: Box::new(run),
                    receipt: Box::new(receipt),
                })
            }
            RequestBody::AgentRunTurnList { run_id } => {
                let turns = self.medagent(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |medagent| medagent.list_agent_turns(&run_id),
                )?;
                Ok(ResponseBody::MedAgentTurnList { turns })
            }
            RequestBody::AgentToolInvoke {
                run_id,
                kind,
                arguments,
            } => {
                // Spec 078 security.md T2: a run bound to an agent lane is
                // held to the lane's narrower policy before Spec 077's own
                // capability/context checks run.
                self.model_fleet(
                    &req.vault_id,
                    req.realm_id.clone(),
                    req.authority_scope_id.clone(),
                    req.session_id.clone(),
                    |mut fleet| fleet.require_lane_policy_allows_tool(&run_id, kind, &arguments),
                )?;
                let (invocation, receipt) = self.medagent(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut medagent| medagent.invoke_tool(&run_id, kind, arguments),
                )?;
                Ok(ResponseBody::MedAgentToolInvocation {
                    invocation: Box::new(invocation),
                    receipt: receipt.map(Box::new),
                })
            }
            RequestBody::AgentRunExecute {
                run_id,
                local_path,
                max_tokens,
                synthetic_only,
            } => {
                let max_tokens =
                    usize::try_from(max_tokens).map_err(|_| AuthorityError::InvalidArgument {
                        message: "max_tokens is not representable".to_owned(),
                    })?;
                let (turn, proposal) = self.medagent(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut medagent| {
                        medagent.execute_agent_run(&run_id, &local_path, max_tokens, synthetic_only)
                    },
                )?;
                Ok(ResponseBody::MedAgentRunExecuted {
                    turn: Box::new(turn),
                    proposal: Box::new(proposal),
                })
            }
            RequestBody::AgentRunComplete {
                run_id,
                expected_revision,
            } => {
                let (run, receipt) = self.medagent(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut medagent| medagent.complete_agent_run(&run_id, expected_revision),
                )?;
                Ok(ResponseBody::MedAgentRunTerminal {
                    run: Box::new(run),
                    receipt: Box::new(receipt),
                })
            }
            RequestBody::AgentRunFail {
                run_id,
                expected_revision,
                failure_reason,
            } => {
                let (run, receipt) = self.medagent(
                    &req.vault_id,
                    req.realm_id,
                    req.authority_scope_id,
                    req.session_id,
                    |mut medagent| {
                        medagent.fail_agent_run(&run_id, expected_revision, failure_reason)
                    },
                )?;
                Ok(ResponseBody::MedAgentRunTerminal {
                    run: Box::new(run),
                    receipt: Box::new(receipt),
                })
            }
        }
    }
}

fn blob_lookup(
    vault: Option<&SyntheticVault>,
    digest: &medscale_contracts::objects::DigestSha256,
) -> Option<(Vec<u8>, bool)> {
    let vault = vault?;
    match vault.blobs.get_blob(digest) {
        Ok(bytes) => {
            let ok = vault.blobs.verify(digest, bytes.len() as u64).is_ok();
            Some((bytes, ok))
        }
        Err(_) => None,
    }
}

fn capability_matches(cap: &Capability, body: &RequestBody) -> bool {
    matches!(
        (cap, body),
        (Capability::AcquireLease, RequestBody::AcquireLease { .. })
            | (Capability::ReleaseLease, RequestBody::ReleaseLease { .. })
            | (Capability::Ping, RequestBody::Ping)
            | (
                Capability::CreateSourceRecord,
                RequestBody::CreateSourceRecord { .. }
            )
            | (
                Capability::CreateDerivedArtifact,
                RequestBody::CreateDerivedArtifact { .. }
            )
            | (
                Capability::CreateProposal,
                RequestBody::CreateProposal { .. }
            )
            | (
                Capability::PromoteProposal,
                RequestBody::PromoteProposal { .. }
            )
            | (
                Capability::AmendAssertion,
                RequestBody::AmendAssertion { .. }
            )
            | (
                Capability::CreateIdentityAssertion,
                RequestBody::CreateIdentityAssertion { .. }
            )
            | (
                Capability::DecideIdentityMerge,
                RequestBody::DecideIdentityMerge { .. }
            )
            | (Capability::AppendAudit, RequestBody::AppendAudit { .. })
            | (
                Capability::TransitionEffect,
                RequestBody::TransitionEffect { .. }
            )
            | (Capability::ReadObject, RequestBody::ReadObject { .. })
            | (
                Capability::OpenSyntheticVault,
                RequestBody::OpenSyntheticVault { .. }
            )
            | (Capability::CloseVault, RequestBody::CloseVault)
            | (
                Capability::IngestFhirSynthetic,
                RequestBody::IngestFhirSynthetic { .. }
            )
            | (
                Capability::AttachValidatorEvidence,
                RequestBody::AttachValidatorEvidence { .. }
            )
            | (
                Capability::RebuildProjection,
                RequestBody::RebuildProjection { .. }
            )
            | (
                Capability::ReadCanonicalVisibility,
                RequestBody::ReadCanonicalVisibility { .. }
            )
            | (Capability::VerifyBlob, RequestBody::VerifyBlob { .. })
            | (Capability::BackupVault, RequestBody::BackupVault { .. })
            | (Capability::RestoreVault, RequestBody::RestoreVault { .. })
            | (Capability::RunBlobGc, RequestBody::RunBlobGc)
            | (Capability::GetTimeline, RequestBody::GetTimeline { .. })
            | (Capability::GetBrief, RequestBody::GetBrief { .. })
            | (Capability::GetCoverage, RequestBody::GetCoverage { .. })
            | (
                Capability::DrillDownPresentation,
                RequestBody::DrillDownPresentation { .. }
            )
            | (
                Capability::CreateEncryptedVault,
                RequestBody::CreateEncryptedVault { .. }
            )
            | (
                Capability::OpenEncryptedVault,
                RequestBody::OpenEncryptedVault { .. }
            )
            | (
                Capability::CloseEncryptedVault,
                RequestBody::CloseEncryptedVault
            )
            | (
                Capability::NetworkBrokerInvoke,
                RequestBody::NetworkBrokerInvoke { .. }
            )
            | (
                Capability::SetEgressAllowlist,
                RequestBody::SetEgressAllowlist { .. }
            )
            | (
                Capability::PacksInstallLocal,
                RequestBody::PacksInstallLocal { .. }
            )
            | (Capability::PacksList, RequestBody::PacksList)
            | (Capability::PacksPromote, RequestBody::PacksPromote { .. })
            | (
                Capability::PacksEvaluateLocal,
                RequestBody::PacksEvaluateLocal { .. }
                    | RequestBody::PacksGenerateLocal { .. }
                    | RequestBody::PacksUnloadLocal { .. },
            )
            | (
                Capability::DocumentIntake,
                RequestBody::DocumentIntake { .. }
            )
            | (Capability::OcrStub, RequestBody::OcrStub { .. })
            | (Capability::AsrStub, RequestBody::AsrStub { .. })
            | (
                Capability::RetrieveLexical,
                RequestBody::RetrieveLexical { .. }
            )
            | (
                Capability::CreateExternalActionIntent,
                RequestBody::CreateExternalActionIntent { .. }
            )
            | (Capability::ListOutbox, RequestBody::ListOutbox)
            | (Capability::NphiesInvoke, RequestBody::NphiesInvoke { .. })
            | (
                Capability::OnlinePackAcquire,
                RequestBody::OnlinePackAcquire { .. }
            )
            | (
                Capability::MescArtifactAdmit,
                RequestBody::MescArtifactAdmit { .. }
            )
            | (
                Capability::MescArtifactVerify,
                RequestBody::MescArtifactVerify { .. }
            )
            | (Capability::OpenSession, RequestBody::OpenSession { .. })
            | (Capability::RevokeSession, RequestBody::RevokeSession { .. })
            | (
                Capability::GetFhirSupportMatrix,
                RequestBody::GetFhirSupportMatrix
            )
            | (
                Capability::ExportFhirLossAware,
                RequestBody::ExportFhirLossAware { .. }
            )
            | (
                Capability::RejectProposal,
                RequestBody::RejectProposal { .. }
            )
            | (
                Capability::AppendDisclosure,
                RequestBody::AppendDisclosure { .. }
            )
            | (Capability::ListDisclosures, RequestBody::ListDisclosures)
            | (Capability::ProjectCreate, RequestBody::ProjectCreate { .. })
            | (Capability::ProjectRead, RequestBody::ProjectGet { .. })
            | (Capability::ProjectRead, RequestBody::ProjectList { .. })
            | (Capability::ProjectUpdate, RequestBody::ProjectUpdate { .. })
            | (
                Capability::ProjectArchive,
                RequestBody::ProjectArchive { .. }
            )
            | (
                Capability::ProjectArchive,
                RequestBody::ProjectRestore { .. }
            )
            | (
                Capability::ExperimentCreate,
                RequestBody::ExperimentCreate { .. }
            )
            | (
                Capability::ExperimentRead,
                RequestBody::ExperimentGet { .. }
            )
            | (
                Capability::ExperimentRead,
                RequestBody::ExperimentList { .. }
            )
            | (
                Capability::ExperimentUpdate,
                RequestBody::ExperimentUpdate { .. }
            )
            | (
                Capability::ExperimentArchive,
                RequestBody::ExperimentArchive { .. }
            )
            | (
                Capability::ProjectArtifactAttach,
                RequestBody::ProjectAttach { .. }
            )
            | (
                Capability::ProjectArtifactDetach,
                RequestBody::ProjectDetach { .. }
            )
            | (Capability::ProjectRead, RequestBody::ProjectListRefs { .. })
            | (
                Capability::ProjectGraphMutate,
                RequestBody::GraphEdgeCreate { .. }
            )
            | (
                Capability::ProjectGraphMutate,
                RequestBody::GraphEdgeRemove { .. }
            )
            | (
                Capability::ProjectGraphRead,
                RequestBody::GraphNeighbors { .. }
            )
            | (
                Capability::ProjectRead,
                RequestBody::ProjectContextResolve { .. }
            )
            | (
                Capability::ProjectRead,
                RequestBody::ProjectSummaryQuery { .. }
            )
            | (
                Capability::DataSourceCreate,
                RequestBody::DataSourceCreate { .. }
            )
            | (
                Capability::DataSourceRead,
                RequestBody::DataSourceGet { .. }
            )
            | (
                Capability::DataSourceRead,
                RequestBody::DataSourceList { .. }
            )
            | (
                Capability::DataSourceUpdate,
                RequestBody::DataSourceUpdate { .. }
            )
            | (
                Capability::DataSourceArchive,
                RequestBody::DataSourceArchive { .. }
            )
            | (
                Capability::SnapshotImport,
                RequestBody::SnapshotImport { .. }
            )
            | (
                Capability::SnapshotPreview,
                RequestBody::SnapshotPreview { .. }
            )
            | (Capability::SnapshotRead, RequestBody::SnapshotGet { .. })
            | (Capability::SnapshotRead, RequestBody::SnapshotList { .. })
            | (Capability::SnapshotRead, RequestBody::SnapshotRows { .. })
            | (
                Capability::SnapshotRefresh,
                RequestBody::SnapshotRefresh { .. }
            )
            | (
                Capability::SavedViewCreate,
                RequestBody::SavedViewCreate { .. }
            )
            | (Capability::SavedViewRead, RequestBody::SavedViewGet { .. })
            | (Capability::SavedViewRead, RequestBody::SavedViewList { .. })
            | (
                Capability::SavedViewUpdate,
                RequestBody::SavedViewUpdate { .. }
            )
            | (
                Capability::TransformExecute,
                RequestBody::TransformExecute { .. }
            )
            | (
                Capability::DatasetReleaseCreate,
                RequestBody::DatasetReleaseCreate { .. }
            )
            | (
                Capability::DatasetReleaseRead,
                RequestBody::DatasetReleaseGet { .. }
            )
            | (
                Capability::DatasetReleaseRead,
                RequestBody::DatasetReleaseList { .. }
            )
            | (
                Capability::ParticipantRegister,
                RequestBody::ParticipantRegister { .. }
            )
            | (
                Capability::ParticipantRead,
                RequestBody::ParticipantGet { .. }
            )
            | (
                Capability::ParticipantRevoke,
                RequestBody::ParticipantRevoke { .. }
            )
            | (Capability::RoomCreate, RequestBody::RoomCreate { .. })
            | (Capability::RoomRead, RequestBody::RoomGet { .. })
            | (Capability::RoomRead, RequestBody::RoomList { .. })
            | (Capability::RoomUpdate, RequestBody::RoomRename { .. })
            | (Capability::RoomArchive, RequestBody::RoomArchive { .. })
            | (
                Capability::RoomMembershipManage,
                RequestBody::RoomMembershipAdd { .. }
            )
            | (
                Capability::RoomMembershipRead,
                RequestBody::RoomMembershipList { .. }
            )
            | (
                Capability::RoomMembershipManage,
                RequestBody::RoomMembershipRemove { .. }
            )
            | (Capability::ThreadCreate, RequestBody::ThreadOpen { .. })
            | (Capability::ThreadRead, RequestBody::ThreadGet { .. })
            | (Capability::ThreadRead, RequestBody::ThreadList { .. })
            | (
                Capability::ThreadResolve,
                RequestBody::ThreadSetStatus { .. }
            )
            | (Capability::MessagePost, RequestBody::MessagePost { .. })
            | (Capability::MessageRead, RequestBody::MessageList { .. })
            | (Capability::MessageEdit, RequestBody::MessageEditBody { .. })
            | (Capability::MessageEdit, RequestBody::MessageDelete { .. })
            | (Capability::TaskCreate, RequestBody::TaskCreate { .. })
            | (Capability::TaskRead, RequestBody::TaskGet { .. })
            | (Capability::TaskRead, RequestBody::TaskList { .. })
            | (Capability::TaskUpdate, RequestBody::TaskUpdate { .. })
            | (Capability::NoteCreate, RequestBody::NoteCreate { .. })
            | (Capability::NoteRead, RequestBody::NoteGet { .. })
            | (Capability::NoteRead, RequestBody::NoteListRevisions { .. })
            | (Capability::NoteUpdate, RequestBody::NoteEdit { .. })
            | (
                Capability::ApprovalRequestCreate,
                RequestBody::ApprovalRequestCreate { .. }
            )
            | (
                Capability::ApprovalRequestRead,
                RequestBody::ApprovalRequestGet { .. }
            )
            | (
                Capability::ApprovalWithdraw,
                RequestBody::ApprovalRequestWithdraw { .. }
            )
            | (
                Capability::ApprovalDecide,
                RequestBody::ApprovalDecide { .. }
            )
            | (
                Capability::ApprovalRequestRead,
                RequestBody::ApprovalDecisionList { .. }
            )
            | (Capability::ActivityRead, RequestBody::ActivityList { .. })
            | (
                Capability::AgentIdentityRegister,
                RequestBody::AgentIdentityRegister { .. }
            )
            | (
                Capability::AgentIdentityRead,
                RequestBody::AgentIdentityGet { .. }
            )
            | (
                Capability::AgentIdentityRead,
                RequestBody::AgentIdentityList { .. }
            )
            | (
                Capability::AgentIdentityRevoke,
                RequestBody::AgentIdentityRevoke { .. }
            )
            | (
                Capability::ContextManifestCreate,
                RequestBody::ContextManifestCreate { .. }
            )
            | (
                Capability::ContextManifestRead,
                RequestBody::ContextManifestGet { .. }
            )
            | (
                Capability::AgentRunCreate,
                RequestBody::AgentRunCreate { .. }
            )
            | (Capability::AgentRunRead, RequestBody::AgentRunGet { .. })
            | (Capability::AgentRunRead, RequestBody::AgentRunList { .. })
            | (Capability::AgentRunStart, RequestBody::AgentRunStart { .. })
            | (
                Capability::AgentRunCancel,
                RequestBody::AgentRunCancel { .. }
            )
            | (
                Capability::AgentRunRead,
                RequestBody::AgentRunTurnList { .. }
            )
            | (
                Capability::AgentToolInvoke,
                RequestBody::AgentToolInvoke { .. }
            )
            | (
                Capability::AgentRunExecute,
                RequestBody::AgentRunExecute { .. }
            )
            | (
                Capability::AgentRunComplete,
                RequestBody::AgentRunComplete { .. }
            )
            | (Capability::AgentRunFail, RequestBody::AgentRunFail { .. })
            | (
                Capability::AgentLaneCreate,
                RequestBody::AgentLaneCreate { .. }
            )
            | (Capability::AgentLaneRead, RequestBody::AgentLaneGet { .. })
            | (Capability::AgentLaneRead, RequestBody::AgentLaneList { .. })
            | (
                Capability::AgentLaneRetire,
                RequestBody::AgentLaneRetire { .. }
            )
            | (
                Capability::FleetRunCreate,
                RequestBody::FleetRunCreate { .. }
            )
            | (Capability::FleetRunRead, RequestBody::FleetRunGet { .. })
            | (Capability::FleetRunRead, RequestBody::FleetRunList { .. })
            | (
                Capability::FleetRunDispatch,
                RequestBody::FleetRunDispatch { .. }
            )
            | (
                Capability::FleetRunExecuteLane,
                RequestBody::FleetRunExecuteLane { .. }
            )
            | (
                Capability::FleetRunCancel,
                RequestBody::FleetRunCancel { .. }
            )
            | (
                Capability::ComparisonCompute,
                RequestBody::ComparisonCompute { .. }
            )
            | (
                Capability::ComparisonRead,
                RequestBody::ComparisonReportList { .. }
            )
            | (
                Capability::PrivacyClassify,
                RequestBody::PrivacyClassify { .. }
            )
            | (
                Capability::PrivacyRead,
                RequestBody::PrivacyClassificationGet { .. }
                    | RequestBody::PrivacyClassificationList { .. }
                    | RequestBody::PrivacyProfileGet { .. }
                    | RequestBody::PrivacyProfileList { .. }
                    | RequestBody::PrivacyMapList { .. }
                    | RequestBody::PrivacyReceiptGet { .. }
                    | RequestBody::PrivacyReceiptList { .. }
                    | RequestBody::PrivacyReidentificationAuditList { .. }
                    | RequestBody::PrivacyEgressDecisionList { .. }
            )
            | (
                Capability::PrivacyProfileCreate,
                RequestBody::PrivacyProfileCreate { .. }
            )
            | (
                Capability::PrivacyProfileRevoke,
                RequestBody::PrivacyProfileRevoke { .. }
            )
            | (
                Capability::PrivacyTransform,
                RequestBody::PrivacyTransform { .. }
            )
            | (
                Capability::PrivacyReceiptRevoke,
                RequestBody::PrivacyReceiptRevoke { .. }
            )
            | (
                Capability::PrivacyMapCreate,
                RequestBody::PrivacyMapCreate { .. }
            )
            | (
                Capability::PrivacyMapRevoke,
                RequestBody::PrivacyMapRevoke { .. }
            )
            | (
                Capability::PrivacyReidentify,
                RequestBody::PrivacyReidentify { .. }
            )
            | (
                Capability::PrivacyEgressEvaluate,
                RequestBody::PrivacyEgressEvaluate { .. }
            )
            | (
                Capability::BrowseAllowlistManage,
                RequestBody::BrowseAllowlistAdd { .. } | RequestBody::BrowseAllowlistDisable { .. }
            )
            | (
                Capability::BrowseRead,
                RequestBody::BrowseAllowlistList { .. }
                    | RequestBody::BrowseRouteList
                    | RequestBody::BrowseSessionGet { .. }
                    | RequestBody::BrowseSessionList { .. }
            )
            | (Capability::BrowseRun, RequestBody::BrowseRun { .. })
            | (
                Capability::BrowseCancel,
                RequestBody::BrowseSessionCancel { .. }
            )
            | (Capability::AudioImport, RequestBody::AudioImport { .. })
            | (
                Capability::AudioRead,
                RequestBody::AudioSourceGet { .. }
                    | RequestBody::AudioSourceList { .. }
                    | RequestBody::AudioCaptureGet { .. }
                    | RequestBody::AudioCaptureList { .. }
                    | RequestBody::AudioRouteList
                    | RequestBody::AudioTranscriptList { .. }
                    | RequestBody::AudioTranscriptGet { .. }
                    | RequestBody::AudioReceiptList { .. }
                    | RequestBody::AudioEvidenceGet { .. }
            )
            | (
                Capability::AudioCapture,
                RequestBody::AudioCaptureStart { .. }
                    | RequestBody::AudioCaptureAppend { .. }
                    | RequestBody::AudioCaptureTransition { .. }
                    | RequestBody::AudioCaptureStop { .. }
            )
            | (
                Capability::AudioTranscribe,
                RequestBody::AudioTranscribe { .. }
            )
            | (
                Capability::AudioCorrect,
                RequestBody::AudioTranscriptCorrect { .. }
            )
            | (
                Capability::AnalyticsQuery,
                RequestBody::AnalyticsQuery { .. }
            )
            | (
                Capability::AnalyticsRead,
                RequestBody::AnalyticsReplay { .. }
                    | RequestBody::AnalyticsReceiptGet { .. }
                    | RequestBody::AnalyticsReceiptList { .. }
                    | RequestBody::AnalyticsResultGet { .. }
                    | RequestBody::AnalyticsStatistics { .. }
                    | RequestBody::AnalyticsCohortList { .. }
            )
            | (
                Capability::AnalyticsCohort,
                RequestBody::AnalyticsCohortCreate { .. } | RequestBody::AnalyticsCohortRun { .. }
            )
            | (
                Capability::KnowledgeIndex,
                RequestBody::KnowledgeIndexBuild { .. }
            )
            | (
                Capability::KnowledgeSearch,
                RequestBody::KnowledgeSearch { .. }
            )
            | (
                Capability::KnowledgeRead,
                RequestBody::KnowledgeIndexStatus { .. }
                    | RequestBody::KnowledgeReceiptGet { .. }
                    | RequestBody::KnowledgeReceiptList { .. }
                    | RequestBody::CanvasGet { .. }
                    | RequestBody::CanvasList { .. }
            )
            | (
                Capability::KnowledgeCanvas,
                RequestBody::CanvasCreate { .. } | RequestBody::CanvasEdit { .. }
            )
            | (
                Capability::HubAdmin,
                RequestBody::HubInit
                    | RequestBody::HubInvite { .. }
                    | RequestBody::HubInvitationRevoke { .. }
                    | RequestBody::HubDeviceRevoke { .. }
            )
            | (
                Capability::HubRead,
                RequestBody::HubStatus
                    | RequestBody::HubLinkList
                    | RequestBody::HubLinkGet { .. }
                    | RequestBody::HubOutboxList { .. }
                    | RequestBody::HubMirrorList { .. }
            )
            | (
                Capability::HubBootstrap,
                RequestBody::HubEnroll { .. }
                    | RequestBody::HubChallenge { .. }
                    | RequestBody::HubHandshake { .. }
            )
            | (
                Capability::HubSync,
                RequestBody::HubSubmit { .. } | RequestBody::HubPull { .. }
            )
            | (
                Capability::HubClient,
                RequestBody::HubJoinPrepare { .. }
                    | RequestBody::HubJoinComplete { .. }
                    | RequestBody::HubQueue { .. }
                    | RequestBody::HubSignHandshake { .. }
                    | RequestBody::HubRecordOutcomes { .. }
                    | RequestBody::HubMirrorAppend { .. }
            )
            | (
                Capability::ComputeSubmit,
                RequestBody::ComputeSubmit { .. } | RequestBody::ComputeCancel { .. }
            )
            | (
                Capability::ComputeRun,
                RequestBody::ComputeRun { .. } | RequestBody::ComputeRecover
            )
            | (
                Capability::ComputeRead,
                RequestBody::ComputeJobGet { .. }
                    | RequestBody::ComputeJobList { .. }
                    | RequestBody::ComputeStatus
            )
            | (
                Capability::RWorkspaceStage,
                RequestBody::RWorkspaceStage { .. } | RequestBody::RWorkspaceLaunch { .. }
            )
            | (Capability::RWorkspaceRun, RequestBody::RWorkspaceRun { .. })
            | (
                Capability::ExtensionAdmin,
                RequestBody::ExtensionTrustPublisher { .. }
                    | RequestBody::ExtensionRevokePublisher { .. }
                    | RequestBody::ExtensionRevokeRelease { .. }
                    | RequestBody::ExtensionInstall { .. }
                    | RequestBody::ExtensionRollback { .. }
                    | RequestBody::ExtensionSetEnabled { .. }
                    | RequestBody::ExtensionUninstall { .. }
                    | RequestBody::ExtensionGrant { .. }
            )
            | (
                Capability::ExtensionInvoke,
                RequestBody::ExtensionInvoke { .. }
            )
            | (Capability::ExtensionRead, RequestBody::ExtensionList { .. })
            | (Capability::HuddleAct, RequestBody::HuddleAct { .. })
            | (Capability::PackAdmin, RequestBody::PackAct { .. })
            | (Capability::AdapterAdmin, RequestBody::AdapterAct { .. })
            | (
                Capability::AdapterRead,
                RequestBody::AdapterGet { .. } | RequestBody::AdapterList { .. }
            )
            | (
                Capability::FederationAdmin,
                RequestBody::FederationAct { .. }
            )
            | (Capability::FederationRead, RequestBody::FederationGet)
            | (
                Capability::ModelCatalogRead,
                RequestBody::ModelCatalogQuery { .. }
            )
            | (
                Capability::PackRead,
                RequestBody::PackCatalog
                    | RequestBody::PacksResidency
                    | RequestBody::PackInstallGet { .. }
                    | RequestBody::PackArtifactGet { .. }
                    | RequestBody::PackArtifactList { .. }
            )
            | (
                Capability::HuddleRead,
                RequestBody::HuddleGet { .. } | RequestBody::HuddleList { .. }
            )
            | (
                Capability::RWorkspacePublish,
                RequestBody::RWorkspacePublish { .. }
            )
            | (
                Capability::RWorkspaceRead,
                RequestBody::RWorkspaceInspect { .. }
                    | RequestBody::RWorkspaceGet { .. }
                    | RequestBody::RWorkspaceList { .. }
                    | RequestBody::RWorkspacePublished { .. }
                    | RequestBody::RWorkspaceStatus
            )
    )
}

fn enc_err(err: medscale_storage::EncryptedVaultError) -> AuthorityError {
    match err {
        medscale_storage::EncryptedVaultError::Claim(
            medscale_storage::ClaimError::SyncRootRefused(_),
        )
        | medscale_storage::EncryptedVaultError::Claim(medscale_storage::ClaimError::Escape) => {
            AuthorityError::PathOutsideClaim
        }
        medscale_storage::EncryptedVaultError::MissingKeyMaterial => {
            AuthorityError::MissingKeyMaterial
        }
        medscale_storage::EncryptedVaultError::LeaseHeld(holder) => AuthorityError::LeaseHeld {
            holder_id: OpaqueId::new(holder),
        },
        other => AuthorityError::InvalidArgument {
            message: other.to_string(),
        },
    }
}

fn lease_err(err: LeaseError) -> AuthorityError {
    match err {
        LeaseError::AlreadyHeld { holder_id } => AuthorityError::AlreadyHeld { holder_id },
        LeaseError::NotHolder => AuthorityError::NotHolder,
        LeaseError::NotHeld => AuthorityError::NotHeld,
    }
}

fn scope_err(err: ScopeError) -> AuthorityError {
    match err {
        ScopeError::NotFound => AuthorityError::NotFound,
        ScopeError::WrongScope => AuthorityError::WrongScope,
    }
}

fn promote_err(err: PromoteError) -> AuthorityError {
    match err {
        PromoteError::NotFound => AuthorityError::NotFound,
        PromoteError::WrongScope => AuthorityError::WrongScope,
    }
}

fn amend_err(err: super::amend::AmendError) -> AuthorityError {
    match err {
        super::amend::AmendError::NotFound => AuthorityError::NotFound,
        super::amend::AmendError::WrongScope => AuthorityError::WrongScope,
        super::amend::AmendError::AlreadySuperseded => AuthorityError::IllegalTransition,
        super::amend::AmendError::InvalidTime(message) => {
            AuthorityError::InvalidArgument { message }
        }
    }
}

/// `(key, bytes, pinned)` of every resident entry, owned so the pool lock is
/// released before the caller reads the Pack store.
fn resident_rows<T>(pool: &medscale_pack::ResidencyPool<T>) -> Vec<(String, u64, bool)> {
    pool.resident()
        .into_iter()
        .map(|(key, bytes, pinned)| (key.to_owned(), bytes, pinned))
        .collect()
}

/// Unloads `target` (a content-digest key) or, for `None`, every unpinned
/// entry. Pinned entries are kept. Returns the keys unloaded.
fn unload_from<T>(pool: &mut medscale_pack::ResidencyPool<T>, target: Option<&str>) -> Vec<String> {
    match target {
        Some(key) => pool
            .unload(key)
            .ok()
            .map(|_| vec![key.to_owned()])
            .unwrap_or_default(),
        None => pool.unload_all_unpinned(),
    }
}

fn digest_from_hex(hex: &str) -> Option<medscale_contracts::objects::DigestSha256> {
    if hex.len() != 64 {
        return None;
    }
    let mut bytes = [0u8; 32];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hex.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(medscale_contracts::objects::DigestSha256::from_bytes(bytes))
}
