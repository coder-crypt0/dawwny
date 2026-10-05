#!/usr/bin/env python3
"""Package the native Dawwny Windows release without requiring third-party Python packages."""
import argparse
import hashlib
import json
import re
import subprocess
import zipfile
from pathlib import Path

VERSION_RE = re.compile(r"^[0-9A-Za-z][0-9A-Za-z._+-]{0,63}$")


def third_party_notices(root: Path) -> dict[str, bytes]:
    def cargo(*args):
        return subprocess.check_output(["cargo", *args], cwd=root, text=True, encoding="utf-8")
    data = json.loads(cargo("metadata", "--format-version", "1", "--locked", "--filter-platform", "x86_64-pc-windows-msvc"))
    tree = cargo("tree", "--workspace", "--target", "x86_64-pc-windows-msvc", "--edges", "normal", "--prefix", "none", "--format", "{p}", "--locked")
    selected = set(re.findall(r"(?m)^([A-Za-z0-9_-]+) v([^ \n]+)", tree))
    own = set(data["workspace_members"])
    packages = sorted((p for p in data["packages"] if p["id"] not in own and (p["name"], p["version"]) in selected), key=lambda p: (p["name"], p["version"]))
    lines = ["Third-party dependencies in the normal Windows dependency graph.", "License identifiers below are declared by their authors. Full bundled notices follow under licenses/.", ""]
    entries = {}
    for package in packages:
        name = f"{package['name']}-{package['version']}"
        lines.append(f"{name} | {package.get('license') or 'See included license file'} | {package.get('repository') or package.get('source', '')}")
        directory = Path(package["manifest_path"]).parent
        files = set()
        if package.get("license_file"):
            files.add(directory / package["license_file"])
        for path in directory.rglob("*"):
            lower = path.name.lower()
            if path.is_file() and (("license" in lower or "licence" in lower or lower.startswith(("copying", "copyright", "notice", "ofl", "ufl")) or ("fonts" in path.parts and path.suffix == ".txt")) or any(part.lower() in ("licenses", "licences") for part in path.relative_to(directory).parts[:-1])):
                files.add(path)
        for path in sorted(files):
            if not path.is_file():
                continue
            relative = path.relative_to(directory).as_posix()
            entries[f"licenses/{name}/{relative}"] = path.read_bytes()
        # Some published crates omit license files that remain in their upstream repository.
        supplemental = root / "third-party" / name
        if supplemental.is_dir():
            for path in sorted(supplemental.rglob("*")):
                if path.is_file():
                    relative = path.relative_to(supplemental).as_posix()
                    entries[f"licenses/{name}/{relative}"] = path.read_bytes()
    entries["THIRD-PARTY-NOTICES.txt"] = ("\n".join(lines) + "\n").encode("utf-8")
    return entries


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", default="0.1.0")
    args = parser.parse_args()
    if not VERSION_RE.fullmatch(args.version) or ".." in args.version:
        parser.error("version must be a safe single filename component")
    root = Path(__file__).resolve().parents[1]
    required = [root / "target" / "release" / "dawwny.exe", root / "target" / "release" / "dawwny-mcp.exe"]
    missing = [str(p) for p in required if not p.is_file()]
    if missing:
        raise SystemExit("missing release executable(s): " + ", ".join(missing))
    out_dir = root / "artifacts"
    out_dir.mkdir(exist_ok=True)
    archive = out_dir / f"dawwny-{args.version}-windows-x64.zip"
    top = f"dawwny-{args.version}"
    files = [(required[0], "dawwny.exe"), (required[1], "dawwny-mcp.exe")]
    for rel in ("README.md", "LICENSE", "scripts/install-sound-bank.py"):
        path = root / rel
        if path.is_file(): files.append((path, rel))
    for path in sorted((root / "docs").glob("*.md")):
        files.append((path, f"docs/{path.name}"))
    for path in sorted((root / "docs" / "images").glob("*.png")):
        files.append((path, f"docs/images/{path.name}"))
    for demo in sorted((root / "examples").glob("*.dawwny.json")):
        files.append((demo, f"examples/{demo.name}"))
    notices = third_party_notices(root)
    with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as zf:
        entries = [(p.read_bytes(), f"{top}/{name}") for p, name in files] + [(content, f"{top}/{name}") for name, content in sorted(notices.items())]
        for content, name in entries:
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0)); info.compress_type = zipfile.ZIP_DEFLATED
            zf.writestr(info, content)
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    (archive.with_suffix(archive.suffix + ".sha256")).write_text(f"{digest}  {archive.name}\n", encoding="ascii")
    print(f"created {archive}\nsha256 {digest}")


if __name__ == "__main__": main()
