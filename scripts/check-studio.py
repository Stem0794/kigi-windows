"""One real WKWebView load: bootstrap, rendered DOM and window lifecycle."""
import json
import select
import subprocess
import time
from pathlib import Path

config = Path.home() / "Library/Application Support/io.kigi.windows/config.json"
before = config.read_bytes() if config.exists() else None
app = subprocess.Popen(["dist/Kigi Windows.app/Contents/MacOS/kigi-windows", "--studio-check"], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, bufsize=0)
rows = {}
output = []
try:
    deadline = time.monotonic() + 22
    while time.monotonic() < deadline and len(rows) < 3:
        if app.poll() is not None:
            break
        readable, _, _ = select.select([app.stdout], [], [], 0.2)
        if readable:
            line = app.stdout.readline().decode(errors="replace")
            output.append(line)
            for prefix in ["KIGI_STUDIO_READY", "KIGI_STUDIO_RENDER", "KIGI_STUDIO_LIFECYCLE"]:
                if line.startswith(prefix + " "):
                    rows[prefix] = json.loads(line.removeprefix(prefix + " "))
    assert app.poll() is None and len(rows) == 3, "".join(output)
    assert rows["KIGI_STUDIO_READY"]["protocolVersion"] == 1
    assert rows["KIGI_STUDIO_RENDER"] == {"title": "Un espace pour chaque moment.", "tabs": 4, "busy": "false", "error": False}, rows
    lifecycle = rows["KIGI_STUDIO_LIFECYCLE"]
    assert lifecycle["reopened"] and lifecycle["hotkeysRetained"], rows
    assert (config.read_bytes() if config.exists() else None) == before, "Read-only UI changed config"
    print("STUDIO_VERIFIED " + json.dumps(rows))
finally:
    if app.poll() is None:
        app.terminate()
        try:
            app.wait(timeout=4)
        except subprocess.TimeoutExpired:
            app.kill()
            app.wait()
