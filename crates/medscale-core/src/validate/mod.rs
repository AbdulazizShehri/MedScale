//! FFI admission and worker policy validators.

use medscale_contracts::ffi_policy::FfiAdmissionRecord;
use medscale_contracts::worker_policy::WorkerSupervisionPolicy;

/// Validates an FFI admission record (incomplete → fail closed).
#[must_use]
pub fn validate_ffi_admission(record: &FfiAdmissionRecord) -> bool {
    record.is_complete()
}

/// Validates that worker policy remains deny-by-default for ambient privileges.
#[must_use]
pub fn validate_worker_policy_deny_default(policy: &WorkerSupervisionPolicy) -> bool {
    policy.ambient_denied()
}
