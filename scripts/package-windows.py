"""Stage the single portable Windows EXE; no ZIP, installer or companion binary."""
import argparse
import hashlib
from pathlib import Path
import shutil

from windows_binary import verify_exe


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/release/dawwny.exe"))
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    source = args.binary.resolve()
    imports = verify_exe(source, root / "crates/app/assets/licenses.zip")
    directory = root / "artifacts/windows-x64"
    directory.mkdir(parents=True, exist_ok=True)
    destination = directory / "dawwny.exe"
    shutil.copyfile(source, destination)
    digest = hashlib.sha256(destination.read_bytes()).hexdigest()
    print(f"Standalone EXE: {destination}\nSize: {destination.stat().st_size:,} bytes\n"
          f"SHA256: {digest}\nWindows DLLs: {', '.join(imports)}")


if __name__ == "__main__":
    main()
