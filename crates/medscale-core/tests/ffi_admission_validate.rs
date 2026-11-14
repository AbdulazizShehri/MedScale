use medscale_contracts::ffi_policy::FfiAdmissionRecord;
use medscale_contracts::objects::PlacementClass;
use medscale_core::validate::validate_ffi_admission;

#[test]
fn incomplete_ffi_record_fails() {
    let incomplete = FfiAdmissionRecord {
        component_name: "x".to_owned(),
        upstream_url: String::new(),
        revision: String::new(),
        build_flags: String::new(),
        abi_notes: String::new(),
        ownership_freeing: String::new(),
        thread_affinity: String::new(),
        no_panic_across_extern_c: false,
        typed_errors: false,
        allocator_assumptions: String::new(),
        arches: vec![],
        placement_class: PlacementClass::P1,
        fuzz_sanitizer_evidence: String::new(),
        sbom_path: String::new(),
        update_strategy: String::new(),
        exit_strategy: String::new(),
    };
    assert!(!validate_ffi_admission(&incomplete));
}

#[test]
fn complete_ffi_record_passes() {
    let complete = FfiAdmissionRecord {
        component_name: "example".to_owned(),
        upstream_url: "https://example.invalid/lib".to_owned(),
        revision: "abc".to_owned(),
        build_flags: "-O2".to_owned(),
        abi_notes: "C ABI".to_owned(),
        ownership_freeing: "caller frees".to_owned(),
        thread_affinity: "any".to_owned(),
        no_panic_across_extern_c: true,
        typed_errors: true,
        allocator_assumptions: "system".to_owned(),
        arches: vec!["x86_64".to_owned()],
        placement_class: PlacementClass::P2,
        fuzz_sanitizer_evidence: "none-yet".to_owned(),
        sbom_path: "sbom/example.json".to_owned(),
        update_strategy: "pin".to_owned(),
        exit_strategy: "remove crate".to_owned(),
    };
    assert!(validate_ffi_admission(&complete));
}
