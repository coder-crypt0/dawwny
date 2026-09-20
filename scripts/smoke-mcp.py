"""Exercise a real stdio MCP process against a temporary local session."""
import argparse
import json
from pathlib import Path
import queue
import subprocess
import tempfile
import threading


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/release/dawwny-mcp.exe"))
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="dawwny-smoke-") as root:
        root = Path(root)
        proc = subprocess.Popen(
            [str(args.binary.resolve()), "--project", str(root / "session.json"),
             "--export-dir", str(root / "exports")],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, encoding="utf-8",
        )
        lines = queue.Queue()

        def read():
            for line in proc.stdout:
                lines.put(line)

        threading.Thread(target=read, daemon=True).start()
        request_id = 0

        def send(method, params=None, notification=False):
            nonlocal request_id
            request_id += 1
            message = {"jsonrpc": "2.0", "method": method, "params": params or {}}
            if not notification:
                message["id"] = request_id
            proc.stdin.write(json.dumps(message) + "\n")
            proc.stdin.flush()
            if notification:
                return None
            while True:
                response = json.loads(lines.get(timeout=30))
                if response.get("id") == request_id:
                    if "error" in response:
                        raise RuntimeError(response["error"])
                    return response["result"]

        try:
            send("initialize", {"protocolVersion": "2025-11-25", "capabilities": {},
                                "clientInfo": {"name": "dawwny-smoke", "version": "0.1.0"}})
            send("notifications/initialized", notification=True)
            tools = send("tools/list")["tools"]
            assert {t["name"] for t in tools} == {"read_project", "apply_commands", "export_midi", "render_wav", "list_sounds", "get_sound"}
            sounds = send("tools/call", {"name": "list_sounds", "arguments": {}})
            assert json.loads(sounds["content"][0]["text"])["total"] == 2310
            project = send("tools/call", {"name": "read_project", "arguments": {}})
            project = json.loads(project["content"][0]["text"])
            assert project["revision"] == 0 and len(project["tracks"]) == 6
            edit = {"name": "apply_commands", "arguments": {"expected_revision": 0,
                    "commands": [{"type": "set_tempo", "tempo": 96}, {"type": "apply_sound_preset", "track_id": project["tracks"][0]["id"], "preset_id": "amber_keys"}]}}
            changed = send("tools/call", edit)
            assert not changed.get("isError", False)
            assert json.loads(changed["content"][0]["text"])["revision"] == 1
            assert send("tools/call", edit)["isError"]
            for tool in ("export_midi", "render_wav"):
                result = send("tools/call", {"name": tool, "arguments": {"expected_revision": 1}})
                assert not result.get("isError", False), result
                result = json.loads(result["content"][0]["text"])
                artifact = Path(result["path"])
                assert artifact.is_file() and artifact.stat().st_size > 100
                assert artifact.parent == root / "exports"
                if tool == "render_wav":
                    assert result["peak"] > 0.01 and result["stolen_voices"] == 0
            print("PASS: stdio handshake, tool discovery, project edit, conflict, MIDI, WAV")
        finally:
            proc.stdin.close()
            try:
                proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                proc.terminate()
                proc.wait(timeout=5)
            if proc.returncode:
                raise RuntimeError(proc.stderr.read())


if __name__ == "__main__":
    main()
