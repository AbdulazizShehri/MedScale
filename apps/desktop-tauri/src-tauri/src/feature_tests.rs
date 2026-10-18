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
        let app = super::presentation_builder(mock_builder())
            .build(tauri::generate_context!())
            .expect("build production context");
        let dir = std::env::temp_dir().join(format!(
            "medscale-tauri-feature-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
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
    assert!(
        !h.ok("insights_ask", json!({ "query": "coverage gaps" }))
            .as_str()
            .unwrap()
            .is_empty()
    );

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
