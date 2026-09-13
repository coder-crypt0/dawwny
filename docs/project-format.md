# Dawwny project format

Dawwny projects are UTF-8 JSON documents. The root object is a `Project` with schema version, stable id, revision, tempo, bar length, master gain, tracks, and sections. Tracks contain clips; clips contain notes whose timing is measured in quarter-note beats.

Projects are validated at every load and write. A project may contain at most 32 tracks, 128 clips per track, 32,768 notes total, and 256 bars. Tempo is 20–300 BPM. IDs must be unique, velocities are 0–1, pans are -1–1, and note/clip timelines must remain within the project length.

Commands are tagged JSON objects, for example `{ "type": "set_tempo", "tempo": 92.0 }`. Session revisions increase monotonically; stale edits are rejected.
