#!/usr/bin/env python3
"""12 deterministic, distinct-speaker human recordings from LibriSpeech test-clean.

OpenSLR SLR12 / CC BY 4.0. Do not commit, log or redistribute source media,
reference transcripts or speaker identifiers. Input archive is separately
validated against OpenSLR's published MD5 and exact file length.
"""
from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tarfile
import wave
from pathlib import Path


def key(x: str) -> str:
    return hashlib.sha256(("medscale-real-asr-v1:" + x).encode()).hexdigest()


def main(archive: Path, dest: Path) -> None:
    dest.mkdir(parents=True, exist_ok=False)
    with tarfile.open(archive, "r:gz") as archive_file:
        members = {x.name: x for x in archive_file.getmembers() if x.isfile()}
        text = {}
        for name, member in members.items():
            if not name.endswith(".trans.txt"):
                continue
            handle = archive_file.extractfile(member)
            if handle is None:
                raise RuntimeError("missing transcription content")
            for line in handle.read().decode("utf-8").splitlines():
                uid, sep, transcription = line.partition(" ")
                if sep and uid and transcription.strip():
                    text[uid] = transcription.strip()
        speakers = {}
        for name, member in members.items():
            if name.endswith(".flac"):
                uid = Path(name).stem
                if uid in text:
                    speakers.setdefault(uid.split("-")[0], []).append((uid, member))
        cases = []
        for speaker in sorted(speakers, key=key):
            for uid, member in sorted(speakers[speaker], key=lambda x: key(x[0])):
                transcript = text[uid]
                if not 5 <= len(transcript.split()) <= 45:
                    continue
                handle = archive_file.extractfile(member)
                if handle is None:
                    continue
                wav = dest / f"case{len(cases):02}.wav"
                result = subprocess.run(
                    ["ffmpeg", "-nostdin", "-loglevel", "error", "-y", "-i", "pipe:0",
                     "-ac", "1", "-ar", "16000", "-c:a", "pcm_s16le", str(wav)],
                    input=handle.read(), capture_output=True, check=False,
                )
                if result.returncode:
                    wav.unlink(missing_ok=True)
                    continue
                with wave.open(str(wav), "rb") as audio:
                    seconds = audio.getnframes() / audio.getframerate()
                    valid = (audio.getnchannels(), audio.getsampwidth(), audio.getframerate()) == (1, 2, 16000)
                if not valid or not 2 <= seconds <= 18:
                    wav.unlink(missing_ok=True)
                    continue
                cases.append({"wav": str(wav), "text": transcript})
                break
            if len(cases) == 12:
                break
        if len(cases) != 12:
            raise RuntimeError(f"real test has only {len(cases)} distinct valid speakers")
        (dest / "cases.json").write_text(json.dumps(cases), encoding="utf-8")
        print("REAL_DATA_PREPARED cases=12 unique_speakers=12 license=CC-BY-4.0; no samples logged")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit("usage: prep_real_speech.py ARCHIVE OUTPUT_DIR")
    main(Path(sys.argv[1]), Path(sys.argv[2]))
