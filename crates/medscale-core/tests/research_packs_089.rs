//! Spec 089 Research Pack integration tests (first domain: Clinical
//! Research). Every call goes through `CliSession` -> `CoreFacade`.

use std::collections::BTreeMap;

use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::OpaqueId;
use medscale_contracts::research_packs::{
    CLINICAL_RESEARCH_PACK_ID, EvidenceAssessment, EvidenceQuality, FieldValue, PackActRequest,
    PackInstallState, SupportRelation, Tri,
};
use medscale_core::CliSession;

fn setup(name: &str) -> (CliSession, OpaqueId) {
    let dir = std::env::temp_dir().join(format!("medscale-089c-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut s = CliSession::connect(&format!("vault-089-{name}")).unwrap();
    s.open_synthetic_vault(&dir.display().to_string()).unwrap();
    let project = s
        .project_create("trial".to_owned(), None)
        .unwrap()
        .header
        .id;
    (s, project)
}

fn pack() -> String {
    CLINICAL_RESEARCH_PACK_ID.to_owned()
}

fn protocol_fields() -> BTreeMap<String, FieldValue> {
    let mut f = BTreeMap::new();
    f.insert(
        "title".to_owned(),
        FieldValue::Text("LDL lowering".to_owned()),
    );
    f.insert("phase".to_owned(), FieldValue::Choice("2".to_owned()));
    f.insert("primary_outcome".to_owned(), FieldValue::Unknown);
    f
}

fn create(
    s: &mut CliSession,
    project: &OpaqueId,
    type_id: &str,
    fields: BTreeMap<String, FieldValue>,
) -> Result<medscale_contracts::research_packs::ResearchArtifact, AuthorityError> {
    s.pack_act(PackActRequest::Create {
        project_id: project.clone(),
        pack_id: pack(),
        type_id: type_id.to_owned(),
        fields,
    })
    .map(|r| r.artifact.unwrap())
}

#[test]
fn a_pack_adds_validated_artifacts_and_workflows_without_new_authority() {
    let (mut s, project) = setup("workflow");
    assert_eq!(s.pack_catalog().unwrap().len(), 2);
    // Nothing exists before install.
    assert!(matches!(
        create(&mut s, &project, "study_protocol", protocol_fields()),
        Err(AuthorityError::NotFound)
    ));
    let installed = s
        .pack_act(PackActRequest::Install {
            project_id: project.clone(),
            pack_id: pack(),
        })
        .unwrap()
        .install
        .unwrap();
    assert_eq!(installed.version, 1);

    let mut bad = protocol_fields();
    bad.insert("phase".to_owned(), FieldValue::Choice("7".to_owned()));
    assert!(matches!(
        create(&mut s, &project, "study_protocol", bad),
        Err(AuthorityError::InvalidArgument { .. })
    ));
    let mut missing = protocol_fields();
    missing.remove("title");
    assert!(create(&mut s, &project, "study_protocol", missing).is_err());
    assert!(create(&mut s, &project, "no_such_type", protocol_fields()).is_err());

    let p = create(&mut s, &project, "study_protocol", protocol_fields()).unwrap();
    assert_eq!(p.workflow_state, "draft");
    // Only declared transitions.
    assert!(matches!(
        s.pack_act(PackActRequest::Transition {
            artifact_id: p.header.id.clone(),
            expected_revision: 1,
            to: "approved".to_owned(),
        }),
        Err(AuthorityError::InvalidArgument { .. })
    ));
    let p = s
        .pack_act(PackActRequest::Transition {
            artifact_id: p.header.id.clone(),
            expected_revision: 1,
            to: "submitted".to_owned(),
        })
        .unwrap()
        .artifact
        .unwrap();
    // Stale revisions conflict.
    assert!(matches!(
        s.pack_act(PackActRequest::Transition {
            artifact_id: p.header.id.clone(),
            expected_revision: 1,
            to: "approved".to_owned(),
        }),
        Err(AuthorityError::Conflict { .. })
    ));

    // Evidence assessments keep every axis explicit.
    let mut claim = BTreeMap::new();
    claim.insert(
        "claim".to_owned(),
        FieldValue::Text("Drug X lowers LDL".to_owned()),
    );
    let c = create(&mut s, &project, "evidence_claim", claim).unwrap();
    let assessment = EvidenceAssessment {
        claim: "Drug X lowers LDL".to_owned(),
        citation: "PMID:0000001".to_owned(),
        citation_exists: Tri::Unknown,
        relation: SupportRelation::Unknown,
        applicable_to_population: Tri::Unknown,
        year: None,
        jurisdiction: None,
        guideline_version: None,
        retracted: Tri::Unknown,
        quality: EvidenceQuality::Unknown,
        trial_criteria_met: Tri::Unknown,
    };
    let c = s
        .pack_act(PackActRequest::Assess {
            artifact_id: c.header.id.clone(),
            expected_revision: 1,
            assessment: assessment.clone(),
        })
        .unwrap()
        .artifact
        .unwrap();
    assert_eq!(c.assessments[0].verdict(), "unknown");
    let mut retracted_support = assessment;
    retracted_support.citation_exists = Tri::Yes;
    retracted_support.retracted = Tri::Yes;
    retracted_support.relation = SupportRelation::Supports;
    assert!(
        s.pack_act(PackActRequest::Assess {
            artifact_id: c.header.id.clone(),
            expected_revision: 2,
            assessment: retracted_support,
        })
        .is_err()
    );
}

#[test]
fn upgrades_migrate_without_loss_and_uninstall_keeps_data() {
    let (mut s, project) = setup("upgrade");
    s.pack_act(PackActRequest::Install {
        project_id: project.clone(),
        pack_id: pack(),
    })
    .unwrap();
    let p = create(&mut s, &project, "study_protocol", protocol_fields()).unwrap();
    let upgraded = s
        .pack_act(PackActRequest::Upgrade {
            project_id: project.clone(),
            pack_id: pack(),
        })
        .unwrap()
        .install
        .unwrap();
    assert_eq!(upgraded.version, 2);
    let migrated = s.pack_artifact_get(p.header.id.clone()).unwrap();
    assert_eq!(migrated.pack_version, 2);
    assert_eq!(migrated.revision, 2);
    for (k, v) in &p.fields {
        assert_eq!(migrated.fields.get(k), Some(v), "field {k} kept");
    }
    assert_eq!(
        migrated.fields.get("registry_id"),
        Some(&FieldValue::Unknown)
    );
    // No version 3 ships.
    assert!(
        s.pack_act(PackActRequest::Upgrade {
            project_id: project.clone(),
            pack_id: pack(),
        })
        .is_err()
    );

    let off = s
        .pack_act(PackActRequest::Uninstall {
            project_id: project.clone(),
            pack_id: pack(),
        })
        .unwrap()
        .install
        .unwrap();
    assert_eq!(off.state, PackInstallState::Disabled);
    assert_eq!(
        s.pack_artifact_list(project.clone(), pack()).unwrap().len(),
        1,
        "data is kept"
    );
    assert!(matches!(
        create(&mut s, &project, "study_protocol", protocol_fields()),
        Err(AuthorityError::Conflict { .. })
    ));
    assert!(
        s.pack_act(PackActRequest::Update {
            artifact_id: p.header.id.clone(),
            expected_revision: 2,
            fields: protocol_fields(),
        })
        .is_err()
    );
    // Reinstalling re-enables at the recorded version.
    let back = s
        .pack_act(PackActRequest::Install {
            project_id: project.clone(),
            pack_id: pack(),
        })
        .unwrap()
        .install
        .unwrap();
    assert_eq!((back.state, back.version), (PackInstallState::Enabled, 2));
    // Another Project sees nothing.
    let other = s
        .project_create("other".to_owned(), None)
        .unwrap()
        .header
        .id;
    assert!(
        s.pack_artifact_list(other.clone(), pack())
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        s.pack_install_get(other, pack()),
        Err(AuthorityError::NotFound)
    ));
}
