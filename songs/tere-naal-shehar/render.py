#!/usr/bin/env python3
"""Export the current Dawwny project to MIDI, WAV, and listening MP3s.

This script reads the existing project without regenerating it, so edits made
in the Dawwny studio are preserved. The current mix follows its mute state.
Temporary copies provide full-guide and singing-backing previews.
"""

from __future__ import annotations

import copy
import json
import shutil
import subprocess
from pathlib import Path


SONG = Path(__file__).resolve().parent
REPO = SONG.parents[1]
PROJECT = SONG / "tere-naal-shehar.dawwny.json"
MIDI = SONG / "tere-naal-shehar.mid"
EXPORTS = REPO / "exports"
CURRENT_WAV = EXPORTS / "tere-naal-shehar-current.wav"
GUIDE_WAV = EXPORTS / "tere-naal-shehar-guide.wav"
INSTRUMENTAL_WAV = EXPORTS / "tere-naal-shehar-instrumental.wav"
CURRENT_MP3 = SONG / "tere-naal-shehar-current.mp3"
GUIDE_MP3 = SONG / "tere-naal-shehar-guide.mp3"
INSTRUMENTAL_MP3 = SONG / "tere-naal-shehar-instrumental.mp3"


def run(*args: str) -> None:
    subprocess.run(args, cwd=REPO, check=True)


def render(project: Path, wav: Path, midi: Path) -> None:
    run(
        "cargo", "run", "--release", "-p", "dawwny-audio",
        "--example", "render_project", "--",
        str(project), str(wav), str(midi),
    )


def preview(wav: Path, mp3: Path) -> None:
    run(
        "ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-i", str(wav),
        "-af", "loudnorm=I=-16:TP=-1.0:LRA=8",
        "-ar", "48000", "-codec:a", "libmp3lame", "-q:a", "2", str(mp3),
    )


def render_variant(name: str, data: dict, wav: Path, mp3: Path) -> None:
    temp_project = EXPORTS / f"tere-naal-shehar-{name}.render.json"
    temp_midi = EXPORTS / f"tere-naal-shehar-{name}.render.mid"
    try:
        temp_project.write_text(json.dumps(data), encoding="utf-8")
        render(temp_project, wav, temp_midi)
        preview(wav, mp3)
    finally:
        temp_project.unlink(missing_ok=True)
        temp_midi.unlink(missing_ok=True)


def main() -> None:
    if shutil.which("cargo") is None or shutil.which("ffmpeg") is None:
        raise SystemExit("cargo and ffmpeg must be on PATH")
    EXPORTS.mkdir(exist_ok=True)
    render(PROJECT, CURRENT_WAV, MIDI)
    preview(CURRENT_WAV, CURRENT_MP3)

    guide = copy.deepcopy(json.loads(PROJECT.read_text(encoding="utf-8")))
    for track in guide["tracks"]:
        if track["id"] in {"t-guitar", "t-vocal", "t-hook"}:
            track["mute"] = False
    render_variant("guide", guide, GUIDE_WAV, GUIDE_MP3)

    instrumental = copy.deepcopy(guide)
    vocal = next(t for t in instrumental["tracks"] if t["id"] == "t-vocal")
    vocal["mute"] = True
    render_variant("instrumental", instrumental, INSTRUMENTAL_WAV, INSTRUMENTAL_MP3)

    for path in (PROJECT, MIDI, CURRENT_WAV, GUIDE_WAV, INSTRUMENTAL_WAV,
                 CURRENT_MP3, GUIDE_MP3, INSTRUMENTAL_MP3):
        print(f"{path} ({path.stat().st_size:,} bytes)")


if __name__ == "__main__":
    main()
