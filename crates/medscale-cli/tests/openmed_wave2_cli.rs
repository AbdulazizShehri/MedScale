use std::process::Command;

fn medscale() -> Command {
    Command::new(env!("CARGO_BIN_EXE_medscale"))
}

#[test]
fn cli_wave2_privacy_clinical_and_units_work_end_to_end() {
    let presets = medscale()
        .args(["privacy", "presets", "--json"])
        .output()
        .expect("presets");
    assert_eq!(
        presets.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&presets.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&presets.stdout).expect("json");
    assert_eq!(v["presets"].as_array().unwrap().len(), 25);

    let shift = medscale()
        .args([
            "privacy",
            "shift-date",
            "--calendar",
            "bikram_sambat",
            "--date",
            "2083-08-12",
            "--days",
            "14",
            "--json",
        ])
        .output()
        .expect("shift-date");
    assert_eq!(
        shift.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&shift.stderr)
    );
    let sv: serde_json::Value = serde_json::from_slice(&shift.stdout).expect("shift json");
    assert_eq!(sv["shifted"], "2083-08-26");

    let deid = medscale()
        .args([
            "privacy",
            "deidentify-text",
            "--text",
            "SSN 123-45-6789 on 2026-11-28",
            "--date-shift-days",
            "7",
            "--json",
        ])
        .output()
        .expect("deidentify-text");
    assert_eq!(
        deid.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&deid.stderr)
    );
    let dv: serde_json::Value = serde_json::from_slice(&deid.stdout).expect("deid json");
    assert_eq!(dv["receipt_verified"], true);
    assert!(dv["redacted_text"].as_str().unwrap().contains("2026-12-05"));

    let clin = medscale()
        .args([
            "clinical",
            "analyze-note",
            "--text",
            "Medications:\nLisinopril 10 mg tablet once daily",
            "--json",
        ])
        .output()
        .expect("analyze-note");
    assert_eq!(
        clin.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&clin.stderr)
    );
    let cv: serde_json::Value = serde_json::from_slice(&clin.stdout).expect("clinical json");
    assert_eq!(cv["parsed_sigs"].as_array().unwrap().len(), 1);

    let units = medscale()
        .args([
            "units",
            "convert",
            "--value",
            "90",
            "--from",
            "mg/dL",
            "--to",
            "mmol/L",
            "--analyte",
            "glucose",
            "--json",
        ])
        .output()
        .expect("units convert");
    assert_eq!(
        units.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&units.stderr)
    );
    let uv: serde_json::Value = serde_json::from_slice(&units.stdout).expect("units json");
    assert_eq!(uv["conversion"]["status"], "ok");
}
