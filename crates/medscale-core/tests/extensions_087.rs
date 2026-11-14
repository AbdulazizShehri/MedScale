//! Spec 087 Community Extensions Core authority integration tests.
//!
//! Every call goes through `CliSession` -> `CoreFacade::dispatch`. The
//! "third-party" extension is built and signed here with a freshly
//! generated publisher key, exactly as an external developer would with
//! the SDK helper. No extension code exists or runs: commands map to
//! read-only Host API operations executed by Core.

use medscale_contracts::data_sources::{LocalFileFormat, SourceLocator};
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::extensions::{
    EntrypointKind, ExtensionCapability, ExtensionCommand, ExtensionManifest, ExtensionPack,
    ExtensionRefusal, ExtensionVersion, HostOperation, InstallState, InvocationDenial,
};
use medscale_contracts::objects::OpaqueId;
use medscale_contracts::privacy_gate::DataClass;
use medscale_core::CliSession;
use medscale_core::authority::sign_extension_pack;
use medscale_keys::generate_device_key;

const LABS: &str = "id,age\n1,34\n2,71\n3,58\n";

struct Lab {
    s: CliSession,
    project: OpaqueId,
    snapshot: OpaqueId,
    secret: String,
    public: String,
}

fn setup(name: &str) -> Lab {
    let dir = std::env::temp_dir().join(format!("medscale-087c-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("labs.csv"), LABS).unwrap();
    let mut s = CliSession::connect(&format!("vault-087-{name}")).unwrap();
    s.open_synthetic_vault(&dir.display().to_string()).unwrap();
    let project = s
        .project_create("study".to_owned(), None)
        .unwrap()
        .header
        .id;
    let source = s
        .data_source_create(
            project.clone(),
            "labs".to_owned(),
            SourceLocator::LocalPath {
                path: "labs.csv".to_owned(),
                format: LocalFileFormat::Csv,
            },
            None,
        )
        .unwrap()
        .header
        .id;
    let snapshot = s.snapshot_import(source).unwrap().0.header.id;
    let (secret, public) = generate_device_key();
    Lab {
        s,
        project,
        snapshot,
        secret,
        public,
    }
}

fn manifest(lab: &Lab, minor: u32, caps: &[ExtensionCapability]) -> ExtensionManifest {
    let mut commands = vec![ExtensionCommand {
        name: "schema".to_owned(),
        title: "Show schema".to_owned(),
        operation: HostOperation::SnapshotSchema,
        max_rows: None,
    }];
    if caps.contains(&ExtensionCapability::SnapshotRowsRead) {
        commands.push(ExtensionCommand {
            name: "peek".to_owned(),
            title: "First rows".to_owned(),
            operation: HostOperation::SnapshotRows,
            max_rows: Some(2),
        });
    }
    ExtensionManifest {
        extension_id: "org.example.peek".to_owned(),
        version: ExtensionVersion {
            major: 1,
            minor,
            patch: 0,
        },
        publisher_id: "example".to_owned(),
        publisher_key_hex: lab.public.clone(),
        host_api_min: 1,
        host_api_max: 1,
        license: "Apache-2.0".to_owned(),
        description: "Sample third-party extension.".to_owned(),
        entrypoint: EntrypointKind::Declarative,
        capabilities: caps.to_vec(),
        commands,
    }
}

fn pack(lab: &Lab, m: &ExtensionManifest) -> String {
    serde_json::to_string(&sign_extension_pack(m, &lab.secret).unwrap()).unwrap()
}

fn trusted(name: &str) -> Lab {
    let mut lab = setup(name);
    let key = lab.public.clone();
    lab.s
        .ext_trust_publisher("example".to_owned(), key)
        .unwrap();
    lab
}

#[test]
fn a_sample_extension_installs_is_denied_until_granted_and_runs_through_core() {
    let mut lab = trusted("lifecycle");
    let v1 = manifest(&lab, 0, &[ExtensionCapability::SnapshotSchemaRead]);
    let installed = lab
        .s
        .ext_install(lab.project.clone(), pack(&lab, &v1))
        .unwrap();
    assert_eq!(installed.resulting_state, Some(InstallState::Enabled));

    // Default capabilities are empty.
    let denied = lab
        .s
        .ext_invoke(
            lab.project.clone(),
            "org.example.peek".to_owned(),
            "schema".to_owned(),
            Some(lab.snapshot.clone()),
        )
        .unwrap();
    assert_eq!(
        denied.receipt.denial,
        Some(InvocationDenial::CapabilityNotGranted)
    );
    assert!(denied.result.is_none());

    // A ceiling below the data's class still denies (snapshots are local_phi).
    lab.s
        .ext_grant(
            lab.project.clone(),
            "org.example.peek".to_owned(),
            ExtensionCapability::SnapshotSchemaRead,
            Some(DataClass::Public),
        )
        .unwrap();
    let low = lab
        .s
        .ext_invoke(
            lab.project.clone(),
            "org.example.peek".to_owned(),
            "schema".to_owned(),
            Some(lab.snapshot.clone()),
        )
        .unwrap();
    assert_eq!(
        low.receipt.denial,
        Some(InvocationDenial::DataClassAboveCeiling)
    );

    lab.s
        .ext_grant(
            lab.project.clone(),
            "org.example.peek".to_owned(),
            ExtensionCapability::SnapshotSchemaRead,
            Some(DataClass::LocalPhi),
        )
        .unwrap();
    let ran = lab
        .s
        .ext_invoke(
            lab.project.clone(),
            "org.example.peek".to_owned(),
            "schema".to_owned(),
            Some(lab.snapshot.clone()),
        )
        .unwrap();
    assert!(ran.receipt.denial.is_none(), "{ran:?}");
    assert_eq!(ran.result.as_ref().unwrap()["row_count"], 3);

    // A capability the release never declared cannot be granted, and an
    // undeclared command does not exist.
    let refused = lab
        .s
        .ext_grant(
            lab.project.clone(),
            "org.example.peek".to_owned(),
            ExtensionCapability::SnapshotRowsRead,
            Some(DataClass::LocalPhi),
        )
        .unwrap();
    assert!(refused.refusal.is_some());
    let unknown = lab
        .s
        .ext_invoke(
            lab.project.clone(),
            "org.example.peek".to_owned(),
            "peek".to_owned(),
            Some(lab.snapshot.clone()),
        )
        .unwrap();
    assert_eq!(
        unknown.receipt.denial,
        Some(InvocationDenial::UnknownCommand)
    );

    // Upgrade with a new capability needs re-consent.
    let v2 = manifest(
        &lab,
        1,
        &[
            ExtensionCapability::SnapshotSchemaRead,
            ExtensionCapability::SnapshotRowsRead,
        ],
    );
    let upgraded = lab
        .s
        .ext_upgrade(lab.project.clone(), pack(&lab, &v2))
        .unwrap();
    assert_eq!(upgraded.resulting_state, Some(InstallState::PendingConsent));
    assert_eq!(
        upgraded.capability_expansion,
        vec![ExtensionCapability::SnapshotRowsRead]
    );
    let pending = lab
        .s
        .ext_invoke(
            lab.project.clone(),
            "org.example.peek".to_owned(),
            "schema".to_owned(),
            Some(lab.snapshot.clone()),
        )
        .unwrap();
    assert_eq!(pending.receipt.denial, Some(InvocationDenial::NotEnabled));
    lab.s
        .ext_set_enabled(lab.project.clone(), "org.example.peek".to_owned(), true)
        .unwrap();
    lab.s
        .ext_grant(
            lab.project.clone(),
            "org.example.peek".to_owned(),
            ExtensionCapability::SnapshotRowsRead,
            Some(DataClass::LocalPhi),
        )
        .unwrap();
    let rows = lab
        .s
        .ext_invoke(
            lab.project.clone(),
            "org.example.peek".to_owned(),
            "peek".to_owned(),
            Some(lab.snapshot.clone()),
        )
        .unwrap();
    assert_eq!(
        rows.result.as_ref().unwrap()["rows"]
            .as_array()
            .unwrap()
            .len(),
        2,
        "bounded by max_rows"
    );

    // Downgrade is not an upgrade; rollback returns to v1.
    let again = lab
        .s
        .ext_upgrade(lab.project.clone(), pack(&lab, &v1))
        .unwrap();
    assert_eq!(again.refusal, Some(ExtensionRefusal::NotAnUpgrade));
    let back = lab
        .s
        .ext_rollback(lab.project.clone(), "org.example.peek".to_owned())
        .unwrap();
    assert_eq!(back.resulting_state, Some(InstallState::Enabled));
    let gone = lab
        .s
        .ext_invoke(
            lab.project.clone(),
            "org.example.peek".to_owned(),
            "peek".to_owned(),
            Some(lab.snapshot.clone()),
        )
        .unwrap();
    assert_eq!(gone.receipt.denial, Some(InvocationDenial::UnknownCommand));

    // Uninstall revokes grants; reinstall starts from nothing again.
    lab.s
        .ext_uninstall(lab.project.clone(), "org.example.peek".to_owned())
        .unwrap();
    lab.s
        .ext_install(lab.project.clone(), pack(&lab, &v1))
        .unwrap();
    let fresh = lab
        .s
        .ext_invoke(
            lab.project.clone(),
            "org.example.peek".to_owned(),
            "schema".to_owned(),
            Some(lab.snapshot.clone()),
        )
        .unwrap();
    assert_eq!(
        fresh.receipt.denial,
        Some(InvocationDenial::CapabilityNotGranted)
    );
}

#[test]
fn untrusted_forged_or_incompatible_packs_are_refused() {
    let mut lab = setup("verify");
    let m = manifest(&lab, 0, &[ExtensionCapability::SnapshotSchemaRead]);
    let good = pack(&lab, &m);
    let refusal = |lab: &mut Lab, pack_json: String| {
        lab.s
            .ext_install(lab.project.clone(), pack_json)
            .unwrap()
            .refusal
    };
    assert_eq!(
        refusal(&mut lab, good.clone()),
        Some(ExtensionRefusal::PublisherUnknown)
    );
    let key = lab.public.clone();
    lab.s
        .ext_trust_publisher("example".to_owned(), key)
        .unwrap();

    // Signed by someone else.
    let (other_secret, _) = generate_device_key();
    let forged = serde_json::to_string(&sign_extension_pack(&m, &other_secret).unwrap()).unwrap();
    assert_eq!(
        refusal(&mut lab, forged),
        Some(ExtensionRefusal::SignatureInvalid)
    );
    // Manifest edited after signing.
    let mut p: ExtensionPack = serde_json::from_str(&good).unwrap();
    p.manifest_json = p.manifest_json.replace("Sample", "Evil");
    assert_eq!(
        refusal(&mut lab, serde_json::to_string(&p).unwrap()),
        Some(ExtensionRefusal::SignatureInvalid)
    );
    // Unknown capability or executable entrypoint: fails closed.
    for (from, to) in [
        ("\"snapshot_schema_read\"", "\"network_fetch\""),
        ("\"declarative\"", "\"native\""),
    ] {
        let mut p: ExtensionPack = serde_json::from_str(&good).unwrap();
        p.manifest_json = p.manifest_json.replacen(from, to, 1);
        assert_eq!(
            refusal(&mut lab, serde_json::to_string(&p).unwrap()),
            Some(ExtensionRefusal::ManifestInvalid)
        );
    }
    // Not canonical JSON.
    let mut p: ExtensionPack = serde_json::from_str(&good).unwrap();
    p.manifest_json = format!(" {}", p.manifest_json);
    assert_eq!(
        refusal(&mut lab, serde_json::to_string(&p).unwrap()),
        Some(ExtensionRefusal::ManifestInvalid)
    );
    // Oversized.
    assert_eq!(
        refusal(&mut lab, "x".repeat(300 * 1024)),
        Some(ExtensionRefusal::PackTooLarge)
    );
    // A future Host API.
    let mut future = m.clone();
    future.host_api_min = 2;
    future.host_api_max = 2;
    let future_pack = pack(&lab, &future);
    assert_eq!(
        refusal(&mut lab, future_pack),
        Some(ExtensionRefusal::ApiIncompatible)
    );
    // Another key claiming the trusted publisher id.
    let (_, other_public) = generate_device_key();
    let mut impostor = m.clone();
    impostor.publisher_key_hex = other_public;
    let impostor_pack = pack(&lab, &impostor);
    assert_eq!(
        refusal(&mut lab, impostor_pack),
        Some(ExtensionRefusal::PublisherKeyMismatch)
    );
    assert_eq!(refusal(&mut lab, good.clone()), None);
    assert_eq!(
        refusal(&mut lab, good),
        Some(ExtensionRefusal::AlreadyInstalled)
    );
    assert!(matches!(
        lab.s.ext_trust_publisher("bad".to_owned(), "00".repeat(32)),
        Err(AuthorityError::InvalidArgument { .. })
    ));
}

#[test]
fn revocation_quarantines_and_nothing_crosses_projects() {
    let mut lab = trusted("revoke");
    let m = manifest(&lab, 0, &[ExtensionCapability::SnapshotSchemaRead]);
    let p = pack(&lab, &m);
    lab.s.ext_install(lab.project.clone(), p.clone()).unwrap();
    lab.s
        .ext_grant(
            lab.project.clone(),
            "org.example.peek".to_owned(),
            ExtensionCapability::SnapshotSchemaRead,
            Some(DataClass::LocalPhi),
        )
        .unwrap();

    // A second Project: the first Project's snapshot is not its target.
    let other = lab
        .s
        .project_create("other".to_owned(), None)
        .unwrap()
        .header
        .id;
    lab.s.ext_install(other.clone(), p.clone()).unwrap();
    lab.s
        .ext_grant(
            other.clone(),
            "org.example.peek".to_owned(),
            ExtensionCapability::SnapshotSchemaRead,
            Some(DataClass::LocalPhi),
        )
        .unwrap();
    let cross = lab
        .s
        .ext_invoke(
            other.clone(),
            "org.example.peek".to_owned(),
            "schema".to_owned(),
            Some(lab.snapshot.clone()),
        )
        .unwrap();
    assert_eq!(
        cross.receipt.denial,
        Some(InvocationDenial::TargetNotInProject)
    );

    let quarantined = lab.s.ext_revoke_publisher("example".to_owned()).unwrap();
    assert_eq!(quarantined.len(), 2);
    let after = lab
        .s
        .ext_invoke(
            lab.project.clone(),
            "org.example.peek".to_owned(),
            "schema".to_owned(),
            Some(lab.snapshot.clone()),
        )
        .unwrap();
    assert_eq!(after.receipt.denial, Some(InvocationDenial::NotEnabled));
    let enable = lab
        .s
        .ext_set_enabled(lab.project.clone(), "org.example.peek".to_owned(), true)
        .unwrap();
    assert!(enable.refusal.is_some(), "a quarantined install stays off");
    let reinstall = lab.s.ext_install(lab.project.clone(), p).unwrap();
    assert!(reinstall.refusal.is_some());
}
