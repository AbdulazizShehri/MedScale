//! End-to-end feature test: every product surface is driven through the
//! production handler, the generated capability context and a real Core
//! session on a fresh synthetic vault. MockRuntime replaces only the WebView.

use serde_json::{Value, json};
use tauri::Manager;
use tauri::test::{MockRuntime, get_ipc_response, mock_builder};
use tauri::webview::InvokeRequest;

fn origin() -> &'static str {
    if cfg!(any(windows, target_os = "android")) {
        "http://tauri.localhost"
    } else {
        "tauri://localhost"
    }
}

struct Harness {
    _app: tauri::App<MockRuntime>,
    window: tauri::WebviewWindow<MockRuntime>,
    /// Every (command, args, outcome), so a run can be replayed for visual QA.
    log: std::cell::RefCell<Vec<Value>>,
}

impl Harness {
    fn new(name: &str) -> Self {
        Self::start(name, true)
    }

    /// A new app process on the same data directory (an app restart).
    fn restart(name: &str) -> Self {
        Self::start(name, false)
    }

    fn start(name: &str, fresh: bool) -> Self {
        let app = super::presentation_builder(mock_builder())
            .build(tauri::generate_context!())
            .expect("build production context");
        let dir = std::env::temp_dir().join(format!(
            "medscale-tauri-feature-{name}-{}",
            std::process::id()
        ));
        if fresh {
            let _ = std::fs::remove_dir_all(&dir);
        }
        std::fs::create_dir_all(&dir).unwrap();
        app.state::<crate::host::Host>()
            .state
            .lock()
            .unwrap()
            .data_dir = Some(dir);
        let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        Self {
            _app: app,
            window,
            log: std::cell::RefCell::new(Vec::new()),
        }
    }

    fn call(&self, cmd: &str, args: Value) -> Result<Value, Value> {
        let recorded = args.clone();
        let req = InvokeRequest {
            cmd: cmd.to_owned(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: origin().parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(args),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.to_owned(),
        };
        let out = get_ipc_response(&self.window, req)
            .map(|body| body.deserialize::<Value>().unwrap_or(Value::Null));
        self.log.borrow_mut().push(match &out {
            Ok(v) => json!({ "cmd": cmd, "args": recorded, "ok": v }),
            Err(e) => json!({ "cmd": cmd, "args": recorded, "err": e }),
        });
        out
    }

    /// Writes the recorded run when MEDSCALE_UI_FIXTURES names a file.
    fn dump(&self) {
        if let Ok(path) = std::env::var("MEDSCALE_UI_FIXTURES") {
            std::fs::write(
                path,
                serde_json::to_vec_pretty(&*self.log.borrow()).unwrap(),
            )
            .unwrap();
        }
    }

    fn ok(&self, cmd: &str, args: Value) -> Value {
        self.call(cmd, args.clone())
            .unwrap_or_else(|e| panic!("{cmd} {args} failed: {e}"))
    }
}

/// Exercises the real named native command across a simulated app restart,
/// without an installed model, microphone, user data or external network.
#[test]
fn confirmed_imported_audio_removal_survives_native_restart() {
    let h = Harness::new("audio-removal-restart");
    h.ok("workspace_open_synthetic", json!({}));
    let project = h.ok(
        "project_create",
        json!({"name": "Disposable audio retention check", "description": null}),
    );
    let project_id = project["id"]
        .as_str()
        .expect("created project ID")
        .to_owned();
    let imported = h.ok("audio_import_synthetic", json!({"projectId": project_id}));
    let source_id = imported["source_id"]
        .as_str()
        .expect("source ID")
        .to_owned();
    let inspected = h.ok(
        "audio_source_retention_preview",
        json!({"projectId": project_id, "sourceId": source_id}),
    );
    assert_eq!(inspected["delete_available"], true, "{inspected}");
    let expected_digest = inspected["expected_digest"]
        .as_str()
        .expect("source SHA-256")
        .to_owned();
    let acknowledged = h.ok(
        "audio_source_remove_confirmed",
        json!({
            "projectId": project_id,
            "sourceId": source_id,
            "expectedDigest": expected_digest,
            "confirmation": format!("REMOVE {source_id} FROM ACTIVE VAULT"),
        }),
    );
    assert_eq!(acknowledged["source_id"], source_id);
    assert_eq!(acknowledged["removed_bytes"], inspected["recording_bytes"]);
    assert!(acknowledged["receipt_id"].is_string(), "{acknowledged}");
    assert_eq!(acknowledged["active_vault_only"], true);
    assert_eq!(acknowledged["older_backups_may_retain_recording"], true);
    h.ok("workspace_lock", json!({}));
    drop(h);

    let restarted = Harness::restart("audio-removal-restart");
    restarted.ok("workspace_open_synthetic", json!({}));
    let overview = restarted.ok("audio_overview", json!({"projectId": project_id}));
    assert!(
        overview["sources"]
            .as_array()
            .expect("source list")
            .iter()
            .all(|s| s["id"] != source_id),
        "a removed source reappeared after native restart: {overview}"
    );
    let inspect_error = restarted
        .call(
            "audio_source_retention_preview",
            json!({"projectId": project_id, "sourceId": source_id}),
        )
        .unwrap_err();
    assert_eq!(inspect_error["kind"], "missing", "{inspect_error}");
    // A second attempt must never silently succeed after restart.
    let repeated = restarted
        .call(
            "audio_source_remove_confirmed",
            json!({
                "projectId": project_id,
                "sourceId": source_id,
                "expectedDigest": expected_digest,
                "confirmation": format!("REMOVE {source_id} FROM ACTIVE VAULT"),
            }),
        )
        .unwrap_err();
    assert_eq!(repeated["kind"], "missing", "{repeated}");
}

#[test]
fn every_surface_runs_against_real_core() {
    let h = Harness::new("all");

    // Workspace gate: nothing works before a vault is open.
    let err = h.call("patients_list", json!({})).unwrap_err();
    assert_eq!(err["kind"], "unavailable", "{err}");
    let status = h.ok("workspace_status", json!({}));
    assert_eq!(status["open"], false);

    // Open the synthetic workspace: fixtures are ingested and promoted.
    let ws = h.ok("workspace_open_synthetic", json!({}));
    assert_eq!(ws["subjects"], 2, "{ws}");
    assert!(ws["sources"].as_u64().unwrap() >= 10);
    assert_eq!(
        h.ok("get_shell_status", json!({}))["coreConnection"],
        "connected"
    );

    // Patients: real brief/timeline/coverage from Core.
    let patients = h.ok("patients_list", json!({}));
    let subjects = patients["subjects"].as_array().unwrap();
    assert_eq!(subjects.len(), 2);
    let ada = subjects
        .iter()
        .find(|s| s["subject_ref"] == "synthetic-subject-ada")
        .unwrap();
    assert!(
        ada["display_name"].as_str().unwrap().contains("Ada"),
        "{ada}"
    );
    let detail = h.ok(
        "patient_detail",
        json!({ "subjectRef": "synthetic-subject-ada" }),
    );
    assert!(!detail["timeline"].as_array().unwrap().is_empty());
    assert!(
        detail["contracts"]["coverage"]["slots"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["status"] == "unsupported_resource_type"),
        "{}",
        detail["contracts"]["coverage"]
    );
    let other = h.ok(
        "patient_detail",
        json!({ "subjectRef": "synthetic-subject-coverage" }),
    );
    assert!(
        other["contracts"]["coverage"]["slots"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["status"] != "present")
    );
    // A subject outside this workspace is denied, never empty.
    assert_eq!(
        h.call("patient_detail", json!({ "subjectRef": "someone-else" }))
            .unwrap_err()["kind"],
        "denied"
    );
    assert_eq!(
        h.call("patient_detail", json!({ "subjectRef": "../etc" }))
            .unwrap_err()["kind"],
        "invalid"
    );

    // Documents, evidence, insights.
    assert!(
        h.ok("documents_list", json!({}))["sources"]
            .as_array()
            .unwrap()
            .len()
            >= 10
    );
    let corpus = h.ok("evidence_corpus", json!({}));
    assert!(!corpus["documents"].as_array().unwrap().is_empty());
    let hits = h.ok(
        "evidence_search",
        json!({ "query": "diabetes", "includeRetracted": false }),
    );
    assert!(hits["relevance_is_not_authority"].as_bool().unwrap_or(true));
    let insights = h.ok("insights_overview", json!({}));
    assert_eq!(insights["cohort_size"], 2, "{insights}");
    // Grounded answers: cited sentences from the corpus, or an abstention.
    let grounded = h.ok(
        "insights_ask",
        json!({ "query": "metformin dose in diabetes" }),
    );
    assert_eq!(grounded["answer"]["abstained"], false, "{grounded}");
    assert_eq!(
        grounded["answer"]["citations"][0]["doc_id"], "doc-dm",
        "{grounded}"
    );
    assert!(grounded["answer"]["citations"][0]["content_digest"].is_string());
    // The only statin document is retracted, so nothing supports it.
    let retracted = h.ok("insights_ask", json!({ "query": "statin myopathy" }));
    assert_eq!(retracted["answer"]["abstained"], true, "{retracted}");

    // Projects.
    let project = h.ok(
        "project_create",
        json!({ "name": "Feature study", "description": null }),
    );
    let pid = project["id"].as_str().unwrap().to_owned();
    assert!(
        h.ok("projects_list", json!({}))
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == pid)
    );
    h.ok(
        "experiment_create",
        json!({ "projectId": pid, "name": "Coverage pass" }),
    );
    assert_eq!(
        h.ok("project_detail", json!({ "projectId": pid }))["experiment_count"],
        1
    );

    // Data: sample source → snapshot → page → transform.
    let source = h.ok("data_add_sample_source", json!({ "projectId": pid }));
    let sid = source["source_id"].as_str().unwrap().to_owned();
    assert_eq!(
        h.ok("data_sources", json!({ "projectId": pid }))
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let snap = h.ok("data_import_snapshot", json!({ "sourceId": sid }));
    assert_eq!(snap["rows"], 4);
    let snap_id = snap["id"].as_str().unwrap().to_owned();
    let page = h.ok("data_page", json!({ "snapshotId": snap_id }));
    assert!(page["schema_line"].as_str().unwrap().contains("label"));
    h.ok("data_transform", json!({ "snapshotId": snap_id, "columns": ["label"], "filterColumn": null, "filterOp": null, "filterValue": null, "sort": null }));

    // Analytics: read-only query over the snapshot; writes are denied.
    let q = h.ok("analytics_query", json!({ "projectId": pid, "sql": "SELECT label FROM t", "bindings": format!("t={snap_id}") }));
    assert!(!q["summary"].as_str().unwrap().is_empty());
    let denied = h.ok(
        "analytics_query",
        json!({ "projectId": pid, "sql": "DELETE FROM t", "bindings": format!("t={snap_id}") }),
    );
    assert!(
        denied["summary"]
            .as_str()
            .unwrap()
            .contains("not_read_only"),
        "{denied}"
    );
    assert!(
        !h.ok("analytics_receipts", json!({ "projectId": pid }))
            .as_array()
            .unwrap()
            .is_empty()
    );

    // Knowledge.
    h.ok("knowledge_build", json!({ "projectId": pid }));
    h.ok(
        "knowledge_search",
        json!({ "projectId": pid, "query": "alpha" }),
    );
    h.ok("knowledge_overview", json!({ "projectId": pid }));

    // Browse: denied with an empty allowlist, then fetched from the offline fixture.
    let overview = h.ok("browse_overview", json!({ "projectId": pid }));
    let url = overview["fixture_url"].as_str().unwrap().to_owned();
    assert_eq!(
        h.ok("browse_fetch", json!({ "projectId": pid, "url": url }))["session"]["state"],
        "denied"
    );
    h.ok(
        "browse_allow",
        json!({ "projectId": pid, "hostName": "fixture.medscale.test", "pathPrefix": "" }),
    );
    let fetched = h.ok("browse_fetch", json!({ "projectId": pid, "url": url }));
    assert_ne!(fetched["session"]["state"], "denied", "{fetched}");

    // Research OS + privacy.
    h.ok("research_os_rows", json!({ "projectId": pid }));
    h.ok("privacy_overview", json!({ "projectId": pid }));
    let first_source = h.ok("documents_list", json!({}))["sources"][0]["source_id"]
        .as_str()
        .unwrap()
        .to_owned();
    h.ok(
        "privacy_check_egress",
        json!({ "projectId": pid, "artifactId": first_source, "boundary": "browse" }),
    );

    // Models: least-privilege pack session admits the fixture pack.
    h.ok("models_admit_fixture_pack", json!({}));
    assert!(
        !h.ok("models_overview", json!({}))["models"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    // MedAgent: create → start → execute on the local ONNX fixture runtime.
    let run = h.ok(
        "medagent_create_run",
        json!({ "projectId": pid, "prompt": "summarize the bound sources" }),
    );
    assert_eq!(run["status"], "pending");
    let rid = run["id"].as_str().unwrap().to_owned();
    let started = h.ok(
        "medagent_start",
        json!({ "runId": rid, "revision": run["revision"] }),
    );
    assert_eq!(started["status"], "running", "{started}");
    let executed = h.ok("medagent_execute", json!({ "runId": rid }));
    assert!(executed["proposal"].is_object(), "{executed}");
    assert!(
        !h.ok("medagent_turns", json!({ "runId": rid }))
            .as_array()
            .unwrap()
            .is_empty()
    );

    // Model fleet: two lanes → fleet → dispatch → execute → compare.
    let lanes = h.ok("fleet_setup_lanes", json!({ "projectId": pid }));
    let lane_ids: Vec<String> = lanes["lanes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(lane_ids.len(), 2);
    let fleet = h.ok(
        "fleet_create",
        json!({ "projectId": pid, "prompt": "summarize" }),
    );
    let fid = fleet["id"].as_str().unwrap().to_owned();
    h.ok(
        "fleet_dispatch",
        json!({ "fleetId": fid, "revision": fleet["revision"], "laneIds": lane_ids }),
    );
    for lane in &lane_ids {
        h.ok(
            "fleet_execute_lane",
            json!({ "fleetId": fid, "laneId": lane }),
        );
    }
    let report = h.ok("fleet_compare", json!({ "fleetId": fid }));
    assert!(report["id"].is_string(), "{report}");
    assert!(h.ok("fleet_detail", json!({ "fleetId": fid }))["report"].is_object());

    // Collaboration.
    let room = h.ok(
        "collab_create_room",
        json!({ "projectId": pid, "name": "Review room" }),
    );
    let room_id = room["id"].as_str().unwrap().to_owned();
    let thread = h.ok(
        "collab_open_thread",
        json!({ "roomId": room_id, "artifactId": first_source }),
    );
    let tid = thread["id"].as_str().unwrap().to_owned();
    h.ok(
        "collab_post_message",
        json!({ "threadId": tid, "body": "Coverage gap on medication." }),
    );
    assert_eq!(
        h.ok("collab_messages", json!({ "threadId": tid }))
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let task = h.ok(
        "collab_create_task",
        json!({ "roomId": room_id, "title": "Check conflict" }),
    );
    h.ok(
        "collab_complete_task",
        json!({ "taskId": task["id"], "revision": task["revision"] }),
    );
    h.ok("collab_room_detail", json!({ "roomId": room_id }));

    // Workflows, audio, governance, about.
    assert_eq!(
        h.ok("workflow_overview", json!({}))["steps"]
            .as_array()
            .unwrap()
            .len(),
        5
    );
    let audio = h.ok("audio_import_synthetic", json!({ "projectId": pid }));
    h.ok(
        "audio_segment",
        json!({ "projectId": pid, "sourceId": audio["source_id"] }),
    );
    assert!(
        !h.ok("audio_overview", json!({ "projectId": pid }))["sources"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let gov = h.ok("governance_overview", json!({}));
    assert!(!gov["settings_rows"].as_array().unwrap().is_empty());
    assert!(gov["fhir_support"].is_object());
    let about = h.ok("about_info", json!({}));
    assert_eq!(about["product"], "MedScale");
    // The product name drops "Preview"; the unqualified state stays explicit.
    assert_eq!(about["release_ready"], false);

    // Final read of every surface (also the visual-QA replay snapshot).
    for (cmd, args) in [
        ("workspace_status", json!({})),
        ("projects_list", json!({})),
        ("medagent_runs", json!({ "projectId": pid })),
        ("medagent_turns", json!({ "runId": rid })),
        ("fleet_overview", json!({ "projectId": pid })),
        ("collab_rooms", json!({ "projectId": pid })),
        ("data_sources", json!({ "projectId": pid })),
        ("data_snapshots", json!({ "sourceId": sid })),
        ("analytics_receipts", json!({ "projectId": pid })),
        ("knowledge_overview", json!({ "projectId": pid })),
        ("browse_overview", json!({ "projectId": pid })),
        ("research_os_rows", json!({ "projectId": pid })),
        ("privacy_overview", json!({ "projectId": pid })),
        ("models_overview", json!({})),
        ("workflow_overview", json!({})),
        ("audio_overview", json!({ "projectId": pid })),
        ("governance_overview", json!({})),
        ("documents_list", json!({})),
        ("patients_list", json!({})),
        ("insights_overview", json!({})),
        ("evidence_corpus", json!({})),
        ("about_info", json!({})),
    ] {
        h.ok(cmd, args);
    }
    h.dump();

    // Lock: the session is dropped and reads become unavailable again.
    h.ok("workspace_lock", json!({}));
    assert_eq!(
        h.call("patients_list", json!({})).unwrap_err()["kind"],
        "unavailable"
    );
}

#[test]
fn orphaned_encrypted_seed_marker_is_not_a_vault() {
    let h = Harness::new("orphan-encrypted-seed");
    let root = std::env::temp_dir().join(format!(
        "medscale-tauri-feature-orphan-encrypted-seed-{}",
        std::process::id()
    ));
    let marker = root.join("encrypted-workspace.desktop-seed.json");
    let original = b"{\"subjects\":[],\"sources\":[]}";
    std::fs::write(&marker, original).unwrap();

    let status = h.ok("workspace_status", json!({}));
    assert_eq!(status["encrypted_vault_exists"], false);
    assert_eq!(status["open"], false);

    let missing = h
        .call(
            "workspace_unlock",
            json!({ "passphrase": "fictional test only" }),
        )
        .unwrap_err();
    assert_eq!(missing["kind"], "missing", "{missing}");
    assert_eq!(std::fs::read(&marker).unwrap(), original);
    assert!(!root.join("encrypted-workspace").exists());

    // A partially written vault header without sealed Core metadata is
    // not an unlockable vault either; do not open, delete, or rewrite it.
    let vault_root = root.join("encrypted-workspace");
    std::fs::create_dir_all(&vault_root).unwrap();
    let header = vault_root.join("vault_header.json");
    std::fs::write(&header, b"{}").unwrap();
    assert_eq!(
        h.ok("workspace_status", json!({}))["encrypted_vault_exists"],
        false
    );
    let incomplete = h
        .call(
            "workspace_unlock",
            json!({ "passphrase": "fictional test only" }),
        )
        .unwrap_err();
    assert_eq!(incomplete["kind"], "corrupt", "{incomplete}");
    assert_eq!(std::fs::read(&header).unwrap(), b"{}");
    assert_eq!(std::fs::read(&marker).unwrap(), original);
}

#[test]
fn encrypted_vault_create_lock_unlock_and_wrong_passphrase() {
    let h = Harness::new("enc");
    assert_eq!(
        h.call(
            "workspace_unlock",
            json!({ "passphrase": "correct horse battery" })
        )
        .unwrap_err()["kind"],
        "missing"
    );
    assert_eq!(
        h.call(
            "workspace_create_encrypted",
            json!({ "passphrase": "short" })
        )
        .unwrap_err()["kind"],
        "invalid"
    );
    let created = h.ok(
        "workspace_create_encrypted",
        json!({ "passphrase": "correct horse battery" }),
    );
    assert_eq!(created["workspace"]["kind"], "encrypted");
    // Core admits FHIR ingest into synthetic vaults only; an encrypted vault starts empty.
    assert_eq!(
        h.ok("patients_list", json!({}))["subjects"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    let project = h.ok(
        "project_create",
        json!({ "name": "Encrypted study", "description": null }),
    );
    let vault_root = std::env::temp_dir()
        .join(format!("medscale-tauri-feature-enc-{}", std::process::id()))
        .join("encrypted-workspace");
    assert!(
        vault_root.join("meta.work.sqlite3").exists(),
        "work DB exists while unlocked"
    );
    h.ok("workspace_lock", json!({}));
    assert!(
        !vault_root.join("meta.work.sqlite3").exists(),
        "lock must seal and remove the working DB"
    );
    assert!(
        h.call(
            "workspace_unlock",
            json!({ "passphrase": "wrong passphrase!" })
        )
        .is_err()
    );
    h.ok(
        "workspace_unlock",
        json!({ "passphrase": "correct horse battery" }),
    );
    // Lock seals the vault through Core: writes survive and no unsealed working DB remains.
    let listed = h.ok("projects_list", json!({}));
    assert_eq!(
        listed.as_array().unwrap().len(),
        1,
        "projects after unlock: {listed}"
    );
    assert_eq!(listed[0]["id"], project["id"]);
    assert_eq!(
        h.call(
            "workspace_create_encrypted",
            json!({ "passphrase": "another passphrase" })
        )
        .unwrap_err()["kind"],
        "conflict"
    );
}

#[test]
fn medagent_workbench_agent_context_evidence_and_resume() {
    let pid;
    let rid;
    let agent_id;
    {
        let h = Harness::new("medagent");
        h.ok("workspace_open_synthetic", json!({}));
        let project = h.ok(
            "project_create",
            json!({ "name": "Agent study", "description": null }),
        );
        pid = project["id"].as_str().unwrap().to_owned();

        // Model selection: the bundled fixture Pack is admitted and runnable.
        let models = h.ok("medagent_models", json!({}));
        let fixture = models
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["fixture"] == true)
            .unwrap_or_else(|| panic!("{models}"))
            .clone();
        assert_eq!(fixture["runnable"], true);
        let pack_id = fixture["id"].as_str().unwrap().to_owned();

        // A relative or missing folder is refused before Core sees it.
        assert_eq!(
            h.call("medagent_admit_pack", json!({ "path": "relative/pack" }))
                .unwrap_err()["kind"],
            "invalid"
        );

        // Agent creation: name, Pack and granted tools; unknown tools refused.
        assert_eq!(
            h.call(
                "medagent_create_agent",
                json!({ "projectId": pid, "name": "x", "packId": pack_id, "tools": ["shell"] }),
            )
            .unwrap_err()["kind"],
            "invalid"
        );
        let agent = h.ok(
            "medagent_create_agent",
            json!({
                "projectId": pid,
                "name": "Synthetic summarizer",
                "packId": pack_id,
                "tools": ["read_context_artifact", "search_context_artifacts"],
            }),
        );
        assert_eq!(agent["label"], "Synthetic summarizer");
        assert_eq!(agent["pack_id"], pack_id.as_str());
        assert_eq!(agent["tools"].as_array().unwrap().len(), 2, "{agent}");
        agent_id = agent["id"].as_str().unwrap().to_owned();

        // Context creation: exactly the chosen ingested source.
        let sources = h.ok("medagent_sources", json!({}));
        let first = sources[0]["id"].as_str().unwrap().to_owned();
        assert_eq!(
            h.call(
                "medagent_create_run",
                json!({ "projectId": pid, "prompt": "p", "agentId": agent_id, "sourceIds": ["not-a-source"] }),
            )
            .unwrap_err()["kind"],
            "invalid"
        );
        let run = h.ok(
            "medagent_create_run",
            json!({
                "projectId": pid,
                "prompt": "summarize the observation",
                "agentId": agent_id,
                "sourceIds": [first],
            }),
        );
        rid = run["id"].as_str().unwrap().to_owned();
        let detail = h.ok("medagent_run_detail", json!({ "runId": rid }));
        assert_eq!(detail["agent"]["id"], agent_id.as_str());
        let evidence = detail["evidence"].as_array().unwrap();
        assert_eq!(evidence.len(), 1, "{detail}");
        assert_eq!(evidence[0]["id"], first.as_str());
        assert!(evidence[0]["fixture"].is_string(), "{detail}");

        h.ok(
            "medagent_start",
            json!({ "runId": rid, "revision": run["revision"] }),
        );
        let executed = h.ok("medagent_execute", json!({ "runId": rid }));
        assert!(executed["proposal"].is_object(), "{executed}");
        let running = h.ok("medagent_run_detail", json!({ "runId": rid }));
        let done = h.ok(
            "medagent_complete",
            json!({ "runId": rid, "revision": running["run"]["revision"] }),
        );
        assert_eq!(done["status"], "completed", "{done}");
        assert!(done["receipt"].is_object(), "{done}");
        // Closing the window releases the vault lease (a new process starts
        // with none); the mock app outlives this scope, so lock explicitly.
        h.ok("workspace_lock", json!({}));
    }

    // Restart: a new app process on the same data directory resumes the
    // session from the vault — agent, run, turns and evidence.
    let h = Harness::restart("medagent");
    h.ok("workspace_open_synthetic", json!({}));
    let runs = h.ok("medagent_runs", json!({ "projectId": pid }));
    let resumed = runs["runs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == rid.as_str())
        .unwrap_or_else(|| panic!("{runs}"))
        .clone();
    assert_eq!(resumed["status"], "completed", "{resumed}");
    assert!(
        runs["agents"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["id"] == agent_id.as_str())
    );
    assert!(
        h.ok("medagent_turns", json!({ "runId": rid }))
            .as_array()
            .unwrap()
            .len()
            >= 2
    );
    let detail = h.ok("medagent_run_detail", json!({ "runId": rid }));
    assert_eq!(detail["evidence"].as_array().unwrap().len(), 1);

    // Revocation keeps history and stops new execution on that agent.
    let revoked = h.ok(
        "medagent_revoke_agent",
        json!({ "agentId": agent_id, "revision": detail["agent"]["revision"] }),
    );
    assert_eq!(revoked["status"], "revoked", "{revoked}");
    // With every agent revoked, a run without an agent gets a new active one.
    let fresh = h.ok(
        "medagent_create_run",
        json!({ "projectId": pid, "prompt": "follow-up question" }),
    );
    let fresh_detail = h.ok("medagent_run_detail", json!({ "runId": fresh["id"] }));
    assert_eq!(fresh_detail["agent"]["status"], "active", "{fresh_detail}");
    assert_ne!(fresh_detail["agent"]["id"], agent_id.as_str());
    assert!(
        !h.ok("medagent_turns", json!({ "runId": rid }))
            .as_array()
            .unwrap()
            .is_empty()
    );
    // Model catalog: the pinned OpenMed v3.0.0 snapshot lists through Core.
    let page = h.ok("models_catalog", json!({}));
    assert_eq!(page["total_rows"], 2266, "{}", page["total_rows"]);
    assert_eq!(page["rows"].as_array().unwrap().len(), 50);
    let pii = h.ok("models_catalog", json!({ "text": "PII", "offset": 0 }));
    assert!(pii["total_matching"].as_u64().unwrap() > 0);
    assert!(pii["total_matching"].as_u64().unwrap() < 2266);

    // Evaluate: the fixture Pack on synthetic text, default and word decoding.
    let models = h.ok("medagent_models", json!({}));
    let fixture_id = models
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["fixture"] == true)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let eval = h.ok(
        "models_evaluate",
        json!({ "packId": fixture_id, "input": "alice visited clinic today" }),
    );
    assert_eq!(eval["evidence_only"], true, "{eval}");
    let worded = h.ok(
        "models_evaluate",
        json!({ "packId": fixture_id, "input": "alice visited clinic today", "decoding": "word_first" }),
    );
    assert_eq!(
        worded["proposal_payload"]["aggregation"], "word_first",
        "{worded}"
    );
    let recommended = h.ok(
        "models_evaluate",
        json!({ "packId": fixture_id, "input": "alice visited clinic today", "decoding": "recommended" }),
    );
    assert_eq!(
        recommended["proposal_payload"]["aggregation"], "word_first",
        "{recommended}"
    );
    assert_eq!(
        h.call(
            "models_evaluate",
            json!({ "packId": fixture_id, "input": "x", "decoding": "best" }),
        )
        .unwrap_err()["kind"],
        "invalid"
    );

    // Model Fleet: a lane on a chosen agent (and Pack) and sources executes
    // on that agent's Pack.
    let lane_agent = h.ok(
        "medagent_create_agent",
        json!({ "projectId": pid, "name": "Fleet reviewer", "packId": fixture_id, "tools": ["read_context_artifact"] }),
    );
    let fleet_sources = h.ok("medagent_sources", json!({}));
    let lane = h.ok(
        "fleet_create_lane",
        json!({ "projectId": pid, "role": "primary reviewer", "agentId": lane_agent["id"], "sourceIds": [fleet_sources[0]["id"]] }),
    );
    assert_eq!(lane["role_label"], "primary reviewer", "{lane}");
    let second = h.ok(
        "fleet_create_lane",
        json!({ "projectId": pid, "role": "second reviewer", "agentId": lane_agent["id"], "sourceIds": [fleet_sources[1]["id"]] }),
    );
    let fleet = h.ok(
        "fleet_create",
        json!({ "projectId": pid, "prompt": "review the bound source" }),
    );
    let dispatched = h.ok(
        "fleet_dispatch",
        json!({ "fleetId": fleet["id"], "revision": fleet["revision"], "laneIds": [lane["id"], second["id"]] }),
    );
    assert_eq!(dispatched["status"], "running", "{dispatched}");
    let executed = h.ok(
        "fleet_execute_lane",
        json!({ "fleetId": fleet["id"], "laneId": lane["id"] }),
    );
    assert!(executed["status"].is_string(), "{executed}");

    // Documents: import a synthetic FHIR file for a new subject through Core.
    let dir = std::env::temp_dir().join(format!("medscale-tauri-import-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let obs = dir.join("observation-import.json");
    std::fs::write(
        &obs,
        br#"{"resourceType":"Observation","id":"obs-import","status":"final","code":{"text":"Heart rate"},"subject":{"reference":"Patient/synthetic-import"},"valueQuantity":{"value":72,"unit":"beats/minute","system":"http://unitsofmeasure.org","code":"/min"}}"#,
    )
    .unwrap();
    let imported = h.ok(
        "documents_import_fhir",
        json!({ "path": obs.display().to_string(), "subjectRef": "synthetic-subject-import" }),
    );
    assert_eq!(imported["resource_type"], "Observation", "{imported}");
    assert_eq!(imported["claim_kind"], "observation");
    let docs = h.ok("documents_list", json!({}));
    assert!(
        docs["sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["source_id"] == imported["source_id"]),
        "{docs}"
    );
    // The imported source can bind an agent's context.
    assert!(
        h.ok("medagent_sources", json!({}))
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["id"] == imported["source_id"])
    );
    // The 1 MiB bound applies to the bytes read.
    std::fs::write(dir.join("big.json"), vec![b' '; (1 << 20) + 1]).unwrap();
    assert_eq!(
        h.call(
            "documents_import_fhir",
            json!({ "path": dir.join("big.json").display().to_string(), "subjectRef": "synthetic-subject-import" }),
        )
        .unwrap_err()["kind"],
        "invalid"
    );
    std::fs::write(dir.join("not-json.json"), b"not json").unwrap();
    assert_eq!(
        h.call(
            "documents_import_fhir",
            json!({ "path": dir.join("not-json.json").display().to_string(), "subjectRef": "synthetic-subject-import" }),
        )
        .unwrap_err()["kind"],
        "corrupt"
    );
    assert_eq!(
        h.call(
            "documents_import_fhir",
            json!({ "path": "relative.json", "subjectRef": "synthetic-subject-import" }),
        )
        .unwrap_err()["kind"],
        "invalid"
    );

    // AudioFlow (Spec 106): admit an Audio Pack, import a WAV, transcribe on
    // the local ASR route, correct a segment.
    let asr_dir = std::env::temp_dir().join(format!("medscale-tauri-asr-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&asr_dir);
    medscale_pack::asr_fixtures::synthetic_audio_pack(&asr_dir);
    let status = h.ok("audio_asr_status", json!({}));
    assert!(
        status["routes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["route"] == "local_asr_pack" && r["available"] == false),
        "{status}"
    );
    h.ok(
        "audio_admit_asr_pack",
        json!({ "path": asr_dir.display().to_string() }),
    );
    let status = h.ok("audio_asr_status", json!({}));
    assert!(
        status["routes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["route"] == "local_asr_pack" && r["available"] == true),
        "{status}"
    );
    let wav_path = dir.join("dictation.wav");
    std::fs::write(
        &wav_path,
        medscale_core::CliSession::synthetic_wav(
            medscale_contracts::audio::PcmFormat::mono_16k(),
            &[(300, false), (900, true), (300, false)],
        ),
    )
    .unwrap();
    // Desktop and Core share the same 32 MiB WAV limit. Reject one byte
    // beyond the contract before attempting to parse or persist a source.
    let oversized_wav = dir.join("oversized-recording.wav");
    std::fs::File::create(&oversized_wav)
        .unwrap()
        .set_len(medscale_contracts::audio::MAX_AUDIO_BYTES as u64 + 1)
        .unwrap();
    let err = h
        .call(
            "audio_import_wav",
            json!({ "projectId": pid, "path": oversized_wav.display().to_string() }),
        )
        .unwrap_err();
    assert_eq!(err["kind"], "invalid", "{err}");
    assert!(
        err["message"]
            .as_str()
            .unwrap_or_default()
            .contains("33554432"),
        "oversized recording must be refused at Core's exact bound: {err}"
    );
    std::fs::remove_file(&oversized_wav).unwrap();

    let imported = h.ok(
        "audio_import_wav",
        json!({ "projectId": pid, "path": wav_path.display().to_string() }),
    );
    let preview = h.ok(
        "audio_source_retention_preview",
        json!({ "projectId": pid, "sourceId": imported["source_id"] }),
    );
    assert_eq!(preview["source_id"], imported["source_id"]);
    assert_eq!(preview["source_kind"], "imported_file");
    assert!(preview["recording_bytes"].as_u64().unwrap() > 44);
    assert_eq!(preview["revision_count"], 0);
    assert_eq!(preview["receipt_count"], 0);
    assert_eq!(preview["delete_available"], true);
    assert_eq!(preview["imported_file"], true);
    assert_eq!(preview["unreferenced_imported_file"], true);
    assert_eq!(preview["has_huddle_media_link"], false);
    assert_eq!(preview["has_capture_reference"], false);
    assert_eq!(preview["active_vault_only"], true);
    assert_eq!(preview["older_backups_may_retain_recording"], true);
    assert_eq!(
        preview["linked_evidence_requires_separate_retention_policy"],
        true
    );
    assert!(
        preview["expected_digest"]
            .as_str()
            .is_some_and(|d| d.len() == 64)
    );
    assert_eq!(
        h.call(
            "audio_source_retention_preview",
            json!({ "projectId": "wrong-project", "sourceId": imported["source_id"] }),
        )
        .unwrap_err()["kind"],
        "invalid",
        "source metadata cannot be read using an unrelated selected project"
    );
    // Qualify a separate, unreferenced real WAV with the actual native
    // command. The first source remains intact for genuine Whisper ASR QA.
    let disposable = h.ok(
        "audio_import_wav",
        json!({ "projectId": pid, "path": wav_path.display().to_string() }),
    );
    let candidate = h.ok(
        "audio_source_retention_preview",
        json!({ "projectId": pid, "sourceId": disposable["source_id"] }),
    );
    assert_eq!(candidate["delete_available"], true);
    let cmd_args = json!({
        "projectId": pid, "sourceId": disposable["source_id"],
        "expectedDigest": candidate["expected_digest"],
        "confirmation": "not confirmed"
    });
    let refusal = h
        .call("audio_source_remove_confirmed", cmd_args.clone())
        .unwrap_err();
    assert_eq!(
        refusal["kind"], "invalid",
        "native removal refusal must be typed and diagnostic: {refusal}"
    );
    // No mutation occurred after a refused confirmation.
    assert_eq!(
        h.ok(
            "audio_source_retention_preview",
            json!({ "projectId": pid, "sourceId": disposable["source_id"] })
        )["expected_digest"],
        candidate["expected_digest"]
    );
    let confirmed = h.ok(
        "audio_source_remove_confirmed",
        json!({
            "projectId": pid, "sourceId": disposable["source_id"],
            "expectedDigest": candidate["expected_digest"],
            "confirmation": format!("REMOVE {} FROM ACTIVE VAULT",
                disposable["source_id"].as_str().unwrap()),
        }),
    );
    assert!(confirmed["receipt_id"].is_string());
    assert_eq!(confirmed["removed_bytes"], candidate["recording_bytes"]);
    assert_eq!(confirmed["older_backups_may_retain_recording"], true);
    assert!(
        h.call(
            "audio_source_retention_preview",
            json!({ "projectId": pid, "sourceId": disposable["source_id"] }),
        )
        .is_err()
    );
    let transcribed = h.ok(
        "audio_transcribe",
        json!({ "projectId": pid, "sourceId": imported["source_id"], "route": "local_asr_pack", "language": "en" }),
    );
    assert_eq!(transcribed["decision"]["outcome"], "allow", "{transcribed}");
    assert_eq!(transcribed["engine"]["is_fixture"], false, "{transcribed}");
    // The locally installed English-only Audio Pack must refuse Arabic.
    // This ensures language is forwarded all the way through Core's engine
    // instead of silently falling back to English.
    assert!(
        h.call(
            "audio_transcribe",
            json!({ "projectId": pid, "sourceId": imported["source_id"],
                     "route": "local_asr_pack", "language": "ar" })
        )
        .is_err(),
        "English-only Whisper Audio Pack must reject unsupported Arabic"
    );
    let revision = transcribed["transcript"]["revision_id"].clone();
    let corrected = h.ok(
        "audio_correct",
        json!({ "revisionId": revision, "seq": 1, "text": "patient has fever", "reason": "reviewed against the recording" }),
    );
    assert_eq!(corrected["revision_no"], 2, "{corrected}");
    assert_eq!(corrected["segments"][0]["text"], "patient has fever");
    let persisted = h.ok(
        "audio_source_retention_preview",
        json!({ "projectId": pid, "sourceId": imported["source_id"] }),
    );
    assert_eq!(persisted["revision_count"], 2, "{persisted}");
    assert_eq!(persisted["receipt_count"], 1, "{persisted}");
    assert_eq!(persisted["delete_available"], false);
    assert_eq!(persisted["has_transcript_revisions"], true);
    assert_eq!(persisted["has_audit_receipts"], true);
    assert_eq!(persisted["unreferenced_imported_file"], false);
    assert_eq!(persisted["expected_digest"], preview["expected_digest"]);
    // A second read must not consume or mutate lineage.
    assert_eq!(
        persisted,
        h.ok(
            "audio_source_retention_preview",
            json!({ "projectId": pid, "sourceId": imported["source_id"] }),
        )
    );
    // Draft note from the corrected revision with the fixture Pack: items
    // cite their segment; the draft stays a proposal.
    let draft = h.ok(
        "audio_draft_note",
        json!({ "revisionId": corrected["revision_id"], "packId": fixture_id }),
    );
    assert_eq!(draft["status"], "proposal", "{draft}");
    assert_eq!(draft["segments_evaluated"], 1, "{draft}");
    let items: Vec<&Value> = draft["sections"]
        .as_object()
        .unwrap()
        .values()
        .flat_map(|v| v.as_array().unwrap())
        .collect();
    assert!(!items.is_empty(), "{draft}");
    assert!(items.iter().all(|i| i["segment_seq"] == 1), "{draft}");
    // Optional speaker grouping labels every segment anonymously.
    let grouped = h.ok(
        "audio_transcribe",
        json!({ "projectId": pid, "sourceId": imported["source_id"], "route": "segmentation_only", "diarize": true }),
    );
    assert_eq!(
        grouped["transcript"]["segments"][0]["speaker"]["kind"], "anonymous",
        "{grouped}"
    );
    assert_eq!(
        h.call(
            "audio_transcribe",
            json!({ "projectId": pid, "sourceId": imported["source_id"], "route": "cloud" })
        )
        .unwrap_err()["kind"],
        "invalid"
    );
    // Reject arbitrary or malformed UI language values before model execution.
    for invalid_language in ["en-US", "AR", "<|ar|>", ""] {
        assert_eq!(
            h.call(
                "audio_transcribe",
                json!({ "projectId": pid, "sourceId": imported["source_id"],
                         "route": "local_asr_pack", "language": invalid_language })
            )
            .unwrap_err()["kind"],
            "invalid",
            "malformed language {invalid_language:?} must be refused"
        );
    }
    // Recovery path: previously downloaded app-owned model Packs are
    // re-admitted via the SAME signed Core boundary, not merely trusted
    // because an app-data directory happens to contain a manifest.
    let stored = h
        ._app
        .state::<crate::host::Host>()
        .state
        .lock()
        .unwrap()
        .data_dir()
        .unwrap()
        .join("audio-models")
        .join("whisper-tiny-en");
    std::fs::create_dir_all(&stored).unwrap();
    // The native command must refuse any existing app-owned cache before
    // spawning an HTTP worker, even if a user approves a second download.
    // This checks the actual named IPC handler; no network is ever opened.
    assert_eq!(
        h.call(
            "audio_model_download_start",
            json!({ "modelId": "whisper-tiny-en", "consent": true })
        )
        .unwrap_err()["kind"],
        "conflict",
        "a partial cached model cannot be silently overwritten"
    );
    std::fs::write(stored.join("pack.manifest.json"), b"{}").unwrap();
    let rejected = h.call(
        "audio_model_activate_existing",
        json!({ "modelId": "whisper-tiny-en" }),
    );
    assert!(
        rejected.is_err(),
        "Core must reject an unsigned saved model"
    );
    assert_eq!(
        h.call(
            "audio_model_activate_existing",
            json!({ "modelId": "../../arbitrary" })
        )
        .unwrap_err()["kind"],
        "invalid",
        "model IDs may never become arbitrary filesystem paths"
    );
    std::fs::remove_dir_all(&stored).unwrap();
    medscale_pack::asr_fixtures::synthetic_audio_pack(&stored);
    let catalog = h.ok("audio_model_catalog", json!({}));
    assert!(
        catalog
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["id"] == "whisper-tiny-en" && m["pack_files_present"] == true),
        "downloaded Pack must remain discoverable after relaunch: {catalog}"
    );
    // The signed synthetic Audio Pack correctly identifies the pinned
    // Whisper tiny.en provenance; change only an app-owned test copy of
    // model.meta.json to simulate a mislabeled local model. This must fail
    // before reaching Core; then restore exact original bytes for admission.
    let provenance_path = stored.join("model.meta.json");
    let original_provenance = std::fs::read(&provenance_path).unwrap();
    let mut altered: Value = serde_json::from_slice(&original_provenance).unwrap();
    altered["repository"] = json!("openai/whisper-tiny");
    std::fs::write(&provenance_path, serde_json::to_vec(&altered).unwrap()).unwrap();
    let wrong_source = h.call(
        "audio_model_activate_existing",
        json!({ "modelId": "whisper-tiny-en" }),
    );
    assert_eq!(
        wrong_source.unwrap_err()["kind"],
        "denied",
        "even an otherwise signed Pack cannot masquerade as another pinned model"
    );
    std::fs::write(&provenance_path, original_provenance).unwrap();
    // A genuine signed Pack with matching provenance now passes both the
    // allowlist preflight and Core's independent signature/digest admission.
    let recovered = h.ok(
        "audio_model_activate_existing",
        json!({ "modelId": "whisper-tiny-en" }),
    );
    assert_eq!(recovered["installed"], true, "{recovered}");

    // Restart again: Core re-verifies and restores the remembered Audio
    // Pack without any network request.
    h.ok("workspace_lock", json!({}));
    {
        let again = Harness::restart("medagent");
        again.ok("workspace_open_synthetic", json!({}));
        let status = again.ok("audio_asr_status", json!({}));
        assert!(
            status["routes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["route"] == "local_asr_pack" && r["available"] == true),
            "local ASR resumes after restart: {status}"
        );
        again.ok("workspace_lock", json!({}));
    }
    h.ok("workspace_open_synthetic", json!({}));

    // Exports: a subject's loss-aware FHIR export appends its disclosure.
    let before = h.ok("governance_overview", json!({}))["audit_rows"]
        .as_array()
        .map_or(0, Vec::len);
    let exported = h.ok(
        "exports_subject",
        json!({ "subjectRef": "synthetic-subject-ada", "purpose": "synthetic interchange review" }),
    );
    assert!(
        !exported["exports"].as_array().unwrap().is_empty(),
        "{exported}"
    );
    assert_eq!(exported["disclosure"]["scope"], "fhir_r4_loss_aware_export");
    let after = h.ok("governance_overview", json!({}))["audit_rows"]
        .as_array()
        .map_or(0, Vec::len);
    assert_eq!(
        after,
        before + 1,
        "the disclosure appears in the Audit Trail"
    );
    assert_eq!(
        h.call(
            "exports_subject",
            json!({ "subjectRef": "nobody", "purpose": "x" })
        )
        .unwrap_err()["kind"],
        "missing"
    );

    // Reads the MedAgent page makes, recorded for visual QA replay.
    for (cmd, args) in [
        ("workspace_status", json!({})),
        ("get_shell_status", json!({})),
        ("projects_list", json!({})),
        ("medagent_runs", json!({ "projectId": pid })),
        ("medagent_models", json!({})),
        ("medagent_sources", json!({})),
        ("models_overview", json!({})),
        ("models_catalog", json!({})),
        ("medagent_run_detail", json!({ "runId": rid })),
    ] {
        h.ok(cmd, args);
    }
    h.dump();
}

/// Explicit macOS real-human-speech qualification, never run in routine CI.
/// A disposable hosted runner supplies verified public LibriSpeech and signed
/// immutable Whisper Pack. The production Tauri IPC and Rust Core execute;
/// MockRuntime replaces only the OS WebView (not actual GUI automation).
#[test]
#[ignore = "requires disposable macOS runner, licensed human speech and pinned real Whisper"]
fn verified_public_human_wav_transcribes_via_native_tauri() {
    if std::env::var("MEDSCALE_QUAL_REAL_MAC_AUDIO").as_deref() != Ok("1") {
        return;
    }
    if std::env::consts::OS != "macos" {
        panic!("must run on a macOS hosted runner");
    }
    let wav = std::env::var("MEDSCALE_E2E_REAL_WAV_PATH")
        .expect("source-verified LibriSpeech WAV required");
    let pack = std::env::var("MEDSCALE_E2E_REAL_ASR_PACK_PATH")
        .expect("pinned signed Whisper Pack required");
    assert!(std::path::Path::new(&wav).is_file());
    assert!(
        std::path::Path::new(&pack)
            .join("pack.manifest.json")
            .is_file()
    );

    let h = Harness::new("mac-real-human-audio");
    h.call("workspace_open_synthetic", json!({}))
        .unwrap_or_else(|_| panic!("macOS workspace open failed"));
    let project = h
        .call(
            "project_create",
            json!({"name": "Public real audio qualification", "description": null}),
        )
        .unwrap_or_else(|_| panic!("macOS project creation failed"));
    let project_id = project["id"].as_str().expect("project ID");
    h.call("audio_admit_asr_pack", json!({"path": pack}))
        .unwrap_or_else(|_| panic!("native Core refused verified Pack"));
    let imported = h
        .call(
            "audio_import_wav",
            json!({"projectId": project_id, "path": wav}),
        )
        .unwrap_or_else(|_| panic!("native WAV import failed"));
    let source_id = imported["source_id"].as_str().expect("audio source ID");
    let transcribed = h
        .call(
            "audio_transcribe",
            json!({"projectId": project_id, "sourceId": source_id,
                   "route": "local_asr_pack", "language": "en"}),
        )
        .unwrap_or_else(|_| panic!("real Whisper transcription failed on macOS"));
    assert_eq!(transcribed["decision"]["outcome"], "allow");
    assert_eq!(transcribed["engine"]["is_fixture"], false);
    let revision = &transcribed["transcript"];
    let segments = revision["segments"]
        .as_array()
        .expect("transcript segments");
    assert!(
        segments
            .iter()
            .any(|s| s["text"].as_str().is_some_and(|t| t.trim().len() > 8)),
        "public human WAV did not produce local Whisper text"
    );
    assert!(revision["revision_id"].is_string());
    // A real model returning text is not enough: source bytes, transcript
    // revision and audit receipt must survive an actual app workspace restart.
    // Reuse the merged project-scoped, read-only native storage inspection.
    let before_restart = h.ok(
        "audio_source_retention_preview",
        json!({"projectId": project_id, "sourceId": source_id}),
    );
    assert_eq!(before_restart["source_id"], source_id);
    assert_eq!(before_restart["revision_count"], 1);
    assert_eq!(before_restart["receipt_count"], 1);
    assert!(before_restart["recording_bytes"].as_u64().unwrap() > 44);
    assert_eq!(before_restart["delete_available"], false);

    h.call("workspace_lock", json!({}))
        .unwrap_or_else(|_| panic!("macOS workspace failed to seal"));
    let restored = Harness::restart("mac-real-human-audio");
    restored
        .call("workspace_open_synthetic", json!({}))
        .unwrap_or_else(|_| panic!("macOS workspace did not reopen"));
    let status = restored
        .call("audio_asr_status", json!({}))
        .unwrap_or_else(|_| panic!("ASR status unavailable after restart"));
    assert!(
        status["routes"].as_array().is_some_and(|routes| routes
            .iter()
            .any(|r| r["route"] == "local_asr_pack" && r["available"] == true)),
        "native macOS did not restore the signed Whisper engine"
    );
    let after_restart = restored.ok(
        "audio_source_retention_preview",
        json!({"projectId": project_id, "sourceId": source_id}),
    );
    assert_eq!(
        after_restart, before_restart,
        "restart changed stored real source, transcript or receipt lineage"
    );
    // Only aggregate proof; do not print voice, transcript, speaker or paths.
    println!("MAC_NATIVE_REAL_HUMAN_ASR_OK clips=1 model=whisper-tiny-en restart=true");
}
