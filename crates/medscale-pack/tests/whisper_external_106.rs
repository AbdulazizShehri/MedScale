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
#[ignore = "needs OpenAI's mel filters as raw f32 (MEDSCALE_MEL_FILTERS_DIR)"]
fn mel_filters_match_openai_reference() {
    // asr-qualification.yml converts OpenAI's mel_filters.npz (deflate) to
    // raw little-endian f32 files mel_80.f32 and mel_128.f32 with numpy.
    let dir = std::path::PathBuf::from(env("MEDSCALE_MEL_FILTERS_DIR"));
    for (name, n_mels) in [("mel_80", 80), ("mel_128", 128)] {
        let bytes = std::fs::read(dir.join(format!("{name}.f32"))).unwrap();
        let expected: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect();
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
    let real_data = std::env::var("MEDSCALE_ASR_REAL_DATASET").ok();
    if real_data.is_some() {
        let expected: usize = std::env::var("MEDSCALE_ASR_EXPECTED_CASES")
            .unwrap_or_else(|_| "12".into())
            .parse()
            .expect("expected positive sample count");
        assert!((12..=100).contains(&expected), "bounded real corpus count");
        assert_eq!(cases.len(), expected, "bounded real test cases");
    }
    let mut total_ref_words = 0usize;
    let mut total_errors = 0f64;
    let mut char_edits = 0usize;
    let mut ref_chars = 0usize;
    let mut total_audio_samples = 0usize;
    let mut total_transcribe_ms = 0u128;
    for (case_no, case) in cases.iter().enumerate() {
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
        if real_data.is_some() {
            // Character error rate on lowercased alphanumeric content, without
            // spaces. No transcript or hypothesis text leaves the CI runner.
            let rc: Vec<char> = words(reference).join("").chars().collect();
            let hc: Vec<char> = words(&hypothesis).join("").chars().collect();
            ref_chars += rc.len();
            char_edits += {
                let mut prev: Vec<usize> = (0..=hc.len()).collect();
                for (i, rch) in rc.iter().enumerate() {
                    let mut cur = vec![i + 1; hc.len() + 1];
                    for (j, hch) in hc.iter().enumerate() {
                        cur[j + 1] = (prev[j] + usize::from(rch != hch))
                            .min(prev[j + 1] + 1).min(cur[j] + 1);
                    }
                    prev = cur;
                }
                prev[hc.len()]
            };
            total_audio_samples += samples.len();
            total_transcribe_ms += ms;
        }
        if real_data.is_some() {
            // Public evaluation logs MUST NOT contain human voice transcripts.
            println!(
                "ASR_REAL_CASE {}",
                serde_json::json!({
                    "index": case_no, "wer": rate,
                    "audio_seconds": samples.len() as f64 / 16_000.0,
                    "transcribe_ms": ms,
                })
            );
        } else {
            println!(
                "ASR_CASE {}",
                serde_json::json!({
                    "repo": meta.id, "revision": meta.sha, "reference": reference,
                    "hypothesis": hypothesis, "wer": rate,
                    "audio_seconds": samples.len() as f64 / 16_000.0, "transcribe_ms": ms,
                })
            );
        }
    }
    let corpus_wer = total_errors / total_ref_words.max(1) as f64;
    println!(
        "ASR_METRICS {}",
        serde_json::json!({
            "repo": meta.id, "revision": meta.sha, "pack_id": pack_id,
            "cases": cases.len(), "corpus_wer": corpus_wer, "prepare_ms": prepare_ms,
            "speech": real_data.as_deref().unwrap_or("espeak-ng synthetic"),
            "claim": "evaluation evidence only; public read speech is not clinical accuracy",
        })
    );
    if let Some(source) = &real_data {
        assert!(ref_chars > 0 && total_audio_samples > 0, "nonzero real reference/audio");
        let audio_secs = total_audio_samples as f64 / 16_000.0;
        println!(
            "ASR_REAL_METRICS {}",
            serde_json::json!({
                "source": source,
                "repo": meta.id,
                "model_revision": meta.sha,
                "cases": cases.len(),
                "ref_words": total_ref_words,
                "word_errors": total_errors.round() as usize,
                "corpus_wer": corpus_wer,
                "ref_chars_no_spaces": ref_chars,
                "char_errors": char_edits,
                "cer_no_spaces": char_edits as f64 / ref_chars as f64,
                "audio_seconds": audio_secs,
                "transcribe_ms": total_transcribe_ms,
                "realtime_factor": (total_transcribe_ms as f64 / 1000.0) / audio_secs,
                "prepare_ms": prepare_ms,
                "normalization": "lowercase; punctuation removed; number zero-twenty mapped to digits; CER strips spaces",
                "scope": "English read speech on a standard GitHub CPU runner",
            })
        );
    }
    // A working recognizer gets most synthetic words right; a broken front
    // end or decoder does not.
    if real_data.is_none() {
        // Preserve the original synthetic regression threshold exactly.
        assert!(corpus_wer < 0.5, "corpus WER {corpus_wer}");
    }
    // Real human speech is measured, not tuned to an invented pass threshold.
    let _ = std::fs::remove_dir_all(&out);
}
