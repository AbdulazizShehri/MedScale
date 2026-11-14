#!/usr/bin/env python3
"""Deterministic 100-clip licensed, speaker-balanced LibriSpeech test-clean pilot.

Public source: OpenSLR SLR12 (CC BY 4.0). Fixed verified tarball, fixed
SHA256-based selection seed and bounded per-speaker count. Never log speaker IDs,
transcripts or raw recordings, and never write any of them into the repository.
"""
from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tarfile
import wave
from pathlib import Path

MAX_PER_SPEAKER = 4
SAMPLE_COUNT = 100
MIN_WORDS = 5
MAX_WORDS = 45
MIN_SECONDS = 2.0
MAX_SECONDS = 18.0


def key(value: str) -> str:
    return hashlib.sha256(("medscale-real-asr-v2-100:" + value).encode()).hexdigest()


def select_candidate_files(speakers: dict, transcripts: dict):
    """Deterministic speaker-balanced ordering, before decoding any model output."""
    ordered = {
        person: [
            (uid, member)
            for uid, member in sorted(speakers[person], key=lambda x: key(x[0]))
            if MIN_WORDS <= len(transcripts[uid].split()) <= MAX_WORDS
        ]
        for person in sorted(speakers, key=key)
    }
    # Keep the speaker mix broad rather than filling from a few narrators.
    for round_index in range(MAX_PER_SPEAKER):
        for speaker in ordered:
            candidates = ordered[speaker]
            if len(candidates) > round_index:
                yield speaker, candidates[round_index]


def prepare(archive: Path, dest: Path) -> None:
    dest.mkdir(parents=True, exist_ok=False)
    with tarfile.open(archive, "r:gz") as source:
        members = {member.name: member for member in source.getmembers() if member.isfile()}
        transcripts = {}
        for name, member in members.items():
            if not name.endswith(".trans.txt"):
                continue
            handle = source.extractfile(member)
            if handle is None:
                raise ValueError("missing LibriSpeech transcript file")
            for line in handle.read().decode("utf-8").splitlines():
                uid, sep, text = line.partition(" ")
                if sep and uid and text.strip():
                    transcripts[uid] = text.strip()
        speakers = {}
        for name, member in members.items():
            if name.endswith(".flac"):
                uid = Path(name).stem
                if uid in transcripts:
                    speakers.setdefault(uid.split("-")[0], []).append((uid, member))
        cases, speaker_counts = [], {}
        for speaker, (uid, member) in select_candidate_files(speakers, transcripts):
            handle = source.extractfile(member)
            if handle is None:
                continue
            wav = dest / f"case{len(cases):03d}.wav"
            conversion = subprocess.run(
                ["ffmpeg", "-nostdin", "-loglevel", "error", "-y", "-i", "pipe:0",
                 "-ac", "1", "-ar", "16000", "-c:a", "pcm_s16le", str(wav)],
                input=handle.read(), capture_output=True, check=False,
            )
            if conversion.returncode:
                wav.unlink(missing_ok=True)
                continue
            with wave.open(str(wav), "rb") as audio:
                seconds = audio.getnframes() / audio.getframerate()
                valid = (audio.getnchannels(), audio.getsampwidth(), audio.getframerate()) == (1, 2, 16000)
            if not valid or not MIN_SECONDS <= seconds <= MAX_SECONDS:
                wav.unlink(missing_ok=True)
                continue
            cases.append({"wav": str(wav), "text": transcripts[uid]})
            speaker_counts[speaker] = speaker_counts.get(speaker, 0) + 1
            if len(cases) >= SAMPLE_COUNT:
                break
        if len(cases) != SAMPLE_COUNT:
            raise ValueError(f"Only {len(cases)}/{SAMPLE_COUNT} valid real recordings in bounded sample")
        if any(n > MAX_PER_SPEAKER for n in speaker_counts.values()):
            raise AssertionError("speaker cap violated")
        if len(speaker_counts) < 25:
            raise ValueError("sample has too few independent speakers")
        (dest / "cases.json").write_text(json.dumps(cases), encoding="utf-8")
        # Public CI evidence prints only totals; original IDs and text are private
        # to ephemeral runner scratch. No recordings or weights become artifacts.
        print(
            f"REAL_DATA_PREPARED cases={len(cases)} "
            f"unique_speakers={len(speaker_counts)} max_per_speaker={max(speaker_counts.values())} "
            "split=test-clean license=CC-BY-4.0 no_raw_text_logged"
        )


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit("usage: prepare_librispeech_real_asr.py ARCHIVE OUTPUT_DIR")
    prepare(Path(sys.argv[1]), Path(sys.argv[2]))
