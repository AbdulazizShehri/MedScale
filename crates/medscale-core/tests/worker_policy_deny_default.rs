use medscale_contracts::worker_policy::WorkerSupervisionPolicy;
use medscale_core::validate::validate_worker_policy_deny_default;

#[test]
fn deny_by_default_passes() {
    let policy = WorkerSupervisionPolicy::deny_by_default();
    assert!(validate_worker_policy_deny_default(&policy));
    assert!(policy.grants.is_empty());
}

#[test]
fn ambient_network_fails_validator() {
    let mut policy = WorkerSupervisionPolicy::deny_by_default();
    policy.allow_network = true;
    assert!(!validate_worker_policy_deny_default(&policy));
}
