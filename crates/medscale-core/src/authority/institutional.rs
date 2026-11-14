//! Institutional adapter authority (Spec 090, first path: object storage).
//!
//! Core owns every step of an external write. An adapter is registered
//! per Project with a declared destination, data-class ceiling,
//! capabilities and credential handle. A write starts as a `pending`
//! intent bound to the exact payload bytes of one Core artifact, whose
//! Spec 079 effective class must be within the ceiling (unclassified data
//! is `local_phi` and never leaves). Sending marks the intent `sent`
//! durably before calling the transport; the transport's answer moves it
//! to `confirmed`, `failed` (nothing stored) or `unknown` (sent, answer
//! lost). `unknown` is never retried blindly: reconciliation asks the
//! destination about the exact key and digest. Every step, applied or
//! refused, leaves a receipt.

use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::institutional::{
    AdapterActRequest, AdapterActResult, AdapterAction, AdapterCapability, AdapterConfig,
    AdapterReceipt, AdapterState, AdapterView, ExternalWriteIntent, INSTITUTIONAL_SCHEMA_VERSION,
    InstitutionalAdapter, PAYLOAD_BYTES_MAX, TransportOutcome, WriteRefusal, idempotency_key,
    validate_object_key,
};
use medscale_contracts::objects::{DigestSha256, EffectState, ObjectHeader, OpaqueId};
use medscale_contracts::privacy_gate::DataClass;
use medscale_storage::{AdapterChange, MetaError};

use super::data_sources::DataSources;
use crate::institutional_transport::InstitutionalTransport;

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

fn invalid(message: impl Into<String>) -> AuthorityError {
    AuthorityError::InvalidArgument {
        message: message.into(),
    }
}

impl DataSources<'_> {
    fn ia_header(&self, id: OpaqueId) -> ObjectHeader {
        ObjectHeader {
            id,
            schema_version: INSTITUTIONAL_SCHEMA_VERSION,
            realm_id: self.realm.clone(),
            authority_scope_id: self.scope.clone(),
        }
    }

    fn ia_alloc(&self, prefix: &str) -> Result<OpaqueId, AuthorityError> {
        self.meta().alloc_institutional_id(prefix).map_err(meta_err)
    }

    fn ia_adapter(&self, id: &OpaqueId) -> Result<InstitutionalAdapter, AuthorityError> {
        let a = self
            .meta()
            .get_institutional_adapter(id)
            .map_err(meta_err)?;
        if a.header.realm_id != self.realm || a.header.authority_scope_id != self.scope {
            return Err(AuthorityError::WrongScope);
        }
        Ok(a)
    }

    fn ia_intent(&self, id: &OpaqueId) -> Result<ExternalWriteIntent, AuthorityError> {
        let i = self.meta().get_write_intent(id).map_err(meta_err)?;
        if i.header.realm_id != self.realm || i.header.authority_scope_id != self.scope {
            return Err(AuthorityError::WrongScope);
        }
        Ok(i)
    }

    fn ia_receipt(
        &self,
        adapter: &InstitutionalAdapter,
        intent: Option<&ExternalWriteIntent>,
        action: AdapterAction,
    ) -> Result<AdapterReceipt, AuthorityError> {
        Ok(AdapterReceipt {
            header: self.ia_header(self.ia_alloc("adapter-receipt")?),
            project_id: adapter.project_id.clone(),
            adapter_id: adapter.header.id.clone(),
            intent_id: intent.map(|i| i.header.id.clone()),
            action,
            from_state: intent.map(|i| i.state),
            to_state: None,
            refusal: None,
            transport_outcome: None,
        })
    }

    fn ia_commit(&mut self, change: AdapterChange<'_>, audit: &str) -> Result<(), AuthorityError> {
        let target = change.receipt.map(|r| r.header.id.clone());
        self.meta()
            .commit_adapter_change(&change)
            .map_err(meta_err)?;
        self.audit(audit, target.into_iter().collect())
    }

    fn ia_refuse(
        &mut self,
        adapter: &InstitutionalAdapter,
        mut receipt: AdapterReceipt,
        refusal: WriteRefusal,
    ) -> Result<AdapterActResult, AuthorityError> {
        receipt.refusal = Some(refusal);
        self.ia_commit(
            AdapterChange {
                receipt: Some(&receipt),
                ..AdapterChange::default()
            },
            "institutional.refused",
        )?;
        Ok(AdapterActResult {
            adapter: Some(adapter.clone()),
            intent: None,
            receipt,
        })
    }

    /// The exact bytes and effective class of a Core source or derived
    /// artifact in `project_id`.
    fn ia_payload(
        &self,
        project_id: &OpaqueId,
        artifact_id: &OpaqueId,
    ) -> Option<(Vec<u8>, DataClass)> {
        let bytes = match self
            .store
            .get_scoped(artifact_id, &self.realm, &self.scope)
            .ok()?
        {
            super::store::StoredObject::Source(s) => s.bytes.clone(),
            super::store::StoredObject::Derived(d) => d.bytes.clone(),
            _ => return None,
        };
        let class = match self.meta().get_classification(project_id, artifact_id) {
            Ok(Some(row)) => row.data_class,
            _ => DataClass::LocalPhi,
        };
        Some((bytes, class))
    }

    fn ia_adapter_change(
        &mut self,
        old: &InstitutionalAdapter,
        next: InstitutionalAdapter,
        action: AdapterAction,
    ) -> Result<AdapterActResult, AuthorityError> {
        let receipt = self.ia_receipt(old, None, action)?;
        self.ia_commit(
            AdapterChange {
                adapter: Some((&next, Some(old.revision))),
                intent: None,
                receipt: Some(&receipt),
            },
            "institutional.adapter",
        )?;
        Ok(AdapterActResult {
            adapter: Some(next),
            intent: None,
            receipt,
        })
    }

    pub fn adapter_register(
        &mut self,
        project_id: OpaqueId,
        name: String,
        config: AdapterConfig,
    ) -> Result<AdapterActResult, AuthorityError> {
        self.require_project(&project_id)?;
        config.validate().map_err(invalid)?;
        let adapter = InstitutionalAdapter {
            header: self.ia_header(self.ia_alloc("institutional-adapter")?),
            project_id,
            name,
            config,
            config_revision: 1,
            previous_config: None,
            state: AdapterState::Active,
            revision: 1,
        };
        adapter.validate().map_err(invalid)?;
        let receipt = self.ia_receipt(&adapter, None, AdapterAction::Register)?;
        self.ia_commit(
            AdapterChange {
                adapter: Some((&adapter, None)),
                intent: None,
                receipt: Some(&receipt),
            },
            "institutional.register",
        )?;
        Ok(AdapterActResult {
            adapter: Some(adapter),
            intent: None,
            receipt,
        })
    }

    fn ia_not_revoked(&self, a: &InstitutionalAdapter) -> Result<(), AuthorityError> {
        if a.state == AdapterState::Revoked {
            return Err(AuthorityError::Conflict {
                message: "adapter is revoked".to_owned(),
            });
        }
        Ok(())
    }

    pub fn adapter_reconfigure(
        &mut self,
        adapter_id: &OpaqueId,
        config: AdapterConfig,
    ) -> Result<AdapterActResult, AuthorityError> {
        let old = self.ia_adapter(adapter_id)?;
        self.ia_not_revoked(&old)?;
        config.validate().map_err(invalid)?;
        let next = InstitutionalAdapter {
            previous_config: Some(old.config.clone()),
            config,
            config_revision: old.config_revision + 1,
            revision: old.revision + 1,
            ..old.clone()
        };
        self.ia_adapter_change(&old, next, AdapterAction::Reconfigure)
    }

    /// Returns to the previous configuration (as a new revision).
    pub fn adapter_rollback(
        &mut self,
        adapter_id: &OpaqueId,
    ) -> Result<AdapterActResult, AuthorityError> {
        let old = self.ia_adapter(adapter_id)?;
        self.ia_not_revoked(&old)?;
        let Some(previous) = old.previous_config.clone() else {
            return Err(AuthorityError::Conflict {
                message: "no previous configuration".to_owned(),
            });
        };
        let next = InstitutionalAdapter {
            config: previous,
            previous_config: None,
            config_revision: old.config_revision + 1,
            revision: old.revision + 1,
            ..old.clone()
        };
        self.ia_adapter_change(&old, next, AdapterAction::Rollback)
    }

    pub fn adapter_set_state(
        &mut self,
        adapter_id: &OpaqueId,
        state: AdapterState,
    ) -> Result<AdapterActResult, AuthorityError> {
        let old = self.ia_adapter(adapter_id)?;
        self.ia_not_revoked(&old)?;
        let action = match state {
            AdapterState::Active => AdapterAction::Resume,
            AdapterState::Suspended => AdapterAction::Suspend,
            AdapterState::Revoked => AdapterAction::Revoke,
        };
        let next = InstitutionalAdapter {
            state,
            revision: old.revision + 1,
            ..old.clone()
        };
        self.ia_adapter_change(&old, next, action)
    }

    /// Records a `pending` write of one artifact's exact bytes.
    pub fn adapter_intend(
        &mut self,
        adapter_id: &OpaqueId,
        artifact_id: &OpaqueId,
        object_key: String,
    ) -> Result<AdapterActResult, AuthorityError> {
        let adapter = self.ia_adapter(adapter_id)?;
        let receipt = self.ia_receipt(&adapter, None, AdapterAction::Intend)?;
        if adapter.state != AdapterState::Active {
            return self.ia_refuse(&adapter, receipt, WriteRefusal::AdapterNotActive);
        }
        if !adapter
            .config
            .capabilities
            .contains(&AdapterCapability::PutObject)
        {
            return self.ia_refuse(&adapter, receipt, WriteRefusal::CapabilityMissing);
        }
        if validate_object_key(&object_key).is_err() {
            return self.ia_refuse(&adapter, receipt, WriteRefusal::BadObjectKey);
        }
        let Some((bytes, class)) = self.ia_payload(&adapter.project_id, artifact_id) else {
            return self.ia_refuse(&adapter, receipt, WriteRefusal::PayloadMissing);
        };
        if bytes.len() as u64 > PAYLOAD_BYTES_MAX {
            return self.ia_refuse(&adapter, receipt, WriteRefusal::PayloadTooLarge);
        }
        if !adapter.config.admits(class) {
            return self.ia_refuse(&adapter, receipt, WriteRefusal::DataClassAboveCeiling);
        }
        let payload_digest = DigestSha256::of(&bytes);
        let intent = ExternalWriteIntent {
            header: self.ia_header(self.ia_alloc("write-intent")?),
            project_id: adapter.project_id.clone(),
            adapter_id: adapter.header.id.clone(),
            config_revision: adapter.config_revision,
            destination: adapter.config.destination.clone(),
            idempotency_key: idempotency_key(
                &adapter.header.id,
                &adapter.config.destination,
                &object_key,
                &payload_digest,
            ),
            object_key,
            artifact_id: artifact_id.clone(),
            payload_digest,
            payload_bytes: bytes.len() as u64,
            data_class: class,
            state: EffectState::Pending,
            attempts: 0,
            revision: 1,
        };
        let mut receipt = receipt;
        receipt.intent_id = Some(intent.header.id.clone());
        receipt.to_state = Some(EffectState::Pending);
        self.ia_commit(
            AdapterChange {
                adapter: None,
                intent: Some((&intent, None)),
                receipt: Some(&receipt),
            },
            "institutional.intend",
        )?;
        Ok(AdapterActResult {
            adapter: None,
            intent: Some(intent),
            receipt,
        })
    }

    fn ia_move(
        &mut self,
        adapter: &InstitutionalAdapter,
        old: &ExternalWriteIntent,
        to: EffectState,
        action: AdapterAction,
        outcome: Option<TransportOutcome>,
        attempts: u32,
    ) -> Result<AdapterActResult, AuthorityError> {
        let next = ExternalWriteIntent {
            state: to,
            attempts,
            revision: old.revision + 1,
            ..old.clone()
        };
        let mut receipt = self.ia_receipt(adapter, Some(old), action)?;
        receipt.to_state = Some(to);
        receipt.transport_outcome = outcome;
        self.ia_commit(
            AdapterChange {
                adapter: None,
                intent: Some((&next, Some(old.revision))),
                receipt: Some(&receipt),
            },
            "institutional.intent",
        )?;
        Ok(AdapterActResult {
            adapter: None,
            intent: Some(next),
            receipt,
        })
    }

    /// Recovers intents a crash left `sent`: they become `unknown`.
    pub fn adapter_recover(&mut self) -> Result<Vec<AdapterActResult>, AuthorityError> {
        let mut out = Vec::new();
        for intent in self.meta().list_write_intents(None).map_err(meta_err)? {
            if intent.state != EffectState::Sent
                || intent.header.realm_id != self.realm
                || intent.header.authority_scope_id != self.scope
            {
                continue;
            }
            let adapter = self.ia_adapter(&intent.adapter_id)?;
            out.push(self.ia_move(
                &adapter,
                &intent,
                EffectState::Unknown,
                AdapterAction::Reconcile,
                None,
                intent.attempts,
            )?);
        }
        Ok(out)
    }

    /// Sends a `pending` write once.
    pub fn adapter_send(
        &mut self,
        intent_id: &OpaqueId,
        transport: &dyn InstitutionalTransport,
    ) -> Result<AdapterActResult, AuthorityError> {
        self.adapter_recover()?;
        let intent = self.ia_intent(intent_id)?;
        let adapter = self.ia_adapter(&intent.adapter_id)?;
        let receipt = self.ia_receipt(&adapter, Some(&intent), AdapterAction::Send)?;
        match intent.state {
            EffectState::Pending => {}
            EffectState::Unknown | EffectState::Sent => {
                return self.ia_refuse(&adapter, receipt, WriteRefusal::UnknownRequiresReconcile);
            }
            EffectState::Confirmed => {
                return self.ia_refuse(&adapter, receipt, WriteRefusal::AlreadyConfirmed);
            }
            EffectState::Failed => {
                return Err(AuthorityError::IllegalTransition);
            }
        }
        if adapter.state != AdapterState::Active {
            return self.ia_refuse(&adapter, receipt, WriteRefusal::AdapterNotActive);
        }
        // Re-check the payload and its class at send time.
        let Some((bytes, class)) = self.ia_payload(&intent.project_id, &intent.artifact_id) else {
            return self.ia_refuse(&adapter, receipt, WriteRefusal::PayloadMissing);
        };
        if DigestSha256::of(&bytes) != intent.payload_digest {
            return self.ia_refuse(&adapter, receipt, WriteRefusal::PayloadMissing);
        }
        if !adapter.config.admits(class) || intent.destination != adapter.config.destination {
            return self.ia_refuse(&adapter, receipt, WriteRefusal::DataClassAboveCeiling);
        }
        // Durable `sent` before the call: a crash now recovers as unknown.
        let sent = self.ia_move(
            &adapter,
            &intent,
            EffectState::Sent,
            AdapterAction::Send,
            None,
            intent.attempts + 1,
        )?;
        let sent = sent.intent.ok_or_else(|| AuthorityError::Internal {
            message: "send produced no intent".to_owned(),
        })?;
        let outcome = transport.put(
            &sent.destination,
            &sent.object_key,
            &sent.idempotency_key,
            &bytes,
        );
        let to = match outcome {
            TransportOutcome::Stored | TransportOutcome::AlreadyStored => EffectState::Confirmed,
            TransportOutcome::Rejected | TransportOutcome::Unreachable => EffectState::Failed,
            _ => EffectState::Unknown,
        };
        self.ia_move(
            &adapter,
            &sent,
            to,
            AdapterAction::Send,
            Some(outcome),
            sent.attempts,
        )
    }

    /// Asks the destination about an `unknown` write.
    pub fn adapter_reconcile(
        &mut self,
        intent_id: &OpaqueId,
        transport: &dyn InstitutionalTransport,
    ) -> Result<AdapterActResult, AuthorityError> {
        self.adapter_recover()?;
        let intent = self.ia_intent(intent_id)?;
        let adapter = self.ia_adapter(&intent.adapter_id)?;
        if intent.state != EffectState::Unknown {
            return Err(AuthorityError::IllegalTransition);
        }
        let receipt = self.ia_receipt(&adapter, Some(&intent), AdapterAction::Reconcile)?;
        if !adapter
            .config
            .capabilities
            .contains(&AdapterCapability::HeadObject)
        {
            return self.ia_refuse(&adapter, receipt, WriteRefusal::CapabilityMissing);
        }
        let outcome = transport.head(
            &intent.destination,
            &intent.object_key,
            &intent.payload_digest,
        );
        let to = match outcome {
            TransportOutcome::PresentMatching => EffectState::Confirmed,
            // Not there: safe to send again (explicitly, as a new attempt).
            TransportOutcome::Absent => EffectState::Pending,
            // Different bytes under our key: never overwrite.
            TransportOutcome::PresentDifferent => EffectState::Failed,
            _ => {
                let mut r = receipt;
                r.transport_outcome = Some(outcome);
                return self.ia_refuse(&adapter, r, WriteRefusal::TransportUnavailable);
            }
        };
        self.ia_move(
            &adapter,
            &intent,
            to,
            AdapterAction::Reconcile,
            Some(outcome),
            intent.attempts,
        )
    }

    /// Re-arms a `failed` write as `pending` (explicit, never automatic).
    pub fn adapter_retry(
        &mut self,
        intent_id: &OpaqueId,
    ) -> Result<AdapterActResult, AuthorityError> {
        let intent = self.ia_intent(intent_id)?;
        let adapter = self.ia_adapter(&intent.adapter_id)?;
        if intent.state != EffectState::Failed {
            let receipt = self.ia_receipt(&adapter, Some(&intent), AdapterAction::Retry)?;
            let refusal = if intent.state == EffectState::Unknown {
                WriteRefusal::UnknownRequiresReconcile
            } else {
                WriteRefusal::AlreadyConfirmed
            };
            return self.ia_refuse(&adapter, receipt, refusal);
        }
        self.ia_move(
            &adapter,
            &intent,
            EffectState::Pending,
            AdapterAction::Retry,
            None,
            intent.attempts,
        )
    }

    pub fn adapter_act(
        &mut self,
        act: AdapterActRequest,
        transport: &dyn InstitutionalTransport,
    ) -> Result<AdapterActResult, AuthorityError> {
        match act {
            AdapterActRequest::Register {
                project_id,
                name,
                config,
            } => self.adapter_register(project_id, name, config),
            AdapterActRequest::Reconfigure { adapter_id, config } => {
                self.adapter_reconfigure(&adapter_id, config)
            }
            AdapterActRequest::Rollback { adapter_id } => self.adapter_rollback(&adapter_id),
            AdapterActRequest::SetState { adapter_id, state } => {
                self.adapter_set_state(&adapter_id, state)
            }
            AdapterActRequest::Intend {
                adapter_id,
                artifact_id,
                object_key,
            } => self.adapter_intend(&adapter_id, &artifact_id, object_key),
            AdapterActRequest::Send { intent_id } => self.adapter_send(&intent_id, transport),
            AdapterActRequest::Reconcile { intent_id } => {
                self.adapter_reconcile(&intent_id, transport)
            }
            AdapterActRequest::Retry { intent_id } => self.adapter_retry(&intent_id),
        }
    }

    /// Adapters of one Project in this realm and scope.
    pub fn adapter_list(
        &self,
        project_id: &OpaqueId,
    ) -> Result<Vec<InstitutionalAdapter>, AuthorityError> {
        self.require_project(project_id)?;
        Ok(self
            .meta()
            .list_institutional_adapters()
            .map_err(meta_err)?
            .into_iter()
            .filter(|a| {
                &a.project_id == project_id
                    && a.header.realm_id == self.realm
                    && a.header.authority_scope_id == self.scope
            })
            .collect())
    }

    pub fn adapter_view(&self, adapter_id: &OpaqueId) -> Result<AdapterView, AuthorityError> {
        let adapter = self.ia_adapter(adapter_id)?;
        Ok(AdapterView {
            intents: self
                .meta()
                .list_write_intents(Some(adapter_id))
                .map_err(meta_err)?,
            receipts: self
                .meta()
                .list_adapter_receipts(Some(adapter_id))
                .map_err(meta_err)?,
            adapter,
        })
    }
}
