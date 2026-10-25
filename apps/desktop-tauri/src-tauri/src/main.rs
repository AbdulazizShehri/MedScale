#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

/// Serializes the listed fields of a view-model into a JSON object with the
/// same snake_case keys, so DTOs stay a literal projection of Core rows.
#[macro_export]
macro_rules! obj {
    ($v:expr => $($f:ident),* $(,)?) => {
        serde_json::json!({ $( stringify!($f): &$v.$f ),* })
    };
}

mod audio_model_manager;
mod cmd_core;
mod cmd_ops;
mod cmd_research;
mod host;
mod navigation_policy;
mod startup_failure;
mod synthesis;

#[cfg(test)]
mod feature_tests;
#[cfg(test)]
mod permission_tests;

/// Every native command the frontend may call. `build.rs` declares the same
/// list in the app manifest and `capabilities/default.json` grants it to the
/// main window only; anything else is refused before it reaches Rust.
fn presentation_builder<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
        .manage(host::Host::default())
        .invoke_handler(tauri::generate_handler![
            cmd_core::get_shell_status,
            cmd_core::workspace_status,
            cmd_core::workspace_open_synthetic,
            cmd_core::workspace_create_encrypted,
            cmd_core::workspace_unlock,
            cmd_core::workspace_lock,
            cmd_core::patients_list,
            cmd_core::patient_detail,
            cmd_core::documents_list,
            cmd_core::documents_import_fhir,
            cmd_core::exports_subject,
            cmd_core::evidence_corpus,
            cmd_core::evidence_search,
            cmd_core::insights_overview,
            cmd_core::insights_ask,
            cmd_core::governance_overview,
            cmd_core::about_info,
            cmd_research::projects_list,
            cmd_research::project_detail,
            cmd_research::project_create,
            cmd_research::project_archive,
            cmd_research::experiment_create,
            cmd_research::data_sources,
            cmd_research::data_add_sample_source,
            cmd_research::data_snapshots,
            cmd_research::data_import_snapshot,
            cmd_research::data_page,
            cmd_research::data_transform,
            cmd_research::analytics_receipts,
            cmd_research::analytics_query,
            cmd_research::knowledge_overview,
            cmd_research::knowledge_build,
            cmd_research::knowledge_search,
            cmd_research::browse_overview,
            cmd_research::browse_allow,
            cmd_research::browse_disable,
            cmd_research::browse_fetch,
            cmd_research::research_os_rows,
            cmd_research::research_os_act,
            cmd_research::privacy_overview,
            cmd_research::privacy_check_egress,
            cmd_ops::models_overview,
            cmd_ops::models_admit_fixture_pack,
            cmd_ops::medagent_runs,
            cmd_ops::medagent_turns,
            cmd_ops::medagent_create_run,
            cmd_ops::medagent_start,
            cmd_ops::medagent_cancel,
            cmd_ops::medagent_execute,
            cmd_ops::medagent_models,
            cmd_ops::medagent_admit_pack,
            cmd_ops::medagent_sources,
            cmd_ops::medagent_create_agent,
            cmd_ops::medagent_revoke_agent,
            cmd_ops::medagent_run_detail,
            cmd_ops::medagent_complete,
            cmd_ops::models_catalog,
            cmd_ops::models_evaluate,
            cmd_ops::fleet_overview,
            cmd_ops::fleet_setup_lanes,
            cmd_ops::fleet_create_lane,
            cmd_ops::fleet_create,
            cmd_ops::fleet_dispatch,
            cmd_ops::fleet_execute_lane,
            cmd_ops::fleet_cancel,
            cmd_ops::fleet_detail,
            cmd_ops::fleet_compare,
            cmd_ops::collab_rooms,
            cmd_ops::collab_create_room,
            cmd_ops::collab_room_detail,
            cmd_ops::collab_open_thread,
            cmd_ops::collab_messages,
            cmd_ops::collab_post_message,
            cmd_ops::collab_create_task,
            cmd_ops::collab_complete_task,
            cmd_ops::workflow_overview,
            cmd_ops::audio_overview,
            cmd_ops::audio_import_synthetic,
            cmd_ops::audio_segment,
            cmd_ops::audio_asr_status,
            cmd_ops::audio_model_catalog,
            cmd_ops::audio_model_preflight,
            cmd_ops::audio_model_download_start,
            cmd_ops::audio_model_download_status,
            cmd_ops::audio_model_download_cancel,
            cmd_ops::audio_admit_asr_pack,
            cmd_ops::audio_import_wav,
            cmd_ops::audio_transcribe,
            cmd_ops::audio_correct,
            cmd_ops::audio_draft_note,
        ])
}

fn main() {
    startup_failure::install_panic_hook();
    presentation_builder(tauri::Builder::default().plugin(tauri_plugin_dialog::init()))
        .setup(|app| {
            let data_dir = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            if let Ok(mut state) = app.state::<host::Host>().state.lock() {
                state.data_dir = Some(data_dir);
            }
            let config = app
                .config()
                .app
                .windows
                .first()
                .ok_or("missing main window")?;
            let builder = tauri::WebviewWindowBuilder::from_config(app, config)?;
            // Test-only: the E2E build exposes WebView2's remote-debugging
            // port to Edge WebDriver (wry's default flags are kept). Release
            // builds never enable `e2e-webdriver`.
            #[cfg(all(windows, feature = "e2e-webdriver"))]
            let builder = builder.additional_browser_args(
                "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --remote-debugging-port=39517",
            );
            builder
                .on_navigation(|url| {
                    navigation_policy::is_local_application_url(url, cfg!(debug_assertions))
                })
                .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
                .on_page_load(|_, payload| {
                    if payload.event() == tauri::webview::PageLoadEvent::Finished {
                        startup_failure::mark_ready();
                    }
                })
                // InPrivate keeps no browsing state on disk. The test-only
                // `e2e-webdriver` build turns it off because WebView2 then
                // writes no DevTools port file for Edge WebDriver; release
                // builds never enable that feature.
                .incognito(!cfg!(feature = "e2e-webdriver"))
                .build()?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .unwrap_or_else(|error| startup_failure::report_and_exit(&error.to_string()))
        .run(|app, event| {
            // Seal an open encrypted vault before the process exits; managed
            // state is not guaranteed to be dropped on exit.
            if let tauri::RunEvent::Exit = event
                && let Ok(mut state) = app.state::<host::Host>().state.lock()
            {
                if let Some(ws) = state.workspace.as_mut() {
                    let _ = ws.close();
                }
                state.workspace = None;
            }
        });
}
