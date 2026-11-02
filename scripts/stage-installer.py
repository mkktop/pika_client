"""Attach the actual CI checkout tree and checksum to each installer artifact."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

platform = os.environ["INSTALLER_PLATFORM"]
assert platform in ("windows-x64", "macos-universal")
files = list(Path(".").glob(os.environ["INSTALLER_GLOB"]))
assert len(files) == 1, f"Expected one installer, got {files}"
source = files[0]
version = json.loads(Path("package.json").read_text())["version"]
suffix = "_x64-setup.exe" if platform == "windows-x64" else "_universal.dmg"
assert source.name.endswith(f"_{version}{suffix}")
directory = Path("artifacts/installers") / platform
directory.mkdir(parents=True, exist_ok=True)
shutil.copyfile(source, directory / source.name)
manifest = {
    "platform": platform,
    "version": version,
    "filename": source.name,
    "size": source.stat().st_size,
    "sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
    "source_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
    "source_tree": subprocess.check_output(["git", "rev-parse", "HEAD^{tree}"], text=True).strip(),
}
(directory / "build-info.json").write_text(json.dumps(manifest, indent=2) + "\n")
print(json.dumps(manifest))
