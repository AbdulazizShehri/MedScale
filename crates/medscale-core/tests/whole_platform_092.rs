//! Spec 092 whole-platform qualification: one integrated campaign.
//!
//! A single synthetic vault goes through every Research OS plane via Core
//! (`CliSession` -> `CoreFacade`): Data Source snapshot (075), analytics
//! (082), knowledge index (083), compute (085), R workspace (086),
//! extensions (087), huddles (088), research packs (089), institutional
//! adapters (090) and federation (091). The vault is then backed up,
//! restored into a new directory, checked by every storage consistency
//! verifier, and reopened through Core, where the same state must be
//! readable and the restore's documented losses (the federation identity
//! secret) must hold. No network is used; no real data.

use std::sync::Arc;

use medscale_contracts::analytics::{QueryRequest, ViewBinding};
use medscale_contracts::audio::{AudioRoute, PcmFormat};
use medscale_contracts::compute::{ComputeJobRequest, ComputeParams, ComputeState};
use medscale_contracts::data_sources::{LocalFileFormat, SourceLocator};
use medscale_contracts::extensions::{
    EntrypointKind, ExtensionCapability, ExtensionCommand, ExtensionManifest, ExtensionVersion,
    HostOperation,
};
use medscale_contracts::federation::{FederationActRequest, FederationRefusal};
use medscale_contracts::huddles::{ConsentAct, HuddleActRequest, HuddleParticipantKind};
use medscale_contracts::institutional::{
    AdapterActRequest, AdapterCapability, AdapterConfig, AdapterKind,
};
use medscale_contracts::objects::{EffectState, OpaqueId};
use medscale_contracts::privacy_gate::DataClass;
use medscale_contracts::r_workspace::{PublishState, RPublishRequest, RStageRequest};
use medscale_contracts::research_packs::{CLINICAL_RESEARCH_PACK_ID, FieldValue, PackActRequest};
use medscale_core::CliSession;
use medscale_core::authority::sign_extension_pack;
use medscale_core::institutional_transport::InProcessStore;
use medscale_core::r_workspace_host::RWorkspaceHost;
use medscale_keys::generate_device_key;
use medscale_storage::{SyntheticVault, backup_vault, restore_vault};

const VAULT: &str = "vault-092-campaign";
const LABS: &str = "id,age,ldl\n1,34,3.1\n2,71,4.4\n3,58,2.9\n";

struct Ids {
    project: OpaqueId,
    snapshot: OpaqueId,
    compute_job: OpaqueId,
    r_workspace: OpaqueId,
    huddle: OpaqueId,
    adapter: OpaqueId,
}

#[allow(clippy::too_many_lines)]
fn exercise_every_plane(s: &mut CliSession, base: &std::path::Path) -> Ids {
    // 075: a local CSV source and an exact snapshot.
    let project = s
        .project_create("campaign".to_owned(), None)
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

    // 082: an analytics query pinned to the snapshot.
    let q = s
        .analytics_query(QueryRequest {
            project_id: project.clone(),
            sql: "SELECT COUNT(*) AS n FROM labs".to_owned(),
            bindings: vec![ViewBinding {
                alias: "labs".to_owned(),
                snapshot_id: snapshot.clone(),
            }],
            max_rows: None,
        })
        .unwrap();
    assert!(q.table.is_some(), "{q:?}");

    // 083: a lexical knowledge index.
    s.knowledge_index_build(project.clone()).unwrap();

    // 085: a closed-kind compute job in the MedScale worker.
    let job = s
        .compute_submit(ComputeJobRequest {
            project_id: project.clone(),
            snapshot_id: snapshot.clone(),
            params: ComputeParams::ColumnProfile {},
            sandbox: None,
            limits: None,
        })
        .unwrap()
        .job
        .header
        .id;
    let ran = s.compute_run(job.clone()).unwrap();
    assert_eq!(ran.job.state, ComputeState::Completed, "{ran:?}");

    // 086: stage outside the vault, publish an output explicitly.
    let stage = base.join("stage");
    std::fs::create_dir_all(&stage).unwrap();
    let mut host = RWorkspaceHost::platform_default();
    host.stage_dir = Some(stage);
    s.set_r_workspace_host(host);
    let ws = s
        .r_stage(RStageRequest {
            project_id: project.clone(),
            label: "campaign".to_owned(),
            snapshot_ids: vec![snapshot.clone()],
        })
        .unwrap();
    let out = std::path::PathBuf::from(&ws.manifest.stage_root).join("outputs");
    std::fs::write(out.join("n.csv"), "n\n3\n").unwrap();
    let published = s
        .r_publish(RPublishRequest {
            workspace_id: ws.id().clone(),
            output_name: "n.csv".to_owned(),
            expected_digest: None,
        })
        .unwrap();
    assert_eq!(published.receipt.state, PublishState::Published);

    // 087: a signed declarative extension, granted and invoked.
    let (secret, public) = generate_device_key();
    s.ext_trust_publisher("example".to_owned(), public.clone())
        .unwrap();
    let manifest = ExtensionManifest {
        extension_id: "org.example.schema".to_owned(),
        version: ExtensionVersion {
            major: 1,
            minor: 0,
            patch: 0,
        },
        publisher_id: "example".to_owned(),
        publisher_key_hex: public,
        host_api_min: 1,
        host_api_max: 1,
        license: "Apache-2.0".to_owned(),
        description: String::new(),
        entrypoint: EntrypointKind::Declarative,
        capabilities: vec![ExtensionCapability::SnapshotSchemaRead],
        commands: vec![ExtensionCommand {
            name: "schema".to_owned(),
            title: "Schema".to_owned(),
            operation: HostOperation::SnapshotSchema,
            max_rows: None,
        }],
    };
    let pack = serde_json::to_string(&sign_extension_pack(&manifest, &secret).unwrap()).unwrap();
    s.ext_install(project.clone(), pack).unwrap();
    s.ext_grant(
        project.clone(),
        "org.example.schema".to_owned(),
        ExtensionCapability::SnapshotSchemaRead,
        Some(DataClass::LocalPhi),
    )
    .unwrap();
    let inv = s
        .ext_invoke(
            project.clone(),
            "org.example.schema".to_owned(),
            "schema".to_owned(),
            Some(snapshot.clone()),
        )
        .unwrap();
    assert!(inv.result.is_some(), "{inv:?}");

    // 088: a consented huddle recording, transcribed by the fixture route.
    s.use_fixture_asr();
    let huddle = s
        .huddle_act(HuddleActRequest::Create {
            project_id: project.clone(),
            title: "Rounds".to_owned(),
            retention_days: 30,
        })
        .unwrap()
        .huddle
        .unwrap()
        .header
        .id;
    let ana = s
        .huddle_act(HuddleActRequest::AddParticipant {
            huddle_id: huddle.clone(),
            display_name: "Ana".to_owned(),
            kind: HuddleParticipantKind::Human,
        })
        .unwrap()
        .receipts[0]
        .targets[0]
        .clone();
    for a in ConsentAct::ALL {
        s.huddle_act(HuddleActRequest::Consent {
            huddle_id: huddle.clone(),
            participant_id: ana.clone(),
            consent_act: *a,
            value: true,
        })
        .unwrap();
    }
    let wav = CliSession::synthetic_wav(PcmFormat::mono_16k(), &[(400, true), (300, false)]);
    let audio = s
        .audio_import(project.clone(), "rounds".to_owned(), wav)
        .unwrap()
        .header
        .id;
    let media = s
        .huddle_act(HuddleActRequest::Attach {
            huddle_id: huddle.clone(),
            source_id: audio,
            agent: None,
            day: 20_000,
        })
        .unwrap()
        .receipts[0]
        .targets[0]
        .clone();
    let t = s
        .huddle_act(HuddleActRequest::Transcribe {
            huddle_id: huddle.clone(),
            media_id: media,
            route: AudioRoute::FixtureAsr,
        })
        .unwrap();
    assert!(t.receipts[0].refusal.is_none(), "{t:?}");

    // 089: a Clinical Research Pack artifact.
    s.pack_act(PackActRequest::Install {
        project_id: project.clone(),
        pack_id: CLINICAL_RESEARCH_PACK_ID.to_owned(),
    })
    .unwrap();
    let mut fields = std::collections::BTreeMap::new();
    fields.insert("claim".to_owned(), FieldValue::Text("LDL falls".to_owned()));
    s.pack_act(PackActRequest::Create {
        project_id: project.clone(),
        pack_id: CLINICAL_RESEARCH_PACK_ID.to_owned(),
        type_id: "evidence_claim".to_owned(),
        fields,
    })
    .unwrap();

    // 090: a de-identified artifact written through an adapter to the
    // in-process institutional store.
    let store = Arc::new(InProcessStore::default());
    s.set_institutional_transport(store.clone());
    let adapter = s
        .adapter_act(AdapterActRequest::Register {
            project_id: project.clone(),
            name: "lab-store".to_owned(),
            config: AdapterConfig {
                kind: AdapterKind::ObjectStorage,
                destination: "s3://lab/exports".to_owned(),
                data_class_ceiling: DataClass::ExternalDeidentified,
                credential_handle: Some("cred:lab".to_owned()),
                capabilities: vec![AdapterCapability::PutObject, AdapterCapability::HeadObject],
            },
        })
        .unwrap()
        .adapter
        .unwrap()
        .header
        .id;
    let artifact = s
        .create_source_record("application/json".to_owned(), b"{\"n\":3}".to_vec())
        .unwrap();
    s.privacy_classify(
        project.clone(),
        artifact.clone(),
        DataClass::ExternalDeidentified,
        None,
    )
    .unwrap();
    let intent = s
        .adapter_act(AdapterActRequest::Intend {
            adapter_id: adapter.clone(),
            artifact_id: artifact,
            object_key: "exports/n.json".to_owned(),
        })
        .unwrap()
        .intent
        .unwrap();
    let sent = s
        .adapter_act(AdapterActRequest::Send {
            intent_id: intent.header.id,
        })
        .unwrap();
    assert_eq!(sent.intent.unwrap().state, EffectState::Confirmed);
    assert!(store.object("s3://lab/exports", "exports/n.json").is_some());

    // 091: an institution identity.
    s.federation_act(FederationActRequest::CreateIdentity {
        institution_id: "campaign-hospital".to_owned(),
    })
    .unwrap();

    Ids {
        project,
        snapshot,
        compute_job: job,
        r_workspace: ws.id().clone(),
        huddle,
        adapter,
    }
}

#[test]
fn every_plane_survives_backup_restore_and_restart_through_core() {
    let base = std::env::temp_dir().join(format!("medscale-092-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let vault_dir = base.join("vault");
    std::fs::create_dir_all(&vault_dir).unwrap();
    std::fs::write(vault_dir.join("labs.csv"), LABS).unwrap();

    let ids = {
        let mut s = CliSession::connect(VAULT).unwrap();
        s.open_synthetic_vault(&vault_dir.display().to_string())
            .unwrap();
        exercise_every_plane(&mut s, &base)
    };

    // Backup and restore at the storage layer, then every verifier.
    let backup = base.join("backup");
    {
        let vault = SyntheticVault::open(VAULT, &vault_dir).unwrap();
        backup_vault(&vault, &backup).unwrap();
    }
    let restored_dir = base.join("restored");
    restore_vault(&backup, &restored_dir).unwrap();
    {
        let restored = SyntheticVault::open(VAULT, &restored_dir).unwrap();
        let m = &restored.meta;
        m.verify_analytics_consistency().unwrap();
        m.verify_knowledge_consistency().unwrap();
        m.verify_compute_consistency().unwrap();
        m.verify_r_workspace_consistency().unwrap();
        m.verify_extension_consistency().unwrap();
        m.verify_huddle_consistency().unwrap();
        m.verify_research_pack_consistency().unwrap();
        m.verify_institutional_consistency().unwrap();
        m.verify_federation_consistency().unwrap();
        m.verify_audio_consistency().unwrap();
        m.verify_privacy_gate_consistency().unwrap();
    }

    // Restart on the restored vault: the same state, through Core.
    let mut s = CliSession::connect(VAULT).unwrap();
    s.open_synthetic_vault(&restored_dir.display().to_string())
        .unwrap();
    let job = s.compute_job(ids.compute_job.clone()).unwrap();
    assert_eq!(job.job.state, ComputeState::Completed);
    assert!(job.table.is_some());
    let history = s.r_workspace(ids.r_workspace.clone()).unwrap();
    assert_eq!(history.publications.len(), 1);
    assert_eq!(s.ext_list(ids.project.clone()).unwrap().installs.len(), 1);
    assert_eq!(s.huddle_get(ids.huddle.clone()).unwrap().media.len(), 1);
    assert_eq!(
        s.pack_artifact_list(ids.project.clone(), CLINICAL_RESEARCH_PACK_ID.to_owned())
            .unwrap()
            .len(),
        1
    );
    let adapter = s.adapter_get(ids.adapter.clone()).unwrap();
    assert_eq!(adapter.intents[0].state, EffectState::Confirmed);
    assert!(s.snapshot_get(ids.snapshot.clone()).is_ok());

    // Documented restore loss: the federation identity secret is never in
    // a backup, so the restored vault cannot sign.
    let fed = s.federation_get().unwrap();
    assert!(fed.identity.is_some());
    s.federation_act(FederationActRequest::TrustPeer {
        institution_id: "peer".to_owned(),
        public_key_hex: generate_device_key().1,
        ceiling: DataClass::Public,
    })
    .unwrap();
    let refused = s
        .federation_act(FederationActRequest::Export {
            project_id: ids.project,
            to_institution: "peer".to_owned(),
            artifact_ids: Vec::new(),
            tombstones: Vec::new(),
        })
        .unwrap();
    assert_eq!(refused.receipt.refusal, Some(FederationRefusal::NoIdentity));

    // Product defaults stay honest after restart.
    assert!(!s.compute_status().unwrap().platform_qualified);
    let r = s.r_status().unwrap();
    assert!(!r.managed_run_admitted && !r.platform_qualified);
}
