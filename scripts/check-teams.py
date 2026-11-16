"""Owned-window placement, reapply, navigation and missing-app launch checks."""
import json
import os
from pathlib import Path
import plistlib
import select
import shutil
import subprocess
import tempfile
import time

binary = Path('dist/Kigi Windows.app/Contents/MacOS/kigi-windows').resolve()
def ready(process):
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline and process.poll() is None:
        readable, _, _ = select.select([process.stdout], [], [], .2)
        if readable:
            line = process.stdout.readline().decode(errors='replace').strip()
            if line.startswith('FIXTURE_READY '):
                assert line.endswith(' true'), line
                return
    raise AssertionError('Fixture did not become ready')
def stop(process):
    if process.poll() is None:
        process.terminate()
        try: process.wait(timeout=4)
        except subprocess.TimeoutExpired:
            process.kill(); process.wait()
# Launch Services does not retain discoverable app registrations in NSTemporaryDirectory.
# Install only owned fixtures under this runner's Applications and remove them afterward.
fixture_directory = Path.home() / 'Applications'
fixture_directory.mkdir(exist_ok=True)
with tempfile.TemporaryDirectory(prefix='Kigi-CI-', dir=fixture_directory) as directory:
    root = Path(directory)
    fixture = root / 'fixture'
    subprocess.run(['swiftc', 'scripts/focus-fixture.swift', '-o', str(fixture)], check=True)
    paths = {}
    for name in ['source', 'target']:
        contents = root / (name + '.app') / 'Contents'
        (contents / 'MacOS').mkdir(parents=True)
        shutil.copy2(fixture, contents / 'MacOS/fixture')
        (contents / 'Info.plist').write_bytes(plistlib.dumps({'CFBundleExecutable':'fixture','CFBundleIdentifier':'io.kigi.focus.'+name,'CFBundleName':'Kigi '+name+' fixture','CFBundlePackageType':'APPL'}))
        subprocess.run(['codesign','--force','--sign','-',str(contents.parent)],check=True)
        paths[name]=contents/'MacOS/fixture'
    for manual, minimum, reapply in [(False,False,False),(True,False,False),(False,True,False),(False,False,True)]:
        flags=(['--meeting-minimum'] if minimum else [])+(['--teams-reapply'] if reapply else [])
        process=subprocess.Popen([str(paths['source']),'--source','--teams-pair']+flags,stdout=subprocess.PIPE)
        try:
            ready(process)
            result=subprocess.run([str(binary),'--teams-check']+(['--manual-override'] if manual else [])+flags,env={**os.environ,'KIGI_TEST_TEAMS':'1'},capture_output=True,text=True,timeout=15)
            print(result.stdout,result.stderr)
            rows=[json.loads(line.removeprefix('KIGI_TEAMS_CHECK ')) for line in result.stdout.splitlines() if line.startswith('KIGI_TEAMS_CHECK ')]
            assert result.returncode==0 and rows==[{'manual':manual,'minimum':minimum,'reapply':reapply,'success':True}],result.stdout+result.stderr
        finally: stop(process)
    source=subprocess.Popen([str(paths['source']),'--source','--teams-pair'],stdout=subprocess.PIPE)
    target=None
    try:
        ready(source)
        target=subprocess.Popen([str(paths['target'])],stdout=subprocess.PIPE)
        ready(target)
        result=subprocess.run([str(binary),'--navigation-check'],env={**os.environ,'KIGI_TEST_NAVIGATION':'1'},capture_output=True,text=True,timeout=20)
        print(result.stdout,result.stderr)
        assert result.returncode==0 and 'KIGI_NAVIGATION_CHECK true' in result.stdout,result.stdout+result.stderr
    finally:
        stop(source)
        if target is not None: stop(target)
    subprocess.run([str(fixture),'--register',str(root/'source.app'),str(root/'target.app')],check=True)
    try:
        result=subprocess.run([str(binary),'--space-launch-check'],env={**os.environ,'KIGI_TEST_NAVIGATION':'1'},capture_output=True,text=True,timeout=15)
        print(result.stdout,result.stderr)
        assert result.returncode==0 and 'KIGI_SPACE_LAUNCH true' in result.stdout,result.stdout+result.stderr
    finally:
        subprocess.run([str(fixture),'--quit-test-apps'],check=True)
