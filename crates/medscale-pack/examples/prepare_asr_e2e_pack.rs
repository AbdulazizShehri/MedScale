//! Build one verified source-bound Audio Pack for a disposable Windows E2E runner.
//! The trusted LibriSpeech audio and real Whisper checkpoint are never committed.
use std::{env, fs, path::PathBuf};
use medscale_pack::{HfRepoMetadata, admit_pack_dir, build_asr_pack};

fn main() {
    let mut args = env::args().skip(1);
    let snapshot = PathBuf::from(args.next().expect("usage: prepare_asr_e2e_pack SNAPSHOT OUT"));
    let target = PathBuf::from(args.next().expect("missing Pack destination"));
    assert!(args.next().is_none(), "unexpected arguments");
    assert!(!target.exists(), "refusing to overwrite Pack");
    let meta: HfRepoMetadata = serde_json::from_slice(
        &fs::read(snapshot.join("metadata.json")).expect("pinned model metadata"),
    ).expect("valid Hugging Face metadata");
    assert_eq!(meta.id, "openai/whisper-tiny.en");
    assert_eq!(meta.sha, "87c7102498dcde7456f24cfd30239ca606ed9063");
    let id = build_asr_pack(&meta, &snapshot, &target).expect("verify and sign actual Audio Pack");
    let admitted = admit_pack_dir(&target).expect("verify admission");
    assert_eq!(admitted.pack_id.as_str(), id);
    println!("REAL_AUDIO_PACK_ADMITTED verified=yes model=whisper-tiny.en");
}
