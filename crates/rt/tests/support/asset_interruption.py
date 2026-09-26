"""Exercise cancellation through the public Reactant preparation consumers."""
import fcntl
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time

from build_interruption import SUITE, alive, wait_until


TOOL = r'''#!/usr/bin/env python3
import base64, hashlib, json, os, socket, subprocess, sys, time
from pathlib import Path
args = sys.argv[1:]
tool = Path(sys.argv[0]).name
phase = os.environ['FIXTURE_PHASE']
if tool == 'rustc':
    print('rustc 1.98.1\nhost: aarch64-apple-darwin')
    sys.exit(0)
if tool in ('xcrun', 'xcodebuild', 'odiff') or '--version' in args or '-version' in args:
    print({'unity':'6000.0.56f1','cargo':'cargo 1.98.1','xcrun':'15.2','xcodebuild':'Xcode 26.0','odiff':'odiff 4.5.0'}[tool])
    sys.exit(0)
def block():
    child = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(90)'])
    Path(os.environ['FIXTURE_MARKER']).write_text(json.dumps([os.getpid(), child.pid]))
    try:
        child.wait()
    finally:
        child.terminate()
        child.wait()
if tool == 'cargo' and args[0] == 'metadata':
    if phase == 'metadata':
        block()
    manifest = Path(args[args.index('--manifest-path') + 1])
    reactant = manifest.parent.parent / 'reactant/Cargo.toml'
    def package(name, manifest):
        return dict(id=name, name=name, version='0.1.0', source=None, manifest_path=str(manifest), targets=[dict(name=name, kind=['lib'], src_path=str(manifest.parent / 'src/lib.rs'))])
    print(json.dumps(dict(packages=[package('rules', manifest), package('reactant', reactant)], resolve=dict(nodes=[dict(id='rules', deps=[dict(name='reactant', pkg='reactant')]), dict(id='reactant', deps=[])]))))
elif tool == 'browser':
    if phase == 'browser-startup':
        block()
    profile = Path(next(x.split('=', 1)[1] for x in args if x.startswith('--user-data-dir=')))
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        listener.listen()
        (profile / 'DevToolsActivePort').write_text(str(listener.getsockname()[1]) + '\n/devtools/browser/fixture\n')
        connection, _ = listener.accept()
        with connection:
            if phase == 'browser-handshake':
                block()
            request = b''
            while b'\r\n\r\n' not in request:
                request += connection.recv(4096)
            key = next(x.split(':', 1)[1].strip() for x in request.decode().split('\r\n') if x.lower().startswith('sec-websocket-key:'))
            accept = base64.b64encode(hashlib.sha1((key + '258EAFA5-E914-47DA-95CA-C5AB0DC85B11').encode()).digest()).decode()
            connection.sendall(('HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ' + accept + '\r\n\r\n').encode())
            assert connection.recv(4096), 'client never sent its protocol request'
            block()
else:
    Path(os.environ['FIXTURE_BUILD']).write_text('unexpected build')
    sys.exit(9)
'''


def run(binary, root):
    repo = root / 'repo'
    files = {
        '.gitignore': 'game/Library/\n.logs/\n',
        'contracts/native-abi.json': '{}', 'contracts/wire-contract.json': '{}',
        'game/Assets/Scenes/Game.unity': 'scene',
        'game/Assets/Generated/BattlementReactant/keep.png': 'original installed asset',
        'game/Packages/manifest.json': '{"dependencies":{"com.battlement.client":"file:../../package"}}',
        'game/ProjectSettings/ProjectVersion.txt': 'm_EditorVersion: 6000.0.56f1\n',
        'game/reactant.toml': '[project]\napplication="Fixture"\nscene="Assets/Scenes/Game.unity"\n',
        'package/package.json': '{"name":"com.battlement.client"}',
        'game/rules/Cargo.toml': '[package]\nname="rules"\nversion="0.1.0"\n[lib]\ncrate-type=["cdylib"]\n[workspace]\n',
        'game/rules/src/lib.rs': 'reactant::asset_generator::generate! { @background PANEL { @canvas 8px 8px; @subject 1px 1px 6px 6px; background: linear-gradient(red, blue); } }',
        'game/reactant/Cargo.toml': '[package]\nname="reactant"\nversion="0.1.0"\n',
        'game/reactant/src/lib.rs': '',
        'ditto.toml': SUITE.replace('unity_project = "game"', 'reactant = true\nunity_project = "game"').replace('rust_manifest = "rules/Cargo.toml"', 'rust_manifest = "game/rules/Cargo.toml"'),
    }
    for name, content in files.items():
        path = repo / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
    for args in [['init', '-q'], ['add', '.'], ['-c', 'user.name=Fixture', '-c', 'user.email=fixture@invalid', 'commit', '-qm', 'fixture']]:
        subprocess.run(['git', *args], cwd=repo, check=True)
    tools = root / 'tools'
    tools.mkdir()
    for name in ['cargo', 'rustc', 'unity', 'xcrun', 'xcodebuild', 'odiff', 'browser']:
        path = tools / name
        path.write_text(TOOL)
        path.chmod(0o755)
    temporary = root / 'temporary' / 'T'
    temporary.mkdir(parents=True)
    slots = root / 'slots'
    slots.mkdir()
    unrelated = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(90)'])
    reports = []
    try:
        for phase in ['queued', 'metadata', 'browser-startup', 'browser-handshake', 'browser-protocol']:
            marker = root / (phase + '.pids')
            retained = root / (phase + '.result.json')
            report_path = root / (phase + '.work.json')
            holders = [(slots / 'machine-heavy-5.lock').open('w+')]
            if phase == 'queued':
                holders.extend((slots / f'browser-{i}.lock').open('w+') for i in range(2))
            for handle in holders:
                fcntl.flock(handle, fcntl.LOCK_EX)
            environment = os.environ | {
                'PATH': str(tools) + os.pathsep + os.environ['PATH'],
                'TMPDIR': str(temporary) + os.sep,
                'UNITY_EDITOR': str(tools / 'unity'), 'DITTO_ODIFF_PATH': str(tools / 'odiff'),
                'DITTO_CACHE_ROOT': str(root / ('cache-' + phase)),
                'BATTLEMENT_RESOURCE_SLOTS': str(slots), 'FIXTURE_PHASE': phase,
                'FIXTURE_MARKER': str(marker), 'FIXTURE_BUILD': str(root / 'build-started'),
            }
            ditto = phase in ('queued', 'metadata')
            args = ['ditto', '--config', 'ditto.toml', 'run', '--json', '--output', str(retained)] if ditto else ['assets', 'generate', '--project', 'game', '--browser', str(tools / 'browser'), '--work-report', str(report_path)]
            with (root / (phase + '.stdout')).open('w') as stdout, (root / (phase + '.stderr')).open('w') as stderr:
                process = subprocess.Popen([binary, *args], cwd=repo, env=environment, stdout=stdout, stderr=stderr, process_group=0)
                try:
                    assert os.getsid(process.pid) == os.getsid(0), 'fixture escaped the supervising session'
                    boundary = (lambda: any(slots.glob(f'.browser.queue.*.{process.pid:010d}.*.lock'))) if phase == 'queued' else marker.exists
                    wait_until(boundary, process)
                    started = time.monotonic()
                    process.send_signal(signal.SIGINT)
                    code = process.wait(timeout=5)
                    assert code == 130, (phase, code, (root / (phase + '.stderr')).read_text())
                    if ditto:
                        result = json.loads(retained.read_text())
                        assert result['status'] == 'interrupted' and result['exit_code'] == 130, result
                        assert result['player_sessions'] == [] and result['scenarios'][0]['status_reason'] == 'run-interrupted', result
                    else:
                        work = json.loads(report_path.read_text())
                        assert work['browserLaunches'] == 1 and work['filesWritten'] == 0, work
                    assert not (root / 'build-started').exists(), 'asset cancellation launched a build'
                    assert unrelated.poll() is None, 'foreign work was stopped'
                    if marker.exists():
                        assert all(not alive(pid) for pid in json.loads(marker.read_text())), 'owned descendant survived'
                    assert not list(slots.glob(f'.*.queue.*.{process.pid:010d}.*.lock')), 'queue ticket retained'
                    reports.append({'phase': phase, 'seconds': time.monotonic() - started, 'exit_code': code})
                finally:
                    if process.poll() is None:
                        os.killpg(process.pid, signal.SIGKILL)
                        process.wait()
                    if marker.exists():
                        for pid in json.loads(marker.read_text()):
                            if alive(pid):
                                os.kill(pid, signal.SIGKILL)
                    for handle in holders:
                        handle.close()
            for prefix, count in [('machine-heavy', 6), ('browser', 2)]:
                for i in range(count):
                    with (slots / f'{prefix}-{i}.lock').open('a+') as handle:
                        fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
            if not ditto:
                with (temporary.parent / 'X/com.google.Chrome.code_sign_clone/.battlement.lock').open('r+') as handle:
                    fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
            assert subprocess.check_output(['git', 'status', '--porcelain'], cwd=repo) == b'', 'source or installed assets changed'
    finally:
        unrelated.terminate()
        unrelated.wait()
        (root / 'report.json').write_text(json.dumps(reports, indent=2))
    print(json.dumps(reports))


if __name__ == '__main__':
    if len(sys.argv) == 3:
        retained_root = Path(sys.argv[2])
        retained_root.mkdir()
        run(sys.argv[1], retained_root)
    else:
        with tempfile.TemporaryDirectory(prefix='asset-interruption-') as temporary:
            run(sys.argv[1], Path(temporary))
