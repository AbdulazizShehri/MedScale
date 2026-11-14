//! Help & Feedback. Opens one of a fixed set of public pages in the default
//! browser, and checks for a newer release, each only when the person clicks.
//! The page passes a name, never a URL, so nothing else can be opened. No logs,
//! files, paths or workspace data are sent anywhere: an issue form receives only
//! the version and operating system, and the person writes the rest.

use std::io::Read;
use std::time::Duration;

use serde_json::{Value, json};

use crate::host::{CmdError, CmdResult};

const REPO: &str = "https://github.com/AbdulazizShehri/MedScale";
const DOWNLOAD: &str = "https://medscale.si/download/";
const RELEASES_API: &str =
    "https://api.github.com/repos/AbdulazizShehri/MedScale/releases?per_page=30";
const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Set by the release build, such as `v0.1.0-beta.1`. Absent in other builds.
const RELEASE_TAG: Option<&str> = option_env!("MEDSCALE_RELEASE_TAG");

fn platform() -> String {
    let os = match std::env::consts::OS {
        "windows" => "Windows",
        "macos" => "macOS",
        "linux" => "Linux",
        other => other,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => other,
    };
    format!("{os} {arch}")
}

/// Encodes everything but letters, digits and `-._~`, as URL queries expect.
fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn version_label() -> String {
    RELEASE_TAG.map_or_else(|| VERSION.to_owned(), str::to_owned)
}

/// The page for a named destination, or none for an unknown name.
pub fn help_url(target: &str) -> Option<String> {
    let prefilled = |template: &str| {
        format!(
            "{REPO}/issues/new?template={template}&version={}&os={}",
            encode(&version_label()),
            encode(&platform())
        )
    };
    Some(match target {
        "bug" => prefilled("bug_report.yml"),
        "feature" => format!("{REPO}/issues/new?template=feature_request.yml"),
        "install" => prefilled("installation_update.yml"),
        "accessibility" => prefilled("accessibility.yml"),
        "security" => format!("{REPO}/security/advisories/new"),
        "known" => format!("{REPO}/issues?q=is%3Aissue+is%3Aopen+label%3Abug"),
        "changelog" => format!("{REPO}/blob/main/CHANGELOG.md"),
        "releases" => format!("{REPO}/releases"),
        "download" => DOWNLOAD.to_owned(),
        _ => return None,
    })
}

fn allowed(url: &str) -> bool {
    url.starts_with(&format!("{REPO}/")) || url.starts_with(DOWNLOAD)
}

fn launch(url: &str) -> std::io::Result<()> {
    #[cfg(windows)]
    let mut command = {
        let mut c = std::process::Command::new("rundll32.exe");
        c.args(["url.dll,FileProtocolHandler", url]);
        c
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut c = std::process::Command::new("open");
        c.arg(url);
        c
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(url);
        c
    };
    command.spawn().map(|_| ())
}

/// Opens a named Help & Feedback page in the default browser.
#[tauri::command(async)]
pub fn help_open(target: String) -> CmdResult<Value> {
    let url = help_url(&target).ok_or_else(|| CmdError::invalid("Invalid: unknown help page"))?;
    if !allowed(&url) {
        return Err(CmdError::invalid("Invalid: help page outside MedScale"));
    }
    launch(&url).map_err(|_| CmdError::unavailable("Unavailable: couldn't open your browser"))?;
    Ok(json!({ "url": url }))
}

type VersionKey = (u64, u64, u64, u64);

/// `v1.2.3-beta.4` or `v1.2.3`. A stable release sorts after its betas.
fn tag_key(tag: &str) -> Option<VersionKey> {
    let rest = tag.strip_prefix('v')?;
    let (core, beta) = match rest.split_once("-beta.") {
        Some((core, beta)) => (core, beta.parse::<u64>().ok()?),
        None => (rest, u64::MAX),
    };
    let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
    let key = (parts.next()??, parts.next()??, parts.next()??, beta);
    parts.next().is_none().then_some(key)
}

fn version_key(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.split('.').map(|p| p.parse::<u64>().ok());
    let key = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(key)
}

/// What a releases listing says about the installed version. `newer` is null
/// when the installed build has no release tag and the base versions match.
pub fn compare_releases(releases: &Value, version: &str, tag: Option<&str>) -> Value {
    let latest = releases
        .as_array()
        .into_iter()
        .flatten()
        .filter(|r| r["draft"] != true)
        .filter_map(|r| {
            let name = r["tag_name"].as_str()?;
            Some((tag_key(name)?, name.to_owned()))
        })
        .max();
    let Some((latest_key, latest_tag)) = latest else {
        return json!({ "current": tag.unwrap_or(version), "latest": null, "newer": false });
    };
    let newer = match tag.and_then(tag_key) {
        Some(current) => json!(latest_key > current),
        None => match version_key(version) {
            Some((a, b, c)) if (latest_key.0, latest_key.1, latest_key.2) > (a, b, c) => {
                json!(true)
            }
            Some((a, b, c)) if (latest_key.0, latest_key.1, latest_key.2) < (a, b, c) => {
                json!(false)
            }
            _ => Value::Null,
        },
    };
    json!({ "current": tag.unwrap_or(version), "latest": latest_tag, "newer": newer, "download": DOWNLOAD })
}

/// Asks GitHub for the release list, once, when the person clicks. Downloads and
/// installs nothing.
#[tauri::command(async)]
pub fn update_check() -> CmdResult<Value> {
    let unreachable =
        || CmdError::unavailable("Unavailable: couldn't reach GitHub. Nothing was downloaded.");
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .https_only(true)
        .proxy(None)
        .max_redirects(0)
        .timeout_global(Some(Duration::from_secs(20)))
        .user_agent(concat!(
            "MedScale/",
            env!("CARGO_PKG_VERSION"),
            " update-check"
        ))
        .build()
        .new_agent();
    let mut response = agent
        .get(RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|_| unreachable())?;
    let mut body = String::new();
    response
        .body_mut()
        .as_reader()
        .take(1024 * 1024)
        .read_to_string(&mut body)
        .map_err(|_| unreachable())?;
    let releases: Value = serde_json::from_str(&body).map_err(|_| unreachable())?;
    Ok(compare_releases(&releases, VERSION, RELEASE_TAG))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_page_is_a_fixed_medscale_page() {
        for target in [
            "bug",
            "feature",
            "install",
            "accessibility",
            "security",
            "known",
            "changelog",
            "releases",
            "download",
        ] {
            let url = help_url(target).expect(target);
            assert!(allowed(&url), "{url}");
        }
        for unknown in ["", "https://example.com", "../bug", "bug ", "BUG"] {
            assert_eq!(help_url(unknown), None, "{unknown}");
        }
    }

    #[test]
    fn issue_forms_get_only_the_version_and_os() {
        let url = help_url("bug").unwrap();
        let query = url.split_once('?').unwrap().1;
        let keys: Vec<&str> = query
            .split('&')
            .map(|p| p.split('=').next().unwrap())
            .collect();
        assert_eq!(keys, ["template", "version", "os"]);
        assert!(!query.contains(' '));
        assert_eq!(encode("Windows x64"), "Windows%20x64");
        assert_eq!(encode("v0.1.0-beta.1"), "v0.1.0-beta.1");
        assert!(
            help_url("security")
                .unwrap()
                .ends_with("/security/advisories/new"),
            "security reports stay private"
        );
    }

    #[test]
    fn releases_are_compared_by_version_not_by_listing_order() {
        let listing = json!([
            { "tag_name": "v0.1.0-beta.2", "draft": false },
            { "tag_name": "v0.1.0-beta.10", "draft": false },
            { "tag_name": "v0.2.0-beta.1", "draft": true },
            { "tag_name": "engineering-preview-19d419dd7b15", "draft": false },
            { "tag_name": "v0.1.0-beta.3x", "draft": false },
        ]);
        let result = compare_releases(&listing, "0.1.0", Some("v0.1.0-beta.2"));
        assert_eq!(result["latest"], "v0.1.0-beta.10");
        assert_eq!(result["newer"], true);

        let same = compare_releases(&listing, "0.1.0", Some("v0.1.0-beta.10"));
        assert_eq!(same["newer"], false);

        let untagged = compare_releases(&listing, "0.1.0", None);
        assert_eq!(
            untagged["newer"],
            Value::Null,
            "a build without a tag can't tell beta numbers apart"
        );
        assert_eq!(compare_releases(&listing, "0.0.9", None)["newer"], true);
        assert_eq!(compare_releases(&json!([]), "0.1.0", None)["newer"], false);
        assert_eq!(
            compare_releases(&json!({ "message": "rate limited" }), "0.1.0", None)["latest"],
            Value::Null
        );
    }

    #[test]
    fn a_stable_release_sorts_after_its_betas() {
        assert!(tag_key("v0.1.0").unwrap() > tag_key("v0.1.0-beta.99").unwrap());
        assert_eq!(tag_key("v0.1"), None);
        assert_eq!(tag_key("0.1.0"), None);
        assert_eq!(tag_key("v0.1.0.1"), None);
    }
}
