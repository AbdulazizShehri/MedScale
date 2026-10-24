#!/usr/bin/env python3
"""Prepare a single source-verified real LibriSpeech test-clean human recording.

LibriSpeech OpenSLR SLR12 is CC BY 4.0. The full archive SHA/MD5 is checked
by the calling workflow. Never log transcript, speaker or utterance IDs.
"""
from __future__ import annotations

import hashlib
import subprocess
import sys
import tarfile
import wave
from pathlib import Path


def key(utterance: str) -> str:
    return hashlib.sha256(("medscale-native-audio-v1:" + utterance).encode()).hexdigest()


def prepare(archive: Path, destination: Path) -> None:
    if destination.exists():
        raise RuntimeError("refusing overwrite")
    with tarfile.open(archive, "r:gz") as source:
        files = {item.name: item for item in source.getmembers() if item.isfile()}
        eligible = set()
        for name, item in files.items():
            if not name.endswith(".trans.txt"):
                continue
            text_file = source.extractfile(item)
            if text_file is None:
                raise RuntimeError("missing reference")
            for line in text_file.read().decode("utf-8").splitlines():
                uid, sep, words = line.partition(" ")
                if sep and 6 <= len(words.split()) <= 22:
                    eligible.add(uid)
        candidates = sorted(
            (
                (key(Path(name).stem), item)
                for name, item in files.items()
                if name.endswith(".flac") and Path(name).stem in eligible
            ),
            key=lambda row: row[0],
        )
        for _, item in candidates[:80]:
            stream = source.extractfile(item)
            if stream is None:
                continue
            completed = subprocess.run(
                ["ffmpeg", "-nostdin", "-hide_banner", "-loglevel", "error", "-y",
                 "-i", "pipe:0", "-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le",
                 str(destination)],
                input=stream.read(), capture_output=True, check=False,
            )
            if completed.returncode != 0:
                destination.unlink(missing_ok=True)
                continue
            with wave.open(str(destination), "rb") as result:
                duration = result.getnframes() / result.getframerate()
                valid = (
                    result.getnchannels(), result.getsampwidth(), result.getframerate()
                ) == (1, 2, 16_000)
            if valid and 4 <= duration <= 13:
                print("REAL_VOICE_WAV_PREPARED clips=1 origin=LibriSpeech-test-clean license=CC-BY-4.0")
                return
            destination.unlink(missing_ok=True)
    raise RuntimeError("no suitable real human speech clip within bounded selection")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit("usage: prepare_librispeech_one_wav.py VERIFIED_ARCHIVE WAV_OUTPUT")
    prepare(Path(sys.argv[1]), Path(sys.argv[2]))
