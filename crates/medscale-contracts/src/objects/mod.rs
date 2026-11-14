//! Durable object class types and identifiers.

mod authority_classes;
mod eval_proj_audit;
mod identity;
mod ids;
mod record_semantics;
mod scope;
mod source;
mod time_effect;

pub use authority_classes::{ClinicalAssertion, ProducerKind, Proposal};
pub use eval_proj_audit::{ActionAuditKind, ActionAuditRecord, EvaluationRecord, Projection};
pub use identity::{IdentityAssertion, IdentityMergeDecision};
pub use ids::{DigestSha256, ObjectClass, ObjectHeader, OpaqueId, VaultId};
pub use record_semantics::{
    AmendmentKind, AmendmentRecord, IdentityReconciliationCandidate, IdentityUnresolvedSet,
    MissingnessKind, find_unresolved_identity_candidates,
};
pub use scope::{AuthorityScopeId, RealmId};
pub use source::{DerivedSourceArtifact, LossClass, RepresentationKind, SourceRecord};
pub use time_effect::{
    EffectState, MedicalTime, MedicalTimeError, PlacementClass, TimePrecision, TimeRole,
};
