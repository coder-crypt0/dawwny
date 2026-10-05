"""Regenerate the deterministic notice archive embedded in the standalone EXE."""
import argparse
import io
import zipfile
from pathlib import Path

from windows_licenses import third_party_notices


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Fail if embedded notices are stale")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    entries = third_party_notices(root)
    entries["LICENSE"] = (root / "LICENSE").read_bytes()
    output = io.BytesIO()
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for name, content in sorted(entries.items()):
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, content)
    path = root / "crates/app/assets/licenses.zip"
    if args.check:
        if not path.is_file():
            raise SystemExit("Embedded licenses are missing; run python scripts/embed-licenses.py")
        with zipfile.ZipFile(path) as archive:
            if archive.testzip() is not None:
                raise SystemExit("Embedded license archive is damaged")
            embedded = {name: archive.read(name) for name in archive.namelist()}
        # Deflate bytes can differ between Python/zlib versions; notices must match exactly.
        changed = sorted(name for name in embedded.keys() | entries.keys()
                         if embedded.get(name) != entries.get(name))
        if changed:
            raise SystemExit("Embedded licenses are stale: " + ", ".join(changed[:20])
                             + "; run python scripts/embed-licenses.py")
        print("PASS: embedded license notices match the locked Windows dependencies")
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(output.getvalue())
        print(f"Embedded {len(entries)} notices ({path.stat().st_size:,} bytes)")


if __name__ == "__main__":
    main()
