//! Deterministic clinical evidence checks for MedScale.
//!
//! The reference implementations live in the Python package
//! `integrations/clinical-evidence-python` (`medscale_clinical`), derived from
//! OpenMed under Apache-2.0. MedScale's Core can't run Python (founder decision
//! 2026-09-23), so each check is ported here and held to the reference by
//! conformance vectors that the Python package generates
//! (`tests/vectors/`). A port must reproduce the reference report byte for
//! byte, fingerprint included.
//!
//! Every check is advisory: it reports findings for a human reviewer and never
//! resolves them, reads source text, or reaches the network.

pub mod canonical;
pub mod evidence_span_audit;
pub mod summary_empty_evidence;
