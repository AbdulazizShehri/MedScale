//! Spec 106 qualification: a pinned Whisper checkpoint becomes a signed Audio
//! Pack and transcribes synthetic speech (espeak-ng, generated on the runner)
//! on the CPU. Ignored: it needs the files `asr-qualification.yml` downloads.
//! The word error rate is evaluation evidence on synthetic speech, never a
//! clinical or medical-accuracy claim.

use std::path::Path;

use medscale_pack::{
    HfRepoMetadata, admit_pack_dir, build_asr_pack, mel_filters, pcm16_to_mono_f32,
    prepare_whisper_asr,
};

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set"))
}

/// Lowercased words with small number words written as digits, so "two"
/// and "2" compare equal (Whisper writes either).
fn words(text: &str) -> Vec<String> {
    const NUMBERS: &[&str] = &[
        "zero",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
        "twenty",
    ];
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '\'')
        .filter(|w| !w.is_empty())
        .map(|w| {
            NUMBERS
                .iter()
                .position(|n| *n == w)
                .map_or_else(|| w.to_owned(), |i| i.to_string())
        })
        .collect()
}

/// Word error rate: word-level Levenshtein distance over reference length.
fn wer(reference: &str, hypothesis: &str) -> f64 {
    let (r, h) = (words(reference), words(hypothesis));
    let mut prev: Vec<usize> = (0..=h.len()).collect();
    for (i, rw) in r.iter().enumerate() {
        let mut cur = vec![i + 1; h.len() + 1];
        for (j, hw) in h.iter().enumerate() {
            let sub = prev[j] + usize::from(rw != hw);
            cur[j + 1] = sub.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[h.len()] as f64 / r.len().max(1) as f64
}

/// 16-bit PCM samples from a canonical WAV (`fmt ` then `data`).
fn wav_samples(bytes: &[u8]) -> Vec<f32> {
    assert_eq!(&bytes[..4], b"RIFF");
    let mut i = 12;
    let (mut channels, mut rate, mut bits) = (0u16, 0u32, 0u16);
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let len = u32::from_le_bytes(bytes[i + 4..i + 8].try_into().unwrap()) as usize;
        let body = &bytes[i + 8..(i + 8 + len).min(bytes.len())];
        if id == b"fmt " {
            channels = u16::from_le_bytes([body[2], body[3]]);
            rate = u32::from_le_bytes(body[4..8].try_into().unwrap());
            bits = u16::from_le_bytes([body[14], body[15]]);
        } else if id == b"data" {
            assert_eq!((rate, bits), (16_000, 16), "expect 16 kHz 16-bit PCM");
            return pcm16_to_mono_f32(body, channels).expect("PCM frames");
        }
        i += 8 + len + (len & 1);
    }
    panic!("no data chunk");
}

#[test]
fn wer_counts_word_edits() {
    assert_eq!(
        wer("the patient has a fever", "the patient has a fever."),
        0.0
    );
    assert!((wer("a b c d", "a x c") - 0.5).abs() < 1e-9);
    assert_eq!(wer("type two diabetes", "Type 2 diabetes."), 0.0);
}

#[test]
#[ignore = "needs OpenAI's mel_filters.npz (MEDSCALE_MEL_FILTERS_NPZ)"]
fn mel_filters_match_openai_reference() {
    let reference = candle_core::Tensor::read_npz(env("MEDSCALE_MEL_FILTERS_NPZ")).unwrap();
    for (name, n_mels) in [("mel_80", 80), ("mel_128", 128)] {
        let names: Vec<&str> = reference.iter().map(|(n, _)| n.as_str()).collect();
        let tensor = &reference
            .iter()
            .find(|(n, _)| n.trim_end_matches(".npy") == name)
            .unwrap_or_else(|| panic!("{name} not in {names:?}"))
            .1;
        let expected: Vec<f32> = tensor.flatten_all().unwrap().to_vec1().unwrap();
        let ours = mel_filters(n_mels);
        assert_eq!(expected.len(), ours.len(), "{name}");
        let max_diff = expected
            .iter()
            .zip(&ours)
            .map(|(a, b)| (a - b).abs())
            .fold(0f32, f32::max);
        println!("MEL_FILTERS {name} max_abs_diff={max_diff:e}");
        assert!(max_diff < 1e-5, "{name}: {max_diff}");
    }
}

#[test]
#[ignore = "needs a pinned Whisper snapshot and synthetic speech from asr-qualification.yml"]
fn pinned_whisper_transcribes_synthetic_speech() {
    let snapshot = std::path::PathBuf::from(env("MEDSCALE_ASR_SNAPSHOT_DIR"));
    let meta: HfRepoMetadata =
        serde_json::from_slice(&std::fs::read(snapshot.join("metadata.json")).unwrap()).unwrap();
    let out = std::env::temp_dir().join(format!("medscale-106-pack-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let pack_id = build_asr_pack(&meta, &snapshot, &out).expect("pinned snapshot builds a Pack");
    let manifest = admit_pack_dir(&out).expect("signed Audio Pack admits");
    assert_eq!(manifest.pack_id.as_str(), pack_id);
    let started = std::time::Instant::now();
    let asr = prepare_whisper_asr(&out, &manifest).expect("Whisper prepares");
    let prepare_ms = started.elapsed().as_millis();

    let cases: Vec<serde_json::Value> =
        serde_json::from_slice(&std::fs::read(env("MEDSCALE_ASR_CASES")).unwrap()).unwrap();
    let mut total_ref_words = 0usize;
    let mut total_errors = 0f64;
    for case in &cases {
        let wav = Path::new(case["wav"].as_str().unwrap());
        let reference = case["text"].as_str().unwrap();
        let samples = wav_samples(&std::fs::read(wav).unwrap());
        let started = std::time::Instant::now();
        let hypothesis = asr.transcribe(&samples).expect("transcribes");
        let ms = started.elapsed().as_millis();
        let rate = wer(reference, &hypothesis);
        let n = words(reference).len();
        total_ref_words += n;
        total_errors += rate * n as f64;
        println!(
            "ASR_CASE {}",
            serde_json::json!({
                "repo": meta.id, "revision": meta.sha, "reference": reference,
                "hypothesis": hypothesis, "wer": rate,
                "audio_seconds": samples.len() as f64 / 16_000.0, "transcribe_ms": ms,
            })
        );
    }
    let corpus_wer = total_errors / total_ref_words.max(1) as f64;
    println!(
        "ASR_METRICS {}",
        serde_json::json!({
            "repo": meta.id, "revision": meta.sha, "pack_id": pack_id,
            "cases": cases.len(), "corpus_wer": corpus_wer, "prepare_ms": prepare_ms,
            "speech": "espeak-ng synthetic", "claim": "evaluation evidence only",
        })
    );
    // A working recognizer gets most synthetic words right; a broken front
    // end or decoder does not.
    assert!(corpus_wer < 0.5, "corpus WER {corpus_wer}");
    let _ = std::fs::remove_dir_all(&out);
}
