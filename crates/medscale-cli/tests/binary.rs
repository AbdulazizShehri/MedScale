//! The built `medscale` binary, run as a user would: arguments, environment,
//! stdout, stderr and exit codes. Exit codes are part of the CLI contract
//! (docs/product/CLI_PRODUCT_GUIDE.md): 0 success, 1 failure, 2 usage error,
//! 3 failure reported as a JSON object on stderr (`--json`).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const PASSPHRASE: &str = "correct horse battery";

fn medscale() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_medscale"));
    // Never inherit secrets from the developer's shell.
    cmd.env_remove("MEDSCALE_PASSPHRASE")
        .env_remove("MEDSCALE_RECOVERY_CODE");
    cmd
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("medscale-bin-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn code(out: &Output) -> i32 {
    out.status.code().expect("exit code")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn json(out: &Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {}", text(&out.stdout)))
}

fn create_vault(root: &Path) -> Output {
    medscale()
        .args(["vault", "create", "--vault-id", "bin-vault", "--vault-root"])
        .arg(root)
        .env("MEDSCALE_PASSPHRASE", PASSPHRASE)
        .output()
        .unwrap()
}

#[test]
fn version_and_status_json() {
    let out = medscale().arg("--version").output().unwrap();
    assert_eq!(code(&out), 0);
    assert!(text(&out.stdout).starts_with("medscale "));

    let out = medscale().args(["status", "--json"]).output().unwrap();
    assert_eq!(code(&out), 0, "{}", text(&out.stderr));
    let status = json(&out);
    assert_eq!(status["real_phi_authorized"], false);
    assert_eq!(status["release_ready"], false);
}

#[test]
fn a_usage_error_exits_2() {
    let out = medscale().arg("no-such-command").output().unwrap();
    assert_eq!(code(&out), 2);
    let out = medscale().args(["vault", "backup"]).output().unwrap();
    assert_eq!(code(&out), 2, "missing required arguments");
}

#[test]
fn a_json_failure_exits_3_with_an_error_object_on_stderr() {
    let root = temp("json-fail");
    create_vault(&root);
    // Encrypted vault, no secret in the environment.
    let out = medscale()
        .args([
            "project",
            "list",
            "--vault-id",
            "bin-json",
            "--json",
            "--vault-root",
        ])
        .arg(&root)
        .output()
        .unwrap();
    assert_eq!(code(&out), 3, "{}", text(&out.stderr));
    assert!(out.stdout.is_empty(), "nothing on stdout");
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).expect("stderr is JSON");
    assert!(
        err["code"].is_string() && err["message"].is_string(),
        "{err}"
    );
    assert!(
        err["message"]
            .as_str()
            .unwrap()
            .contains("MEDSCALE_PASSPHRASE")
    );
}

#[test]
fn encrypted_vault_lifecycle_through_the_binary() {
    let base = temp("lifecycle");
    let root = base.join("vault");

    // Create: recovery codes once on stderr, never on stdout.
    let out = create_vault(&root);
    assert_eq!(code(&out), 0, "{}", text(&out.stderr));
    let stdout = text(&out.stdout);
    let stderr = text(&out.stderr);
    assert!(stdout.contains("vault_sealed: true"));
    let codes: Vec<String> = stderr
        .lines()
        .filter_map(|l| l.strip_prefix("  ").map(str::trim).map(str::to_owned))
        .filter(|l| !l.is_empty())
        .collect();
    assert!(!codes.is_empty(), "codes on stderr: {stderr}");
    assert!(
        codes.iter().all(|c| !stdout.contains(c.as_str())),
        "no code on stdout"
    );

    // A data command works in the encrypted vault and leaves it sealed.
    let out = medscale()
        .args([
            "project",
            "create",
            "--vault-id",
            "bin-vault",
            "--name",
            "study",
            "--json",
            "--vault-root",
        ])
        .arg(&root)
        .env("MEDSCALE_PASSPHRASE", PASSPHRASE)
        .output()
        .unwrap();
    assert_eq!(code(&out), 0, "{}", text(&out.stderr));
    assert!(
        !root.join("meta.work.sqlite3").exists(),
        "sealed after the command"
    );

    // Read back with a recovery code instead of the passphrase.
    let out = medscale()
        .args([
            "project",
            "list",
            "--vault-id",
            "bin-vault",
            "--json",
            "--vault-root",
        ])
        .arg(&root)
        .env("MEDSCALE_RECOVERY_CODE", &codes[0])
        .output()
        .unwrap();
    assert_eq!(code(&out), 0, "{}", text(&out.stderr));
    assert_eq!(json(&out)["projects"].as_array().unwrap().len(), 1);

    // FHIR export from the encrypted vault records a disclosure.
    let out = medscale()
        .args([
            "fhir",
            "export",
            "--vault-id",
            "bin-vault",
            "--subject",
            "bin-subject",
            "--purpose",
            "binary test",
            "--json",
            "--vault-root",
        ])
        .arg(&root)
        .env("MEDSCALE_PASSPHRASE", PASSPHRASE)
        .output()
        .unwrap();
    assert_eq!(code(&out), 0, "{}", text(&out.stderr));
    let exported = json(&out);
    assert!(exported["exports"].is_array(), "{exported}");
    assert!(
        exported["disclosure"]["disclosure_id"].is_string(),
        "{exported}"
    );

    // Back up, then restore over the vault: the old one is kept.
    let backups = base.join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let out = medscale()
        .args([
            "vault",
            "backup",
            "--json",
            "--label",
            "bin1",
            "--vault-root",
        ])
        .arg(&root)
        .arg("--to")
        .arg(&backups)
        .output()
        .unwrap();
    assert_eq!(code(&out), 0, "{}", text(&out.stderr));
    let made = json(&out);
    assert_eq!(made["verified"], true);
    let folder = made["backup"].as_str().unwrap().to_owned();

    let out = medscale()
        .args([
            "vault",
            "restore",
            "--json",
            "--label",
            "bin2",
            "--from",
            &folder,
            "--vault-root",
        ])
        .arg(&root)
        .output()
        .unwrap();
    assert_eq!(code(&out), 0, "{}", text(&out.stderr));
    let kept = json(&out)["previous_kept_at"].as_str().unwrap().to_owned();
    assert!(Path::new(&kept).join("meta.sealed").is_file());

    // The restored vault still opens and still holds the project.
    let out = medscale()
        .args([
            "project",
            "list",
            "--vault-id",
            "bin-vault",
            "--json",
            "--vault-root",
        ])
        .arg(&root)
        .env("MEDSCALE_PASSPHRASE", PASSPHRASE)
        .output()
        .unwrap();
    assert_eq!(code(&out), 0, "{}", text(&out.stderr));
    assert_eq!(json(&out)["projects"].as_array().unwrap().len(), 1);

    // A wrong passphrase fails with exit 1 and changes nothing.
    let out = medscale()
        .args(["vault", "open", "--vault-id", "bin-vault", "--vault-root"])
        .arg(&root)
        .env("MEDSCALE_PASSPHRASE", "not the passphrase")
        .output()
        .unwrap();
    assert_eq!(code(&out), 1);
    let _ = std::fs::remove_dir_all(&base);
}
