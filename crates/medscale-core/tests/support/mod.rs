//! Shared test support: a tiny BERT token classifier written directly in the
//! safetensors format (deterministic weights, synthetic only) and a signed
//! candle-runtime Pack built from it through the catalog snapshot path.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use medscale_pack::{
    HfRepoMetadata, ModelCatalog, TokenClassifierSnapshot, build_token_classifier_pack,
    reproducibility_hash,
};
use serde_json::json;
use sha2::{Digest, Sha256};

const COMMIT: &str = "ea920f36fadd7b45935247d639f0ffa1ef493b23";
const SHA: &str = "3c914174cf099fda326b43084f679ad31425f88c";
const REPO: &str = "OpenMed/Synthetic-Core-safetensors";
const H: usize = 8;
const I: usize = 16;

pub fn uid() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn git_blob(b: &[u8]) -> String {
    let mut h = sha1::Sha1::new();
    h.update(format!("blob {}\0", b.len()).as_bytes());
    h.update(b);
    hex(&h.finalize())
}

/// Minimal safetensors writer: little-endian header length, JSON header,
/// then raw little-endian f32 data.
pub fn safetensors(tensors: &[(String, Vec<usize>, Vec<f32>)]) -> Vec<u8> {
    let mut header = serde_json::Map::new();
    let mut data = Vec::new();
    for (name, shape, values) in tensors {
        let begin = data.len();
        for v in values {
            data.extend_from_slice(&v.to_le_bytes());
        }
        header.insert(
            name.clone(),
            json!({"dtype": "F32", "shape": shape, "data_offsets": [begin, data.len()]}),
        );
    }
    let mut header = serde_json::to_vec(&header).unwrap();
    while !header.len().is_multiple_of(8) {
        header.push(b' ');
    }
    let mut out = (header.len() as u64).to_le_bytes().to_vec();
    out.extend_from_slice(&header);
    out.extend_from_slice(&data);
    out
}

pub fn tiny_bert() -> Vec<u8> {
    let mut seed = 0u32;
    let mut values = |n: usize| -> Vec<f32> {
        (0..n)
            .map(|_| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                #[allow(clippy::cast_precision_loss)]
                let unit = (seed >> 8) as f32 / (1u32 << 24) as f32;
                (unit - 0.5) * 0.2
            })
            .collect()
    };
    let mut t: Vec<(String, Vec<usize>, Vec<f32>)> = Vec::new();
    let mut push = |name: &str, shape: Vec<usize>, v: Vec<f32>| t.push((name.into(), shape, v));
    push(
        "bert.embeddings.word_embeddings.weight",
        vec![8, H],
        values(8 * H),
    );
    push(
        "bert.embeddings.position_embeddings.weight",
        vec![16, H],
        values(16 * H),
    );
    push(
        "bert.embeddings.token_type_embeddings.weight",
        vec![2, H],
        values(2 * H),
    );
    push("bert.embeddings.LayerNorm.weight", vec![H], vec![1.0; H]);
    push("bert.embeddings.LayerNorm.bias", vec![H], vec![0.0; H]);
    let l = "bert.encoder.layer.0";
    for part in ["query", "key", "value"] {
        push(
            &format!("{l}.attention.self.{part}.weight"),
            vec![H, H],
            values(H * H),
        );
        push(
            &format!("{l}.attention.self.{part}.bias"),
            vec![H],
            values(H),
        );
    }
    push(
        &format!("{l}.attention.output.dense.weight"),
        vec![H, H],
        values(H * H),
    );
    push(
        &format!("{l}.attention.output.dense.bias"),
        vec![H],
        values(H),
    );
    push(
        &format!("{l}.attention.output.LayerNorm.weight"),
        vec![H],
        vec![1.0; H],
    );
    push(
        &format!("{l}.attention.output.LayerNorm.bias"),
        vec![H],
        vec![0.0; H],
    );
    push(
        &format!("{l}.intermediate.dense.weight"),
        vec![I, H],
        values(I * H),
    );
    push(&format!("{l}.intermediate.dense.bias"), vec![I], values(I));
    push(
        &format!("{l}.output.dense.weight"),
        vec![H, I],
        values(H * I),
    );
    push(&format!("{l}.output.dense.bias"), vec![H], values(H));
    push(
        &format!("{l}.output.LayerNorm.weight"),
        vec![H],
        vec![1.0; H],
    );
    push(&format!("{l}.output.LayerNorm.bias"), vec![H], vec![0.0; H]);
    push("classifier.weight", vec![2, H], values(2 * H));
    push("classifier.bias", vec![2], values(2));
    safetensors(&t)
}

pub fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "medscale-104-core-{tag}-{}-{}",
        std::process::id(),
        uid()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Builds a signed candle Pack from a synthetic snapshot; returns its id.
pub fn candle_pack(out: &Path) -> String {
    let config = serde_json::to_vec(&json!({
        "model_type": "bert", "vocab_size": 8, "hidden_size": H, "num_hidden_layers": 1,
        "num_attention_heads": 2, "intermediate_size": I, "hidden_act": "gelu",
        "hidden_dropout_prob": 0.0, "max_position_embeddings": 16, "type_vocab_size": 2,
        "initializer_range": 0.02, "layer_norm_eps": 1e-12, "pad_token_id": 0,
        "classifier_dropout": null, "id2label": {"0": "O", "1": "B-X"}
    }))
    .unwrap();
    let tokenizer = std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../evidence/069-real-local-model-runtime-hf-pack-path/fixtures/pack-tiny-token-classifier-v0/tokenizer.json",
    ))
    .unwrap();
    let weights = tiny_bert();
    let snapshot = dir("snapshot");
    std::fs::write(snapshot.join("config.json"), &config).unwrap();
    std::fs::write(snapshot.join("tokenizer.json"), &tokenizer).unwrap();
    std::fs::write(snapshot.join("model.safetensors"), &weights).unwrap();
    let meta: HfRepoMetadata = serde_json::from_value(json!({
        "id": REPO, "sha": SHA, "lastModified": "2026-07-13T07:43:16.000Z",
        "gated": false, "private": false,
        "siblings": [
            {"rfilename": "config.json", "size": config.len(), "blobId": git_blob(&config)},
            {"rfilename": "model.safetensors", "size": weights.len(),
             "lfs": {"sha256": hex(&Sha256::digest(&weights)), "size": weights.len()}},
            {"rfilename": "tokenizer.json", "size": tokenizer.len(), "blobId": git_blob(&tokenizer)}
        ]
    }))
    .unwrap();
    let row = format!(
        r#"{{"repo_id":"{REPO}","family":"NER","task":"token-classification","architecture":"bert","formats":["pytorch"],"license":"apache-2.0","reproducibility_hash":"{}"}}"#,
        reproducibility_hash(&meta)
    );
    let catalog = ModelCatalog::import("maziyarpanahi/openmed", COMMIT, row.as_bytes()).unwrap();
    let pack_id = build_token_classifier_pack(
        &TokenClassifierSnapshot {
            meta: &meta,
            row: catalog.get(REPO).unwrap(),
            snapshot_dir: &snapshot,
            onnx_file: "model.safetensors",
            fixed_sequence_length: 16,
        },
        out,
    )
    .unwrap();
    let _ = std::fs::remove_dir_all(snapshot);
    pack_id
}

const GLINER_REPO: &str = "OpenMed/Synthetic-Core-ZeroShot-mlx";

/// A signed candle Pack holding a synthetic GLiNER zero-shot export
/// (random weights from `medscale_pack::gliner_fixtures`), built through the
/// catalog snapshot path like a real `ZeroShot-NER-*-mlx` row.
pub fn gliner_pack(out: &Path) -> String {
    use medscale_pack::gliner_fixtures::{tiny_config, tiny_tokenizer, tiny_weights};
    let config_value = tiny_config();
    let config = serde_json::to_vec(&config_value).unwrap();
    let tokenizer = tiny_tokenizer().into_bytes();
    let weights = tiny_weights(&config_value);
    let snapshot = dir("gliner-snapshot");
    std::fs::write(snapshot.join("config.json"), &config).unwrap();
    std::fs::write(snapshot.join("tokenizer.json"), &tokenizer).unwrap();
    std::fs::write(snapshot.join("weights.safetensors"), &weights).unwrap();
    let meta: HfRepoMetadata = serde_json::from_value(json!({
        "id": GLINER_REPO, "sha": SHA, "lastModified": "2026-07-13T07:43:16.000Z",
        "gated": false, "private": false,
        "siblings": [
            {"rfilename": "config.json", "size": config.len(), "blobId": git_blob(&config)},
            {"rfilename": "weights.safetensors", "size": weights.len(),
             "lfs": {"sha256": hex(&Sha256::digest(&weights)), "size": weights.len()}},
            {"rfilename": "tokenizer.json", "size": tokenizer.len(), "blobId": git_blob(&tokenizer)}
        ]
    }))
    .unwrap();
    let row = format!(
        r#"{{"repo_id":"{GLINER_REPO}","family":"ZeroShot","task":"token-classification","architecture":"deberta-v2","formats":["mlx-fp","pytorch"],"license":"apache-2.0","reproducibility_hash":"{}"}}"#,
        reproducibility_hash(&meta)
    );
    let catalog = ModelCatalog::import("maziyarpanahi/openmed", COMMIT, row.as_bytes()).unwrap();
    let pack_id = build_token_classifier_pack(
        &TokenClassifierSnapshot {
            meta: &meta,
            row: catalog.get(GLINER_REPO).unwrap(),
            snapshot_dir: &snapshot,
            onnx_file: "weights.safetensors",
            fixed_sequence_length: 64,
        },
        out,
    )
    .unwrap();
    let _ = std::fs::remove_dir_all(snapshot);
    pack_id
}
