//! Spec 105 phase A: a `text-generation` catalog row whose snapshot ships
//! `model.safetensors` builds a signed generation Pack, admits, prepares and
//! generates. Synthetic weights (randomly initialised tiny Qwen2) and
//! synthetic Hub metadata; no network.

use std::path::PathBuf;

use candle_core::{DType, Device};
use candle_nn::{VarBuilder, VarMap};
use candle_transformers::models::qwen2;
use medscale_pack::{
    CANDLE_TEXT_GENERATION_RUNTIME_ID, HfRepoMetadata, ModelCatalog, TextGenerationSnapshot,
    admit_pack_dir, build_text_generation_pack, prepare_text_generator, reproducibility_hash,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const COMMIT: &str = "ea920f36fadd7b45935247d639f0ffa1ef493b23";
const SHA: &str = "3c914174cf099fda326b43084f679ad31425f88c";
const REPO: &str = "OpenMed/Synthetic-Generator";

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn git_blob(b: &[u8]) -> String {
    let mut h = sha1::Sha1::new();
    h.update(format!("blob {}\0", b.len()).as_bytes());
    h.update(b);
    hex(&h.finalize())
}

fn dir(tag: &str) -> PathBuf {
    // Unique per call: the tests run in parallel in one process.
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("medscale-105-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn tokenizer() -> Vec<u8> {
    let vocab = [
        "<pad>", "<s>", "</s>", "<unk>", "patient", "has", "fever", "and", "cough", "today", ".",
        "no",
    ];
    let vocab: serde_json::Map<String, Value> = vocab
        .iter()
        .enumerate()
        .map(|(i, t)| ((*t).to_owned(), json!(i)))
        .collect();
    json!({
        "version": "1.0", "truncation": null, "padding": null, "added_tokens": [],
        "normalizer": null, "pre_tokenizer": {"type": "WhitespaceSplit"},
        "post_processor": null,
        "decoder": {"type": "WordPiece", "prefix": "##", "cleanup": false},
        "model": {"type": "WordLevel", "vocab": vocab, "unk_token": "<unk>"}
    })
    .to_string()
    .into_bytes()
}

fn snapshot(task: &str) -> (PathBuf, HfRepoMetadata, ModelCatalog) {
    let config = json!({
        "model_type": "qwen2", "architectures": ["Qwen2ForCausalLM"],
        "vocab_size": 12, "hidden_size": 16, "intermediate_size": 32,
        "num_hidden_layers": 1, "num_attention_heads": 2, "num_key_value_heads": 1,
        "max_position_embeddings": 64, "sliding_window": 64, "max_window_layers": 1,
        "tie_word_embeddings": false, "rope_theta": 10000.0, "rms_norm_eps": 1e-6,
        "use_sliding_window": false, "hidden_act": "silu", "eos_token_id": 2
    });
    let config_bytes = serde_json::to_vec(&config).unwrap();
    let tokenizer = tokenizer();
    let snapshot = dir(&format!("snapshot-{task}"));
    let varmap = VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, DType::F32, &Device::Cpu);
    let cfg: qwen2::Config = serde_json::from_value(config).unwrap();
    qwen2::ModelForCausalLM::new(&cfg, vb).unwrap();
    varmap.save(snapshot.join("model.safetensors")).unwrap();
    let weights = std::fs::read(snapshot.join("model.safetensors")).unwrap();
    std::fs::write(snapshot.join("config.json"), &config_bytes).unwrap();
    std::fs::write(snapshot.join("tokenizer.json"), &tokenizer).unwrap();
    let meta: HfRepoMetadata = serde_json::from_value(json!({
        "id": REPO, "sha": SHA, "lastModified": "2026-07-13T07:43:16.000Z",
        "gated": false, "private": false,
        "siblings": [
            {"rfilename": "config.json", "size": config_bytes.len(), "blobId": git_blob(&config_bytes)},
            {"rfilename": "model.safetensors", "size": weights.len(),
             "lfs": {"sha256": hex(&Sha256::digest(&weights)), "size": weights.len()}},
            {"rfilename": "tokenizer.json", "size": tokenizer.len(), "blobId": git_blob(&tokenizer)}
        ]
    }))
    .unwrap();
    let row = format!(
        r#"{{"repo_id":"{REPO}","family":"General","task":"{task}","architecture":"qwen","formats":["pytorch"],"license":"apache-2.0","reproducibility_hash":"{}"}}"#,
        reproducibility_hash(&meta)
    );
    let catalog = ModelCatalog::import("maziyarpanahi/openmed", COMMIT, row.as_bytes()).unwrap();
    (snapshot, meta, catalog)
}

#[test]
fn generation_snapshot_builds_admits_prepares_and_generates() {
    let (snapshot, meta, catalog) = snapshot("text-generation");
    let out = dir("pack");
    let pack_id = build_text_generation_pack(
        &TextGenerationSnapshot {
            meta: &meta,
            row: catalog.get(REPO).unwrap(),
            snapshot_dir: &snapshot,
            weight_file: "model.safetensors",
        },
        &out,
    )
    .expect("verified generation snapshot builds a Pack");
    assert_eq!(pack_id, "hf-synthetic-generator-generation");
    let manifest = admit_pack_dir(&out).expect("signed Pack admits");
    assert!(
        manifest
            .runtime_requirements
            .starts_with(CANDLE_TEXT_GENERATION_RUNTIME_ID)
    );
    let generator = prepare_text_generator(&out, &manifest).expect("prepares");
    assert_eq!(generator.provenance().task, "text-generation");
    assert_eq!(generator.provenance().revision, SHA);
    assert_eq!(generator.context_limit(), 64);
    let output = generator
        .generate(&manifest.pack_id, "patient has fever", 6)
        .expect("generates");
    assert!(output.evidence_only);
    assert_eq!(output.proposal_payload["prompt_tokens"], 3);
    assert!(
        output.proposal_payload["generated_tokens"]
            .as_u64()
            .unwrap()
            <= 6
    );

    // A post-admission swap of the weights fails closed at prepare.
    std::fs::write(out.join("model.safetensors"), b"not the admitted weights").unwrap();
    assert!(prepare_text_generator(&out, &manifest).is_err());
    let _ = std::fs::remove_dir_all(out);
    let _ = std::fs::remove_dir_all(snapshot);
}

#[test]
fn generation_packs_need_a_generation_row_and_safetensors() {
    let (snap, meta, catalog) = snapshot("token-classification");
    let input = TextGenerationSnapshot {
        meta: &meta,
        row: catalog.get(REPO).unwrap(),
        snapshot_dir: &snap,
        weight_file: "model.safetensors",
    };
    assert!(build_text_generation_pack(&input, &dir("refused")).is_err());
    let (snapshot2, meta2, catalog2) = snapshot("text-generation");
    let pickle = TextGenerationSnapshot {
        meta: &meta2,
        row: catalog2.get(REPO).unwrap(),
        snapshot_dir: &snapshot2,
        weight_file: "pytorch_model.bin",
    };
    assert!(build_text_generation_pack(&pickle, &dir("refused2")).is_err());
    let _ = std::fs::remove_dir_all(snap);
    let _ = std::fs::remove_dir_all(snapshot2);
}
