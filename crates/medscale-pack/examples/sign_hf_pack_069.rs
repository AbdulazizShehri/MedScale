//! Build the synthetic-signed external qualification Pack for Spec 069.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use medscale_contracts::objects::DigestSha256;
use medscale_contracts::packs::ModelSourceProvenance;
use medscale_keys::{SYNTHETIC_PACK_TRUST_ROOT_ID, pack_signing_payload, sign_pack_payload};
use serde_json::{Value, json};

const PACK_ID: &str = "hf-onnx-community-bert-base-ner-int8-static128";
const VERSION: &str = "1.0.0";
const PACK_EPOCH: u64 = 1;
const REVISION: &str = "9faa2f4a2d59b396888b318f596ff719cc893f1e";
const SOURCE_SHA: &str = "b324e829f1fad3b897f926d1a1d1372803c6d04546831a3a2ed103652b916adf";
const DERIVED_SHA: &str = "183057ca3123d7bd09fda062738390b326b1298c58d0eee39f0003f5a8920f14";
const RIGHTS_URI: &str = "https://huggingface.co/onnx-community/bert-base-NER-ONNX/blob/9faa2f4a2d59b396888b318f596ff719cc893f1e/README.md";

fn normalized_text(path: &Path) -> Vec<u8> {
    fs::read(path)
        .expect("read artifact")
        .split(|byte| *byte == b'\r')
        .enumerate()
        .fold(Vec::new(), |mut out, (index, part)| {
            if index > 0 {
                out.push(b'\n');
                if part.first() == Some(&b'\n') {
                    out.extend_from_slice(&part[1..]);
                    return out;
                }
            }
            out.extend_from_slice(part);
            out
        })
}

fn artifact_digest(root: &Path, name: &str, text: bool) -> String {
    let path = root.join(name);
    let bytes = if text {
        normalized_text(&path)
    } else {
        fs::read(&path).expect("read artifact")
    };
    DigestSha256::of(&bytes).to_hex()
}

fn main() {
    let root = env::args()
        .nth(1)
        .map(PathBuf::from)
        .expect("usage: sign_hf_pack_069 <pack-dir>");
    let provenance: ModelSourceProvenance =
        serde_json::from_slice(&fs::read(root.join("model.meta.json")).expect("model.meta.json"))
            .expect("valid model provenance");
    assert_eq!(provenance.repository, "onnx-community/bert-base-NER-ONNX");
    assert_eq!(provenance.revision, REVISION);
    assert_eq!(provenance.source_sha256, SOURCE_SHA);
    assert_eq!(provenance.fixed_sequence_length, 128);
    assert_eq!(provenance.license_id, "MIT");

    let artifacts = vec![
        ("model.onnx", "onnx_model", false),
        ("tokenizer.json", "tokenizer_meta", true),
        ("labels.json", "tokenizer_meta", true),
        ("model.meta.json", "model_metadata", true),
    ];
    let rows: Vec<Value> = artifacts
        .into_iter()
        .map(|(name, kind, text)| {
            let digest = artifact_digest(&root, name, text);
            if name == "model.onnx" {
                assert_eq!(digest, DERIVED_SHA, "derived model digest drifted");
            }
            json!({"relative_path": name, "kind": kind, "digest": digest})
        })
        .collect();
    let mut sorted = rows.clone();
    sorted.sort_by_key(|row| row["relative_path"].as_str().unwrap().to_owned());
    let digest_lines = sorted
        .iter()
        .map(|row| row["digest"].as_str().unwrap())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let content_digest = DigestSha256::of(digest_lines.as_bytes()).to_hex();
    let sbom_ref = format!("hf:onnx-community/bert-base-NER-ONNX@{REVISION}#onnx/model_int8.onnx");
    let payload = pack_signing_payload(
        PACK_ID,
        VERSION,
        PACK_EPOCH,
        &content_digest,
        RIGHTS_URI,
        &sbom_ref,
    );
    let signature_hex = sign_pack_payload(&payload);
    let manifest = json!({
        "pack_id": PACK_ID,
        "version": VERSION,
        "pack_epoch": PACK_EPOCH,
        "content_digest": content_digest,
        "artifacts": rows,
        "rights_uri": RIGHTS_URI,
        "sbom_ref": sbom_ref,
        "runtime_requirements": "tract_onnx_token_classification_v1;synthetic_only=true;fixed_sequence_length=128",
        "benchmark_links": ["evidence/069-real-local-model-runtime-hf-pack-path/HF_MODEL_QUALIFICATION.md"],
        "promotion_state": "candidate",
        "trust_root_id": SYNTHETIC_PACK_TRUST_ROOT_ID,
        "signature_hex": signature_hex,
    });
    fs::write(
        root.join("pack.manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap() + "\n",
    )
    .expect("write manifest");
    println!(
        "content_digest={}",
        manifest["content_digest"].as_str().unwrap()
    );
    println!(
        "signature_hex={}",
        manifest["signature_hex"].as_str().unwrap()
    );
}
