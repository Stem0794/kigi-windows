"""Bounded AX checks on an owned pair, no Teams or user-window inspection."""
import json
import os
from pathlib import Path
import plistlib
import select
import subprocess
import tempfile
import time

binary = Path('dist/Kigi Windows.app/Contents/MacOS/kigi-windows').resolve()
with tempfile.TemporaryDirectory(prefix='kigi-teams-') as directory:
    contents = Path(directory) / 'source.app/Contents'
    (contents / 'MacOS').mkdir(parents=True)
    fixture = contents / 'MacOS/fixture'
    subprocess.run(['swiftc', 'scripts/focus-fixture.swift', '-o', str(fixture)], check=True)
    (contents / 'Info.plist').write_bytes(plistlib.dumps({'CFBundleExecutable': 'fixture', 'CFBundleIdentifier': 'io.kigi.focus.source', 'CFBundleName': 'Kigi Teams fixture', 'CFBundlePackageType': 'APPL'}))
    subprocess.run(['codesign', '--force', '--sign', '-', str(contents.parent)], check=True)
    for manual, minimum in [(False, False), (True, False), (False, True)]:
        process = subprocess.Popen([str(fixture), '--source', '--teams-pair'] + (['--meeting-minimum'] if minimum else []), stdout=subprocess.PIPE)
        try:
            deadline = time.monotonic() + 8
            ready = False
            while time.monotonic() < deadline and process.poll() is None:
                readable, _, _ = select.select([process.stdout], [], [], .2)
                if readable:
                    line = process.stdout.readline().decode(errors='replace').strip()
                    if line.startswith('FIXTURE_READY '):
                        ready = line.endswith(' true')
                        break
            assert ready, 'Teams fixture did not become ready'
            result = subprocess.run([str(binary), '--teams-check'] + (['--manual-override'] if manual else []) + (['--meeting-minimum'] if minimum else []), env={**os.environ, 'KIGI_TEST_TEAMS': '1'}, capture_output=True, text=True, timeout=15)
            print(result.stdout, result.stderr)
            rows = [json.loads(line.removeprefix('KIGI_TEAMS_CHECK ')) for line in result.stdout.splitlines() if line.startswith('KIGI_TEAMS_CHECK ')]
            assert result.returncode == 0 and rows == [{'manual': manual, 'minimum': minimum, 'success': True}], result.stdout + result.stderr
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=4)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
