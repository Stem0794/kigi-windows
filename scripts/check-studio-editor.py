"""One isolated native editor flow and one relaunch/readback; no live config writes."""
import json
import os
from pathlib import Path
import select
import subprocess
import tempfile
import time

with tempfile.TemporaryDirectory(prefix="kigi-studio-editor-") as directory:
    path = Path(directory) / "config.json"
    path.write_text(json.dumps({"schema_version": 1, "windows": [
        {"bundle_id": "com.google.Chrome", "geometry": {"x": .1, "y": .1, "width": .6, "height": .8}},
        {"bundle_id": "com.microsoft.teams2", "geometry": {"x": .35, "y": .2, "width": .5, "height": .7}}
    ]}))
    environment = os.environ.copy()
    environment["KIGI_TEST_STUDIO_CONFIG"] = str(path)
    for readback in [False, True]:
        environment.pop("KIGI_TEST_STUDIO_READBACK", None)
        if readback:
            environment["KIGI_TEST_STUDIO_READBACK"] = "1"
        process = subprocess.Popen(["dist/Kigi Windows.app/Contents/MacOS/kigi-windows", "--studio-check", "--studio-editor-check"], env=environment, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, bufsize=0)
        output = []
        result = None
        try:
            deadline = time.monotonic() + 30
            while time.monotonic() < deadline and result is None:
                if process.poll() is not None:
                    break
                readable, _, _ = select.select([process.stdout], [], [], .2)
                if readable:
                    line = process.stdout.readline().decode(errors="replace")
                    output.append(line)
                    if line.startswith("KIGI_STUDIO_EDITOR "):
                        result = json.loads(line.removeprefix("KIGI_STUDIO_EDITOR "))
            assert result and result["success"] and result["width"] == .35, "".join(output)
            catalog = json.loads(path.read_text())
            assert catalog["schema_version"] == 2 and len(catalog["profiles"]) == 1
            assert catalog["profiles"][0]["windows"][0]["geometry"]["width"] == .35
            print("EDITOR_VERIFIED " + json.dumps({"relaunch_readback": readback, "result": result}))
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=4)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
