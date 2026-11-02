"""Check the distributed DMG, both CPU slices, signature integrity and launch."""
from pathlib import Path
import json
import plistlib
import subprocess
import tempfile

bundle_dir = Path("src-tauri/target/universal-apple-darwin/release/bundle")
images = list((bundle_dir / "dmg").glob("*.dmg"))
assert len(images) == 1, images
subprocess.run(["hdiutil", "verify", str(images[0])], check=True)

with tempfile.TemporaryDirectory(prefix="pika-dmg-") as mount:
    subprocess.run(["hdiutil", "attach", "-nobrowse", "-readonly", "-mountpoint", mount, str(images[0])], check=True)
    try:
        app = Path(mount) / "Pika Client.app"
        with (app / "Contents/Info.plist").open("rb") as stream:
            info = plistlib.load(stream)
        assert info["CFBundleIdentifier"] == "com.pika.client"
        assert info["CFBundleShortVersionString"] == json.loads(Path("package.json").read_text())["version"]
        assert info["LSMinimumSystemVersion"].startswith("11.")
        executable = app / "Contents/MacOS" / info["CFBundleExecutable"]
        assert executable.is_file()
        assert (Path(mount) / "Applications").is_symlink()
        subprocess.run(["lipo", str(executable), "-verify_arch", "arm64", "x86_64"], check=True)
        subprocess.run(["codesign", "--verify", "--deep", "--strict", str(app)], check=True)
        process = subprocess.Popen([str(executable)], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        try:
            output, _ = process.communicate(timeout=10)
            raise RuntimeError(f"Application exited during launch ({process.returncode}): {output}")
        except subprocess.TimeoutExpired:
            print("Universal application stayed running during the 10-second native launch check")
        finally:
            process.terminate()
            try:
                process.communicate(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.communicate()
    finally:
        subprocess.run(["hdiutil", "detach", mount], check=True)
