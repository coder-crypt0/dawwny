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
        if not path.is_file() or path.read_bytes() != output.getvalue():
            raise SystemExit("Embedded licenses are stale; run python scripts/embed-licenses.py")
        print("PASS: embedded license notices match the locked Windows dependencies")
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(output.getvalue())
        print(f"Embedded {len(entries)} notices ({path.stat().st_size:,} bytes)")


if __name__ == "__main__":
    main()
