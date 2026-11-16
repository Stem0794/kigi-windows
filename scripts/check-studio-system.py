"""Isolated native shortcut/UI flow, actual Carbon conflict, capture cancel and persisted readback."""
import json
import os
from pathlib import Path
import select
import subprocess
import tempfile
import time


def stop(process):
    if process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=4)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()

with tempfile.TemporaryDirectory(prefix="kigi-system-") as directory:
    root = Path(directory)
    config = root / "config.json"
    config.write_text(json.dumps({"schema_version": 2, "next_id": 1, "profiles": []}))
    before = config.read_bytes()
    fixture = root / "fixture"
    subprocess.run(["swiftc", "scripts/focus-fixture.swift", "-o", str(fixture)], check=True)
    holder = subprocess.Popen([str(fixture), "--hotkey-reservation"], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, bufsize=0)
    try:
        assert holder.stdout.readline().decode().strip() == "HOTKEY_READY 0", "Could not reserve test combination"
        for readback in [False, True]:
            environment = os.environ.copy()
            environment.pop("KIGI_TEST_STUDIO_LAYOUT", None); environment.pop("KIGI_TEST_STUDIO_READBACK", None)
            environment["KIGI_TEST_STUDIO_CONFIG"] = str(config); environment["KIGI_TEST_STUDIO_SYSTEM"] = "1"
            if readback: environment["KIGI_TEST_STUDIO_READBACK"] = "1"
            process = subprocess.Popen(["dist/Kigi Windows.app/Contents/MacOS/kigi-windows", "--studio-check", "--studio-editor-check", "--studio-system-check"], env=environment, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, bufsize=0)
            output = []; result = None
            try:
                deadline = time.monotonic() + 35
                while time.monotonic() < deadline and result is None:
                    if process.poll() is not None: break
                    readable, _, _ = select.select([process.stdout], [], [], .2)
                    if readable:
                        line = process.stdout.readline().decode(errors="replace"); output.append(line)
                        if line.startswith("KIGI_STUDIO_EDITOR "): result = json.loads(line.removeprefix("KIGI_STUDIO_EDITOR "))
                assert result and result["success"], "".join(output)
                settings = json.loads((root / "shortcuts.json").read_text())
                assert settings["schema_version"] == 1 and len(settings["entries"]) == 7 and settings["entries"][0]["command"]["kind"] == "Maximize"
                assert config.read_bytes() == before, "Shortcut operations altered spaces"
                if not readback:
                    clipboard = subprocess.check_output(["pbpaste"], text=True)
                    assert "Kigi Windows — diagnostic" in clipboard and "Derniers événements" in clipboard
                print("SYSTEM_VERIFIED " + json.dumps({"readback": readback, "result": result, "spaces_unchanged": True}))
            finally:
                stop(process)
    finally:
        stop(holder)
