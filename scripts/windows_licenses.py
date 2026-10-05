"""Collect notices from locked normal Windows dependencies and pinned supplements."""
import json
import re
import subprocess
from pathlib import Path


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

