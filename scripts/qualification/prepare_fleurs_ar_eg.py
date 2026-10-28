#!/usr/bin/env python3
"""FLEURS ar_eg test holdout -> 16-kHz WAV, ephemeral CPU runner only.

The exact google/fleurs dataset parquet revision + SHA-256 is validated by the
workflow before invoking this helper. Licensed CC BY 4.0. Never log utterances,
speaker IDs, source paths, row IDs, audio bytes or transcription content.
"""
from __future__ import annotations
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import wave

import pyarrow.parquet as pq

ALLOWED_COUNTS = frozenset({24, 100})
MAX_PER_SPEAKER = 4

def rank(i: int) -> str:
    return hashlib.sha256(f"medscale-fleurs-ar-eg-v1:{i}".encode()).hexdigest()

def main(parquet: Path, dest: Path, count: int) -> None:
    if count not in ALLOWED_COUNTS:
        raise ValueError("Only fixed 24/100 FLEURS sample counts are permitted")
    dest.mkdir(parents=True, exist_ok=False)
    reader = pq.ParquetFile(parquet)
    columns = reader.schema_arrow.names
    if "audio" not in columns:
        raise RuntimeError("FLEURS test data does not contain expected audio field")
    candidate_text = "transcription" if "transcription" in columns else "raw_transcription"
    if candidate_text not in columns:
        raise RuntimeError("FLEURS test data does not contain reference transcription")
    speaker_key = "speaker_id" if "speaker_id" in columns else None
    keep = ["audio", candidate_text] + ([speaker_key] if speaker_key else [])
    candidate = []
    position = 0
    for batch in reader.iter_batches(batch_size=32, columns=keep):
        for row in batch.to_pylist():
            text = row.get(candidate_text)
            audio = row.get("audio")
            if isinstance(text, str) and 8 <= len(text) <= 400 and isinstance(audio, dict):
                raw = audio.get("bytes")
                if isinstance(raw, bytes) and 1000 <= len(raw) <= 9_000_000:
                    candidate.append((rank(position), position, raw, text, row.get(speaker_key) if speaker_key else None))
            position += 1
    candidate.sort(key=lambda item: item[0])
    if len(candidate) < count:
        raise RuntimeError(f"Insufficient real audio samples in pinned FLEURS test split: {len(candidate)}")
    cases, counts = [], {}
    for _, _, raw, text, speaker in candidate:
        if speaker is not None and counts.get(str(speaker), 0) >= MAX_PER_SPEAKER:
            continue
        wav = dest / f"case{len(cases):02}.wav"
        result = subprocess.run(
            ["ffmpeg", "-nostdin", "-hide_banner", "-loglevel", "error",
             "-y", "-i", "pipe:0", "-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le", str(wav)],
            input=raw, capture_output=True, check=False,
        )
        if result.returncode:
            wav.unlink(missing_ok=True)
            continue
        with wave.open(str(wav), "rb") as file:
            duration = file.getnframes() / file.getframerate()
            valid = (file.getnchannels(), file.getsampwidth(), file.getframerate()) == (1, 2, 16000)
        if not valid or not 2.0 <= duration <= 27.0:
            wav.unlink(missing_ok=True)
            continue
        cases.append({"wav": str(wav), "text": text})
        if speaker is not None:
            counts[str(speaker)] = counts.get(str(speaker), 0) + 1
        if len(cases) >= count:
            break
    if len(cases) != count:
        raise RuntimeError(f"Only {len(cases)}/{count} suitable actual Arabic recordings")
    (dest / "cases.json").write_text(json.dumps(cases, ensure_ascii=False), encoding="utf-8")
    print(
        f"ARABIC_REAL_DATA_OK clips={len(cases)} language=ar_eg "
        f"speaker_field_present={speaker_key is not None} "
        "dataset=google/fleurs split=test license=CC-BY-4.0 "
        "audio_and_transcripts=runner_private"
    )

if __name__ == "__main__":
    if len(sys.argv) != 4:
        raise SystemExit("usage: prepare_fleurs_ar_eg.py PINNED_PARQUET OUTPUT_DIRECTORY 24|100")
    main(Path(sys.argv[1]), Path(sys.argv[2]), int(sys.argv[3]))
