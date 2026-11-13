"""One native regression flow, two controlled source-window modes."""
import json
from pathlib import Path
import plistlib
import select
import shutil
import subprocess
import tempfile
import time

binary = Path("dist/Kigi Windows.app/Contents/MacOS/kigi-windows").resolve()
fixture_source = Path("scripts/focus-fixture.swift").resolve()

def ready(process):
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError("Fixture exited")
        readable, _, _ = select.select([process.stdout], [], [], 0.2)
        if readable:
            line = process.stdout.readline().decode(errors="replace").strip()
            if line.startswith("FIXTURE_READY "):
                assert line.endswith(" true"), line
                return
    raise RuntimeError("Fixture not ready")

def stop(process):
    if process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=4)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()

with tempfile.TemporaryDirectory(prefix="kigi-focus-") as directory:
    root = Path(directory)
    executable = root / "fixture"
    subprocess.run(["swiftc", str(fixture_source), "-o", str(executable)], check=True)
    paths = {}
    for name in ["source", "target"]:
        contents = root / (name + ".app") / "Contents"
        (contents / "MacOS").mkdir(parents=True)
        shutil.copy2(executable, contents / "MacOS" / "fixture")
        with (contents / "Info.plist").open("wb") as file:
            plistlib.dump({"CFBundleExecutable": "fixture", "CFBundleIdentifier": "io.kigi.focus." + name, "CFBundleName": "Kigi focus " + name, "CFBundlePackageType": "APPL"}, file)
        subprocess.run(["codesign", "--force", "--sign", "-", str(contents.parent)], check=True)
        paths[name] = contents / "MacOS" / "fixture"
    target = subprocess.Popen([str(paths["target"])], stdout=subprocess.PIPE)
    try:
        ready(target)
        for fullscreen in [False, True]:
            source = subprocess.Popen([str(paths["source"]), "--source"] + (["--fullscreen"] if fullscreen else []), stdout=subprocess.PIPE)
            try:
                ready(source)
                check = subprocess.run([str(binary), "--activation-check"] + (["--check-maximize"] if fullscreen else []), capture_output=True, text=True, timeout=15)
                rows = [json.loads(line.removeprefix("KIGI_FOCUS_CHECK ")) for line in check.stdout.splitlines() if line.startswith("KIGI_FOCUS_CHECK ")]
                assert check.returncode == 0 and len(rows) == 1 and rows[0]["success"], check.stdout + check.stderr
                assert rows[0]["frontmost_pid"] == target.pid, rows
                deadline = time.monotonic() + 2
                visible = False
                while time.monotonic() < deadline:
                    owners = json.loads(subprocess.check_output([str(executable), "--window-order"]))["owners"]
                    visible = target.pid in owners and (source.pid not in owners or owners.index(target.pid) < owners.index(source.pid))
                    if visible:
                        break
                    time.sleep(0.1)
                assert visible, {"target": target.pid, "source": source.pid, "owners": owners}
                print("FOCUS_VERIFIED " + json.dumps({"source_native_fullscreen": fullscreen, "target_maximized": fullscreen, "target_active": True, "target_window_in_front": visible}))
            finally:
                stop(source)
                time.sleep(0.4)
    finally:
        stop(target)
