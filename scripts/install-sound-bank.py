#!/usr/bin/env python3
"""Install the optional GeneralUser GS 2.0.3 sample bank outside Git and the executable."""
import argparse
import hashlib
from pathlib import Path
import shutil
import tempfile
import urllib.request

REVISION = "684543d5e5efaef08d02be50dcda8d552478fa60"
BASE = f"https://raw.githubusercontent.com/mrbumpy409/GeneralUser-GS/{REVISION}"
SHA256 = "9575028c7a1f589f5770fccc8cff2734566af40cd26ed836944e9a5152688cfe"
SIZE = 32319396


def digest(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dest", type=Path, default=Path("sample-banks"))
    parser.add_argument("--source", type=Path, help="Use an already downloaded unmodified bank")
    args = parser.parse_args()
    destination = args.dest.resolve()
    destination.mkdir(parents=True, exist_ok=True)
    output = destination / "GeneralUser-GS.sf2"
    if not output.exists() or digest(output) != SHA256:
        temporary = None
        try:
            with tempfile.NamedTemporaryFile(dir=destination, suffix=".part", delete=False) as target:
                temporary = Path(target.name)
                if args.source:
                    with args.source.open("rb") as source:
                        shutil.copyfileobj(source, target, 1024 * 1024)
                else:
                    print("Downloading optional GeneralUser GS 2.0.3 (30.8 MiB)...", flush=True)
                    with urllib.request.urlopen(f"{BASE}/GeneralUser-GS.sf2", timeout=30) as source:
                        count = 0
                        while block := source.read(1024 * 1024):
                            count += len(block)
                            if count > SIZE:
                                raise ValueError("Download exceeds expected bank size")
                            target.write(block)
            if temporary.stat().st_size != SIZE or digest(temporary) != SHA256:
                raise ValueError("Bank checksum does not match the pinned upstream version")
            temporary.replace(output)
        finally:
            if temporary and temporary.exists():
                temporary.unlink()
    with urllib.request.urlopen(f"{BASE}/documentation/LICENSE.txt", timeout=30) as source:
        license_text = source.read(65537)
    if len(license_text) > 65536:
        raise ValueError("Unexpected bank license size")
    (destination / "GeneralUser-GS-LICENSE.txt").write_bytes(license_text)
    print(f"Verified {output}\nSHA256 {SHA256}\nIn Dawwny: Sounds > Sample instruments > Import SoundFont.")


if __name__ == "__main__":
    main()
