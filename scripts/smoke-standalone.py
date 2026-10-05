"""Launch the exact portable release outside the repo with no companion files."""
import argparse
import ctypes
from ctypes import wintypes
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

from windows_binary import verify_exe


def window_for_process(pid):
    user32 = ctypes.WinDLL("user32", use_last_error=True)
    callback_type = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
    user32.EnumWindows.argtypes = [callback_type, wintypes.LPARAM]
    user32.GetWindowThreadProcessId.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.DWORD)]
    user32.GetWindowTextW.argtypes = [wintypes.HWND, wintypes.LPWSTR, ctypes.c_int]
    user32.IsWindowVisible.argtypes = [wintypes.HWND]
    windows = []

    @callback_type
    def collect(hwnd, _):
        owner = wintypes.DWORD()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(owner))
        if owner.value == pid and user32.IsWindowVisible(hwnd):
            title = ctypes.create_unicode_buffer(512)
            user32.GetWindowTextW(hwnd, title, len(title))
            windows.append((hwnd, title.value))
        return True

    user32.EnumWindows(collect, 0)
    return next(((hwnd, title) for hwnd, title in windows if title.startswith("dawwny")), None)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/release/dawwny.exe"))
    args = parser.parse_args()
    if os.name != "nt":
        raise SystemExit("This smoke test requires a Windows desktop")
    root = Path(__file__).resolve().parents[1]
    imports = verify_exe(args.binary, root / "crates/app/assets/licenses.zip")
    print(f"PASS: x64 GUI, embedded notices, {len(imports)} Windows DLL imports, no external runtime")
    with tempfile.TemporaryDirectory(prefix="dawwny standalone ") as temporary:
        temporary = Path(temporary)
        program = temporary / "program"
        program.mkdir()
        exe = program / "dawwny.exe"
        shutil.copyfile(args.binary, exe)
        env = dict(os.environ, LOCALAPPDATA=str(temporary / "User data"))
        env.pop("DAWWNY_SCREENSHOT_PATH", None)
        proc = subprocess.Popen([str(exe)], cwd=program, env=env,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        try:
            deadline = time.monotonic() + 30
            window = None
            while time.monotonic() < deadline and proc.poll() is None:
                window = window_for_process(proc.pid)
                if window:
                    break
                time.sleep(0.2)
            if not window:
                raise RuntimeError(f"Native studio did not open (exit {proc.poll()})")
            time.sleep(1)
            user32 = ctypes.WinDLL("user32", use_last_error=True)
            user32.PostMessageW.argtypes = [wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM]
            if not user32.PostMessageW(window[0], 0x0010, 0, 0):
                raise ctypes.WinError(ctypes.get_last_error())
            stdout, stderr = proc.communicate(timeout=15)
            if proc.returncode:
                raise RuntimeError(stderr.decode("utf-8", errors="replace"))
            assert not stdout, "Normal GUI startup wrote to stdout"
            session = temporary / "User data/dawwny/sessions/untitled.dawwny.json"
            project = json.loads(session.read_text(encoding="utf-8"))
            assert project["revision"] == 0 and len(project["tracks"]) == 6
            assert sorted(p.name for p in program.iterdir()) == ["dawwny.exe"]
            print("PASS: one-file GUI launch, default user-data session, clean shutdown outside repository")
            subprocess.run([sys.executable, str(root / "scripts/smoke-mcp.py"),
                            "--binary", str(exe), "--mode", "studio"], check=True)
            failed = subprocess.run([str(exe), "--mcp", "--unknown"], cwd=program,
                                    capture_output=True, timeout=10)
            assert failed.returncode != 0 and not failed.stdout and b"Unknown option" in failed.stderr
            print("PASS: MCP errors use stderr and fail without opening a GUI")
        finally:
            if proc.poll() is None:
                proc.terminate()
                proc.communicate(timeout=5)


if __name__ == "__main__":
    main()
