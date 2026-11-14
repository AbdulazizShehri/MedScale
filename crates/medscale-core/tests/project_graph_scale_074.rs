//! Spec 074 scale measurements (074-F, T074-11).
//!
//! Synthetic data only. This harness MEASURES honestly and reports; it never
//! asserts latency budgets or storage guarantees.
//!
//! Full fixtures (spec minimums):
//! Personal: >= 10 Projects and >= 1,000 artifact references.
//! Lab: >= 100 Projects.
//! Run with `-- --nocapture` to read the measurement lines.
//!
//! Routine `cargo test` runs a SMALL smoke shape so the workspace gate stays
//! fast (repository convention follows `perf_harness_027`: heavy scale is
//! env-gated). Set `MEDSCALE_074_FULL_SCALE=1` for the full spec-minimum
//! fixtures and record the output as scale evidence.
//!
//! Cost model (measured, see SCALE_MEASUREMENTS.md): lifecycle mutations
//! append audit rows and re-sync the full authority snapshot, while
//! high-frequency attach/detach/edge ops persist revisioned receipts and skip
//! the snapshot rewrite when the memory store is untouched
//! (`RequestBody::preserves_memory_snapshot`). Per-op cost is therefore
//! constant in graph size; the snapshot term grows only with lifecycle and
//! source history.

use std::fs;
use std::time::Instant;

use medscale_contracts::envelopes::{Capability, RequestBody, ResponseBody};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_contracts::project_graph::{
    ArtifactDescriptor, ArtifactKind, ArtifactVersionBinding, GraphDirection, GraphEndpoint,
    ProjectGraphPredicate,
};
use medscale_core::CoreFacade;

fn tmp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("medscale-074s-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn base_req(
    capability: Capability,
    body: RequestBody,
) -> medscale_contracts::envelopes::AuthorityRequest {
    medscale_contracts::envelopes::AuthorityRequest::new(
        OpaqueId::new("req"),
        VaultId::new("vault-1"),
        RealmId::new("realm-a"),
        AuthorityScopeId::new("scope-a"),
        capability,
        body,
    )
}

struct ScaleHarness {
    facade: CoreFacade,
    session: OpaqueId,
    dir: std::path::PathBuf,
}

fn setup_scale(name: &str) -> ScaleHarness {
    let dir = tmp_dir(name);
    let facade = CoreFacade::new();
    let holder = match facade
        .dispatch(base_req(
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("scale"),
                holder_id_hint: None,
            },
        ))
        .result
        .expect("lease")
    {
        ResponseBody::Lease { holder_id, .. } => holder_id,
        other => panic!("{other:?}"),
    };
    let session = match facade
        .dispatch(base_req(
            Capability::OpenSession,
            RequestBody::OpenSession {
                holder_id: holder,
                granted: vec![
                    Capability::OpenSyntheticVault,
                    Capability::ProjectCreate,
                    Capability::ProjectRead,
                    Capability::ProjectUpdate,
                    Capability::ProjectArchive,
                    Capability::ExperimentCreate,
                    Capability::ExperimentRead,
                    Capability::ExperimentUpdate,
                    Capability::ExperimentArchive,
                    Capability::ProjectArtifactAttach,
                    Capability::ProjectArtifactDetach,
                    Capability::ProjectGraphRead,
                    Capability::ProjectGraphMutate,
                    Capability::CreateSourceRecord,
                ],
                ttl_ticks: 10_000_000,
            },
        ))
        .result
        .expect("session")
    {
        ResponseBody::Session { session_id, .. } => session_id,
        other => panic!("{other:?}"),
    };
    let mut vault_req = base_req(
        Capability::OpenSyntheticVault,
        RequestBody::OpenSyntheticVault {
            vault_root: dir.to_str().unwrap().to_owned(),
        },
    );
    vault_req.session_id = Some(session.clone());
    facade.dispatch(vault_req).result.expect("vault");
    ScaleHarness {
        facade,
        session,
        dir,
    }
}

impl ScaleHarness {
    fn call(
        &self,
        capability: Capability,
        body: RequestBody,
    ) -> Result<ResponseBody, medscale_contracts::envelopes::AuthorityError> {
        let mut request = base_req(capability, body);
        request.session_id = Some(self.session.clone());
        self.facade.dispatch(request).result
    }

    fn project(&self, name: &str) -> OpaqueId {
        match self
            .call(
                Capability::ProjectCreate,
                RequestBody::ProjectCreate {
                    name: name.to_owned(),
                    description: None,
                },
            )
            .expect("project")
        {
            ResponseBody::Project { project } => project.header.id,
            other => panic!("{other:?}"),
        }
    }

    fn source(&self, seed: usize) -> OpaqueId {
        match self
            .call(
                Capability::CreateSourceRecord,
                RequestBody::CreateSourceRecord {
                    media_type: "text/plain".to_owned(),
                    bytes: format!("synthetic-scale-bytes-{seed}").into_bytes(),
                },
            )
            .expect("source")
        {
            ResponseBody::Created { object_id } => object_id,
            other => panic!("{other:?}"),
        }
    }

    fn experiment(&self, project: &OpaqueId, name: &str) -> OpaqueId {
        match self
            .call(
                Capability::ExperimentCreate,
                RequestBody::ExperimentCreate {
                    project_id: project.clone(),
                    name: name.to_owned(),
                    description: None,
                },
            )
            .expect("experiment")
        {
            ResponseBody::Experiment { experiment } => experiment.header.id,
            other => panic!("{other:?}"),
        }
    }

    fn attach(&self, project: &OpaqueId, experiment: &OpaqueId, object: OpaqueId) {
        self.call(
            Capability::ProjectArtifactAttach,
            RequestBody::ProjectAttach {
                project_id: project.clone(),
                experiment_id: Some(experiment.clone()),
                artifact: ArtifactDescriptor {
                    object_id: object,
                    kind: ArtifactKind::SourceRecord,
                    binding: ArtifactVersionBinding::IdentityOnly,
                },
            },
        )
        .expect("attach");
    }

    fn edge(&self, project: &OpaqueId, experiment: &OpaqueId, object: &OpaqueId) {
        self.call(
            Capability::ProjectGraphMutate,
            RequestBody::GraphEdgeCreate {
                project_id: project.clone(),
                subject: GraphEndpoint::Experiment(experiment.clone()),
                predicate: ProjectGraphPredicate::ExperimentInput,
                object: GraphEndpoint::Artifact(ArtifactDescriptor {
                    object_id: object.clone(),
                    kind: ArtifactKind::SourceRecord,
                    binding: ArtifactVersionBinding::IdentityOnly,
                }),
            },
        )
        .expect("edge");
    }

    fn db_bytes(&self) -> u64 {
        fs::metadata(self.dir.join("meta.sqlite3"))
            .expect("db")
            .len()
    }
}

fn report(label: &str, started: Instant, ops: usize) {
    let ms = started.elapsed().as_secs_f64() * 1000.0;
    eprintln!(
        "SCALE074 {label}: ops={ops} total_ms={ms:.1} mean_ms_per_op={:.3}",
        ms / ops as f64
    );
}

/// Full spec-minimum fixtures run only when explicitly enabled so routine
/// workspace tests stay fast. Follows the `perf_harness_027` env-gate
/// convention (`MEDSCALE_027_DELIVERY_PLAN_SCALE`).
fn full_scale_enabled() -> bool {
    matches!(
        std::env::var("MEDSCALE_074_FULL_SCALE").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

fn scale_shape() -> (usize, usize, usize) {
    // (projects, refs_per_project, lab_projects). Smoke stays small and fast;
    // full matches the spec-minimum Personal and Lab fixtures.
    if full_scale_enabled() {
        (10, 100, 100)
    } else {
        eprintln!("SCALE074 shape=smoke (set MEDSCALE_074_FULL_SCALE=1 for full fixtures)");
        (2, 5, 5)
    }
}

#[test]
fn personal_scale_10_projects_1000_refs() {
    let (project_count, refs_per_project, _) = scale_shape();
    let full = full_scale_enabled();
    eprintln!(
        "SCALE074 personal_shape=projects={project_count} refs_per_project={refs_per_project} full={full}"
    );
    let h = setup_scale("personal");
    let projects: Vec<OpaqueId> = (0..project_count)
        .map(|i| h.project(&format!("personal-{i}")))
        .collect();
    let mut first_experiment: Option<OpaqueId> = None;

    let started = Instant::now();
    let mut refs = 0_usize;
    let mut edges = 0_usize;
    let expected_refs = project_count * refs_per_project;
    for (pi, project) in projects.iter().enumerate() {
        let experiment = h.experiment(project, &format!("exp-{pi}"));
        if pi == 0 {
            first_experiment = Some(experiment.clone());
        }
        for i in 0..refs_per_project {
            let seed = pi * 1000 + i;
            let object = h.source(seed);
            let t = Instant::now();
            h.attach(project, &experiment, object.clone());
            refs += 1;
            h.edge(project, &experiment, &object);
            edges += 1;
            if refs == 1 {
                eprintln!(
                    "SCALE074 personal_first_attach_ms={:.2}",
                    t.elapsed().as_secs_f64() * 1000.0
                );
            }
            if refs.is_multiple_of(100) {
                eprintln!(
                    "SCALE074 personal_progress refs={refs} elapsed_s={:.1}",
                    started.elapsed().as_secs_f64()
                );
            }
            if refs == expected_refs {
                eprintln!(
                    "SCALE074 personal_last_attach_ms={:.2}",
                    t.elapsed().as_secs_f64() * 1000.0
                );
            }
        }
    }
    report(
        "personal_mutations",
        started,
        project_count + project_count + expected_refs + refs + edges,
    );
    assert_eq!((refs, edges), (expected_refs, expected_refs));

    let started = Instant::now();
    let (listed, _) = match h
        .call(
            Capability::ProjectRead,
            RequestBody::ProjectList {
                status: None,
                limit: Some(100),
                cursor: None,
            },
        )
        .expect("list")
    {
        ResponseBody::ProjectList {
            projects,
            next_cursor,
        } => (projects, next_cursor),
        other => panic!("{other:?}"),
    };
    eprintln!(
        "SCALE074 personal_list_ms={:.2} projects={}",
        started.elapsed().as_secs_f64() * 1000.0,
        listed.len()
    );
    assert_eq!(listed.len(), project_count);

    let started = Instant::now();
    let context = match h
        .call(
            Capability::ProjectRead,
            RequestBody::ProjectContextResolve {
                project_id: projects[0].clone(),
                experiment_id: None,
                refs_limit: Some(100),
                graph_limit: Some(100),
            },
        )
        .expect("context")
    {
        ResponseBody::ProjectContext { context } => context,
        other => panic!("{other:?}"),
    };
    eprintln!(
        "SCALE074 personal_context_ms={:.2} refs={} edges={}",
        started.elapsed().as_secs_f64() * 1000.0,
        context.authorized_artifact_refs.len(),
        context.graph_slice.len()
    );
    assert_eq!(context.authorized_artifact_refs.len(), refs_per_project);
    assert_eq!(context.graph_slice.len(), refs_per_project);

    let started = Instant::now();
    let page = match h
        .call(
            Capability::ProjectGraphRead,
            RequestBody::GraphNeighbors {
                project_id: projects[0].clone(),
                start: GraphEndpoint::Experiment(first_experiment.expect("first experiment")),
                predicates: None,
                direction: Some(GraphDirection::Both),
                limit: Some(100),
                cursor: None,
            },
        )
        .expect("neighbors")
    {
        ResponseBody::GraphNeighbors { page } => page,
        other => panic!("{other:?}"),
    };
    eprintln!(
        "SCALE074 personal_neighbors_ms={:.2} edges={}",
        started.elapsed().as_secs_f64() * 1000.0,
        page.edges.len()
    );
    assert_eq!(page.edges.len(), refs_per_project);
    eprintln!("SCALE074 personal_db_bytes={}", h.db_bytes());
}

#[test]
fn lab_scale_100_projects() {
    // Minimum compliant lab shape: >= 100 Projects. Refs stay small on
    // purpose so the fixture stays fast everywhere; the personal fixture
    // already proves per-project ref/edge depth. See the module docs for the
    // measured cost model.
    let (_, _, lab_projects) = scale_shape();
    let full = full_scale_enabled();
    // Smoke keeps the project count small; refs-per-project stay at 2 in both
    // shapes so the per-project structure is identical.
    eprintln!("SCALE074 lab_shape=projects={lab_projects} full={full}");
    let h = setup_scale("lab");
    let started = Instant::now();
    let mut refs = 0_usize;
    for i in 0..lab_projects {
        let project = h.project(&format!("lab-{i}"));
        let experiment = h.experiment(&project, &format!("lab-exp-{i}"));
        for j in 0..2 {
            let object = h.source(i * 100 + j);
            h.attach(&project, &experiment, object.clone());
            refs += 1;
            h.edge(&project, &experiment, &object);
        }
        if (i + 1).is_multiple_of(20) {
            eprintln!(
                "SCALE074 lab_progress projects={} elapsed_s={:.1}",
                i + 1,
                started.elapsed().as_secs_f64()
            );
        }
    }
    let expected_refs = lab_projects * 2;
    report(
        "lab_mutations",
        started,
        lab_projects + lab_projects + expected_refs + refs * 2,
    );
    assert_eq!(refs, expected_refs);

    let started = Instant::now();
    let (listed, _) = match h
        .call(
            Capability::ProjectRead,
            RequestBody::ProjectList {
                status: None,
                limit: Some(100),
                cursor: None,
            },
        )
        .expect("list")
    {
        ResponseBody::ProjectList {
            projects,
            next_cursor,
        } => (projects, next_cursor),
        other => panic!("{other:?}"),
    };
    eprintln!(
        "SCALE074 lab_list_ms={:.2} projects={}",
        started.elapsed().as_secs_f64() * 1000.0,
        listed.len()
    );
    assert_eq!(listed.len(), lab_projects);
    eprintln!("SCALE074 lab_db_bytes={}", h.db_bytes());
}

#[test]
fn reopen_duration_after_scale() {
    let dir = tmp_dir("reopen-scale");
    {
        let h = setup_scale("reopen-build");
        // Redirect-free mini fixture: reuse the lab shape on a fixed dir is
        // unnecessary; measure open cost on a moderately populated vault.
        let project = h.project("reopen-0");
        let experiment = h.experiment(&project, "reopen-exp");
        for i in 0..50 {
            let object = h.source(i);
            h.attach(&project, &experiment, object);
        }
        let bytes = h.db_bytes();
        eprintln!("SCALE074 reopen_db_bytes={bytes}");
        // Copy the vault dir to the measured path for a clean reopen sample.
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::copy(h.dir.join("meta.sqlite3"), dir.join("meta.sqlite3")).unwrap();
        for entry in fs::read_dir(&h.dir).expect("read vault dir") {
            let entry = entry.expect("entry");
            let name = entry.file_name();
            if name != "meta.sqlite3" {
                let _ = fs::copy(entry.path(), dir.join(name));
            }
        }
    }
    let started = Instant::now();
    let meta =
        medscale_storage::SqliteMetaStore::open_at(&dir.join("meta.sqlite3")).expect("reopen");
    eprintln!(
        "SCALE074 reopen_ms={:.2} projects={} refs={}",
        started.elapsed().as_secs_f64() * 1000.0,
        meta.list_all_projects().expect("projects").len(),
        meta.list_all_refs().expect("refs").len()
    );
}
