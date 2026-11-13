"""Real Studio apply/restore with two owned AppKit windows; normal and min-size cases."""
import json
import os
from pathlib import Path
import plistlib
import select
import shutil
import subprocess
import tempfile
import time

binary = Path("dist/Kigi Windows.app/Contents/MacOS/kigi-windows").resolve()

def ready(process):
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError("Fixture exited")
        readable, _, _ = select.select([process.stdout], [], [], .2)
        if readable:
            line = process.stdout.readline().decode(errors="replace")
            if line.startswith("FIXTURE_READY "):
                assert line.strip().endswith(" true"), line
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

with tempfile.TemporaryDirectory(prefix="kigi-layout-") as directory:
    root = Path(directory)
    fixture = root / "fixture"
    subprocess.run(["swiftc", "scripts/focus-fixture.swift", "-o", str(fixture)], check=True)
    paths = {}
    for name in ["source", "target"]:
        contents = root / (name + ".app") / "Contents"
        (contents / "MacOS").mkdir(parents=True)
        shutil.copy2(fixture, contents / "MacOS/fixture")
        (contents / "Info.plist").write_bytes(plistlib.dumps({"CFBundleExecutable": "fixture", "CFBundleIdentifier": "io.kigi.focus." + name, "CFBundleName": "Kigi layout " + name, "CFBundlePackageType": "APPL"}))
        subprocess.run(["codesign", "--force", "--sign", "-", str(contents.parent)], check=True)
        paths[name] = contents / "MacOS/fixture"
    for minimum in [False, True]:
        processes = []
        try:
            target = subprocess.Popen([str(paths["target"])], stdout=subprocess.PIPE, bufsize=0); processes.append(target); ready(target)
            source = subprocess.Popen([str(paths["source"]), "--source"] + (["--minimum"] if minimum else []), stdout=subprocess.PIPE, bufsize=0); processes.append(source); ready(source)
            def frames():
                rows = json.loads(subprocess.check_output([str(fixture), "--window-frames", str(source.pid), str(target.pid)]))
                values = {row["pid"]: row["frame"] for row in rows}
                assert source.pid in values and target.pid in values, "Missing fixture frame"
                return values
            before = frames()
            path = root / "config.json"
            left = .35 if minimum else .5
            path.write_text(json.dumps({"schema_version": 2, "next_id": 2, "profiles": [{"id": 1, "name": "Taille minimale" if minimum else "Placement35", "windows": [
                {"bundle_id": "io.kigi.focus.source", "geometry": {"x": 0, "y": 0, "width": left, "height": 1}},
                {"bundle_id": "io.kigi.focus.target", "geometry": {"x": left, "y": 0, "width": 1-left, "height": 1}}
            ]}]}))
            saved_bytes = path.read_bytes()
            environment = os.environ.copy(); environment.pop("KIGI_TEST_STUDIO_READBACK", None)
            environment["KIGI_TEST_STUDIO_CONFIG"] = str(path); environment["KIGI_TEST_STUDIO_LAYOUT"] = "1"
            app = subprocess.Popen([str(binary), "--studio-check", "--studio-editor-check"], env=environment, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, bufsize=0); processes.append(app)
            output = []; result = None; deadline = time.monotonic() + 35
            observed = None; last_sample = 0
            while time.monotonic() < deadline and result is None:
                if app.poll() is not None: break
                if time.monotonic() - last_sample >= .15:
                    current = frames(); last_sample = time.monotonic()
                    left_frame, right_frame = current[source.pid], current[target.pid]
                    width, height, x, y = [before[source.pid][key] for key in ["Width", "Height", "X", "Y"]]
                    left_ok = left_frame["Width"] >= width * .55 - 8 if minimum else abs(left_frame["Width"] - width * .35) <= 8
                    if left_ok and abs(left_frame["X"]-x) <= 8 and abs(right_frame["X"]-(x+width*.35)) <= 8 and abs(right_frame["Width"]-width*.65) <= 8 and all(abs(frame["Height"]-height)<=8 and abs(frame["Y"]-y)<=8 for frame in [left_frame,right_frame]):
                        observed = current
                readable, _, _ = select.select([app.stdout], [], [], .2)
                if readable:
                    line = app.stdout.readline().decode(errors="replace"); output.append(line)
                    if line.startswith("KIGI_STUDIO_EDITOR "): result = json.loads(line.removeprefix("KIGI_STUDIO_EDITOR "))
            assert result and result["success"] and result["restored"], "".join(output)
            assert observed, "No independent sample of the requested placement"
            for pid, reported in zip([source.pid,target.pid], result["reportedWidths"]):
                assert abs(observed[pid]["Width"] - before[source.pid]["Width"] * reported / 100) <= 8, "Studio geometry differed from CG bounds"
            after = frames()
            for pid in before:
                assert all(abs(before[pid][key] - after[pid][key]) <= 8 for key in ["X", "Y", "Width", "Height"]), {"before": before, "after": after}
            if minimum: assert path.read_bytes() == saved_bytes, "Apply/restore wrote saved config"
            assert json.loads(path.read_text())["profiles"][0]["windows"][0]["geometry"]["width"] == .35
            print("LAYOUT_VERIFIED " + json.dumps({"minimum_size": minimum, "real_placement_observed": True, "reported_geometry_matches": True, "real_frames_restored": True, "result": result}))
        finally:
            for process in reversed(processes): stop(process)
