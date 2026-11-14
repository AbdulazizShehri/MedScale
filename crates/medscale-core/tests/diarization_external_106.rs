//! Spec 106 qualification: acoustic-clustering diarization on a synthetic
//! two-voice dialogue (espeak-ng male and female voices, generated on the
//! runner by asr-qualification.yml). Ignored locally. Evaluation evidence on
//! synthetic speech only; speaker labels are never identity.

use medscale_contracts::audio::VadParameters;
use medscale_core::authority::{analyze_pcm, diarize, parse_wav};

#[test]
#[ignore = "needs MEDSCALE_DIAR_WAV and MEDSCALE_DIAR_TURNS from asr-qualification.yml"]
fn acoustic_clustering_separates_two_synthetic_voices() {
    let wav = std::fs::read(std::env::var("MEDSCALE_DIAR_WAV").expect("wav")).unwrap();
    // Turns: [[start_ms, end_ms, voice], ...] for the generated dialogue.
    let turns: Vec<(u64, u64, u16)> = serde_json::from_slice(
        &std::fs::read(std::env::var("MEDSCALE_DIAR_TURNS").expect("turns")).unwrap(),
    )
    .unwrap();
    let (format, pcm) = parse_wav(&wav).expect("valid WAV");
    let spans = analyze_pcm(format, pcm, VadParameters::FROZEN).segments;
    let labels = diarize(format, pcm, &spans);
    // Each VAD segment belongs to the turn it overlaps most.
    let mut votes = std::collections::BTreeMap::<(u16, u16), u64>::new();
    for (&(s, e), &label) in spans.iter().zip(&labels) {
        let best = turns
            .iter()
            .map(|&(ts, te, voice)| (e.min(te).saturating_sub(s.max(ts)), voice))
            .max_by_key(|&(overlap, _)| overlap);
        if let Some((overlap, voice)) = best
            && overlap > 0
        {
            *votes.entry((voice, label)).or_default() += e - s;
        }
    }
    // Each predicted label counts toward its majority voice (time-weighted).
    let total: u64 = votes.values().sum();
    let mut labels_seen: Vec<u16> = votes.keys().map(|&(_, l)| l).collect();
    labels_seen.sort_unstable();
    labels_seen.dedup();
    let agree: u64 = labels_seen
        .iter()
        .map(|&l| {
            votes
                .iter()
                .filter(|((_, ll), _)| *ll == l)
                .map(|(_, &t)| t)
                .max()
                .unwrap_or(0)
        })
        .sum();
    let accuracy = agree as f64 / total.max(1) as f64;
    let speakers = labels.iter().copied().max().unwrap_or(0);
    println!(
        "DIARIZATION_METRICS {}",
        serde_json::json!({
            "segments": spans.len(), "turns": turns.len(), "speakers_found": speakers,
            "time_weighted_accuracy": accuracy, "method": "acoustic_clustering",
            "speech": "espeak-ng synthetic", "claim": "evaluation evidence only",
        })
    );
    let expected: u16 = std::env::var("MEDSCALE_DIAR_EXPECT_SPEAKERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2);
    assert_eq!(speakers, expected, "voices: {labels:?}");
    assert!(accuracy >= 0.9, "accuracy {accuracy}");
}
