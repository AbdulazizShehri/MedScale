//! Invoke boundary tests use the production handler and generated capability context.
//! MockRuntime does not establish native WebView privacy or OS isolation.

use serde::Deserialize;
use tauri::test::{MockRuntime, get_ipc_response, mock_builder};
use tauri::webview::InvokeRequest;

fn test_app() -> tauri::App<MockRuntime> {
    super::presentation_builder(mock_builder())
        .build(tauri::generate_context!())
        .expect("build production permission context")
}

fn request(command: &str, origin: &str) -> InvokeRequest {
    InvokeRequest {
        cmd: command.to_owned(),
        callback: tauri::ipc::CallbackFn(0),
        error: tauri::ipc::CallbackFn(1),
        url: origin.parse().expect("test origin"),
        body: tauri::ipc::InvokeBody::default(),
        headers: Default::default(),
        invoke_key: tauri::test::INVOKE_KEY.to_owned(),
    }
}

fn local_origin() -> &'static str {
    if cfg!(any(windows, target_os = "android")) {
        "http://tauri.localhost"
    } else {
        "tauri://localhost"
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StatusProbe {
    schema_version: u8,
    core_connection: String,
    detail: String,
    synthetic_only: bool,
}

#[test]
fn main_local_window_receives_idle_shell_status_before_a_vault_opens() {
    let app = test_app();
    let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("main mock window");
    let response = get_ipc_response(&window, request("get_shell_status", local_origin()))
        .expect("admitted status read")
        .deserialize::<StatusProbe>()
        .expect("versioned bounded status DTO");
    assert_eq!(response.schema_version, 2);
    assert_eq!(response.core_connection, "idle");
    assert!(response.synthetic_only);
    assert!(!response.detail.is_empty() && response.detail.len() <= 256);
}

#[test]
fn remote_origins_cannot_invoke_the_admitted_status_command() {
    let app = test_app();
    let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("main mock window");
    for origin in [
        "https://example.invalid",
        "http://tauri.localhost.example.invalid",
    ] {
        assert!(get_ipc_response(&window, request("get_shell_status", origin)).is_err());
    }
}

#[test]
fn an_ungranted_window_cannot_read_shell_status() {
    let app = test_app();
    let window = tauri::WebviewWindowBuilder::new(&app, "ungranted", Default::default())
        .build()
        .expect("ungranted mock window");
    assert!(get_ipc_response(&window, request("get_shell_status", local_origin())).is_err());
}

#[test]
fn generic_privilege_and_plugin_commands_are_refused() {
    let app = test_app();
    let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("main mock window");
    for command in [
        "execute_sql",
        "run_shell",
        "run_command",
        "read_any_file",
        "write_any_file",
        "call_arbitrary_core_method",
        "plugin:fs|read_file",
        "plugin:shell|execute",
        "plugin:http|fetch",
        "plugin:clipboard-manager|read_text",
    ] {
        assert!(get_ipc_response(&window, request(command, local_origin())).is_err());
    }
}
