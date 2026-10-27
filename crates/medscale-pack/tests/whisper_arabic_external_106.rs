//! Public Arabic real-audio benchmark using pinned FLEURS ar_eg (CC BY 4.0).
//! Strictly a real-world language capability measure, not patient-data training.
//! Nothing about human voices, utterances, speaker IDs or audio is logged.
use medscale_pack::{
    HfRepoMetadata, admit_pack_dir, build_asr_pack, pcm16_to_mono_f32, prepare_whisper_asr,
};
use std::path::Path;
fn required(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set"))
}
fn unit_words(input: &str) -> Vec<String> {
    input
        .split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|c| c.is_alphanumeric() && !('\u{064b}'..='\u{065f}').contains(c))
                .collect::<String>()
                .to_lowercase()
        })
        .filter(|word| !word.is_empty())
        .collect()
}
fn edits<T: PartialEq>(r: &[T], h: &[T]) -> usize {
    let mut prev: Vec<usize> = (0..=h.len()).collect();
    for (i, rv) in r.iter().enumerate() {
        let mut cur = vec![i + 1; h.len() + 1];
        for (j, hv) in h.iter().enumerate() {
            cur[j + 1] = (prev[j] + usize::from(rv != hv))
                .min(prev[j + 1] + 1)
                .min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[h.len()]
}
fn wav(bytes: &[u8]) -> Vec<f32> {
    assert!(bytes.len() > 44 && &bytes[..4] == b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    let (mut channels, mut rate, mut bits) = (0u16, 0u32, 0u16);
    let mut i = 12usize;
    while i + 8 <= bytes.len() {
        let size = u32::from_le_bytes(bytes[i + 4..i + 8].try_into().unwrap()) as usize;
        let end = i + 8 + size;
        assert!(end <= bytes.len(), "truncated RIFF");
        let b = &bytes[i + 8..end];
        if &bytes[i..i + 4] == b"fmt " {
            channels = u16::from_le_bytes([b[2], b[3]]);
            rate = u32::from_le_bytes(b[4..8].try_into().unwrap());
            bits = u16::from_le_bytes(b[14..16].try_into().unwrap());
        } else if &bytes[i..i + 4] == b"data" {
            assert_eq!((channels, rate, bits), (1, 16_000, 16));
            return pcm16_to_mono_f32(b, channels).expect("valid PCM16");
        }
        i = end + (size & 1);
    }
    panic!("missing PCM data");
}
#[test]
fn score_handles_non_latin_words() {
    let r = unit_words("المريض لديه حمى اليوم");
    let h = unit_words("المريض لديه حمى");
    assert_eq!(edits(&r, &h), 1);
}
#[test]
#[ignore = "requires pinned public FLEURS actual voice clips and multilingual Whisper snapshot"]
fn actual_arabic_fleurs_transcription() {
    let root = std::path::PathBuf::from(required("MEDSCALE_ASR_SNAPSHOT_DIR"));
    let metadata: HfRepoMetadata = serde_json::from_slice(
        &std::fs::read(root.join("metadata.json")).expect("pinned Hugging Face metadata"),
    )
    .expect("valid metadata");
    assert_eq!(
        metadata.id, "openai/whisper-tiny",
        "multilingual model only"
    );
    let dir = std::env::temp_dir().join(format!("medscale-real-ar-{}", std::process::id()));
    assert!(!dir.exists(), "temporary pack path must not already exist");
    let id = build_asr_pack(&metadata, &root, &dir).expect("create governed Audio Pack");
    let manifest = admit_pack_dir(&dir).expect("admit validated Audio Pack");
    assert_eq!(manifest.pack_id.as_str(), id);
    let started = std::time::Instant::now();
    let asr = prepare_whisper_asr(&dir, &manifest).expect("prepare multilingual CPU runtime");
    let prepare_ms = started.elapsed().as_millis();
    let cases: Vec<serde_json::Value> = serde_json::from_slice(
        &std::fs::read(required("MEDSCALE_ASR_CASES")).expect("true FLEURS recordings"),
    )
    .expect("valid evaluation manifest");
    assert_eq!(cases.len(), 24, "bounded real Arabic pilot");
    let (mut total_werr, mut total_wr, mut total_cerr, mut total_cr) = (0, 0, 0, 0);
    let (mut audio_samples, mut transcribe_ms) = (0usize, 0u128);
    for item in &cases {
        let path = Path::new(item["wav"].as_str().expect("audio file"));
        let audio = wav(&std::fs::read(path).expect("genuine public audio"));
        assert!((32_000..=432_000).contains(&audio.len()));
        let start = std::time::Instant::now();
        // Unlike the old transcribe() method, this explicitly emits <|ar|>.
        let output = asr
            .transcribe_with_language(&audio, "ar")
            .expect("genuine multilingual Arabic inference");
        transcribe_ms += start.elapsed().as_millis();
        let reference = unit_words(item["text"].as_str().expect("real transcript"));
        let actual = unit_words(&output);
        assert!(!reference.is_empty(), "reference nonempty");
        total_werr += edits(&reference, &actual);
        total_wr += reference.len();
        let rc: Vec<char> = reference.join("").chars().collect();
        let hc: Vec<char> = actual.join("").chars().collect();
        total_cerr += edits(&rc, &hc);
        total_cr += rc.len();
        audio_samples += audio.len();
        // Never emit original audio, speaker IDs, text or individual cases.
    }
    assert!(total_wr > 50 && total_cr > 200);
    let audio_secs = audio_samples as f64 / 16_000.0;
    println!(
        "ARABIC_ASR_REAL_METRICS {}",
        serde_json::json!({
            "dataset": "google/fleurs",
            "locale": "ar_eg",
            "split": "test",
            "license": "CC-BY-4.0",
            "model": metadata.id,
            "revision": metadata.sha,
            "samples": cases.len(),
            "ref_words": total_wr,
            "word_edits": total_werr,
            "wer": total_werr as f64 / total_wr as f64,
            "ref_chars_no_spaces": total_cr,
            "char_edits": total_cerr,
            "cer_no_spaces": total_cerr as f64 / total_cr as f64,
            "audio_seconds": audio_secs,
            "transcribe_ms": transcribe_ms,
            "realtime_factor": (transcribe_ms as f64 / 1000.0) / audio_secs,
            "prepare_ms": prepare_ms,
            "normalization": "unicode alphanumeric only, Arabic harakat stripped, whitespace-token WER, CER strips spaces",
            "scope": "read public Egyptian Arabic speech; CPU",
        })
    );
    std::fs::remove_dir_all(&dir).expect("clean temporary pack");
}
