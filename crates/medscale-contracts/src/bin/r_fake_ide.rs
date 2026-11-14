//! Spec 086 qualification harness: a stand-in for an external IDE.
//!
//! Not shipped. Started by Core's launcher with a staged workspace as its
//! only argument, it does what a user's R session might: reads the staged
//! CSV copies and writes files into `outputs/`. It also writes the names
//! (never values) of the environment variables it received, so tests can
//! prove the launch environment was reduced to the allowlist. Finally it
//! writes `outputs/done.marker`.

use std::fs;
use std::path::PathBuf;

fn main() {
    let Some(root) = std::env::args_os().nth(1).map(PathBuf::from) else {
        std::process::exit(2);
    };
    let outputs = root.join("outputs");
    let mut names: Vec<String> = std::env::vars_os()
        .filter_map(|(k, _)| k.into_string().ok())
        .collect();
    names.sort();
    let mut report = names.join("\n");
    report.push('\n');
    if fs::write(outputs.join("ide-env.txt"), report).is_err() {
        std::process::exit(3);
    }
    // Copy the first staged table, as an analysis that keeps every row.
    let mut data: Vec<PathBuf> = fs::read_dir(root.join("data"))
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "csv"))
                .collect()
        })
        .unwrap_or_default();
    data.sort();
    let Some(first) = data.first() else {
        std::process::exit(4);
    };
    let Ok(bytes) = fs::read(first) else {
        std::process::exit(5);
    };
    if fs::write(outputs.join("result.csv"), bytes).is_err() {
        std::process::exit(6);
    }
    let _ = fs::write(outputs.join("done.marker"), b"done\n");
}
