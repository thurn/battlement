"""Exercise real Ditto interruption with private capacity and external tool fixtures."""
import fcntl
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time


SUITE = '''name = "interruption"
default_profile = "macos"
[player]
unity_project = "game"
scene = "game/Assets/Scenes/Game.unity"
rust_manifest = "rules/Cargo.toml"
[profiles.macos]
target = "macos"
display = { width = 640, height = 360, scale = 1.0 }
[[scenarios]]
name = "must not launch"
motion = "controlled"
[[scenarios.steps]]
advance = { frames = 1 }
'''

TOOL = '''#!/usr/bin/env python3
import json, os, subprocess, sys, time
from pathlib import Path
args = sys.argv[1:]
tool = Path(sys.argv[0]).name
if tool in ('rustc', 'xcrun', 'xcodebuild', 'odiff') or '--version' in args or '-version' in args:
    print({'unity':'6000.0.56f1','cargo':'cargo 1.98.1','rustc':'rustc 1.98.1','xcrun':'15.2','xcodebuild':'Xcode 26.0','odiff':'odiff 4.5.0'}[tool])
    sys.exit(0)
def argument(name):
    return args[args.index(name) + 1]
phase = 'rules' if tool == 'cargo' else ('shell' if argument('-executeMethod').endswith('BuildMacosShell') else 'content')
if phase == os.environ['FIXTURE_PHASE']:
    child = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(90)'])
    if tool == 'unity':
        Path(argument('-projectPath'), 'Assets', 'interrupted.tmp').write_text('temporary')
    Path(os.environ['FIXTURE_MARKER']).write_text(json.dumps([os.getpid(), child.pid]))
    try:
        child.wait()
    finally:
        child.terminate()
        child.wait()
elif phase == 'rules':
    artifact = Path(argument('--target-dir')) / argument('--target') / 'release/libbattlement_rules.dylib'
    artifact.parent.mkdir(parents=True, exist_ok=True)
    artifact.write_text('rules artifact')
elif phase == 'shell':
    artifact = Path(os.environ['BATTLEMENT_DITTO_BUILD_PATH']) / 'Contents/MacOS/BattlementDitto'
    artifact.parent.mkdir(parents=True, exist_ok=True)
    artifact.write_text('#!/bin/sh\\necho launched > "$FIXTURE_PLAYER_MARKER"\\nexit 1\\n')
    artifact.chmod(0o755)
elif phase == 'content':
    content = Path(os.environ['BATTLEMENT_DITTO_CONTENT_PATH'])
    content.mkdir(parents=True, exist_ok=True)
    (content / 'settings.json').write_text('{}')
if tool == 'unity':
    Path(argument('-logFile')).write_text('fixture')
'''


def wait_until(predicate, process, seconds=15):
    deadline = time.monotonic() + seconds
    while not predicate():
        assert process.poll() is None, 'Ditto exited before the requested boundary'
        assert time.monotonic() < deadline, 'Did not reach the requested boundary'
        time.sleep(0.02)


def alive(pid):
    result = subprocess.run(['ps', '-o', 'stat=', '-p', str(pid)], capture_output=True, text=True)
    return result.returncode == 0 and not result.stdout.strip().startswith('Z')


def run(binary, root):
    repo = root / 'repo'
    files = {
        '.gitignore': 'target/\n.logs/\n',
        'contracts/native-abi.json': '{}', 'contracts/wire-contract.json': '{}',
        'game/Assets/Scenes/Game.unity': 'scene',
        'game/Packages/manifest.json': '{"dependencies":{"com.battlement.client":"file:../../package"}}',
        'game/Packages/packages-lock.json': '{}',
        'game/ProjectSettings/ProjectVersion.txt': 'm_EditorVersion: 6000.0.56f1\n',
        'game/ProjectSettings/ProjectSettings.asset': 'settings',
        'package/package.json': '{"name":"com.battlement.client"}',
        'package/Runtime/Player.cs': 'public class Player {}',
        'rules/Cargo.toml': "[package]\nname='rules'\nversion='0.1.0'\n[lib]\nname='battlement_rules'\ncrate-type=['cdylib']\n[workspace]\n",
        'rules/src/lib.rs': 'pub fn rules() {}', 'Cargo.lock': 'version = 4\n',
        'ditto.toml': SUITE,
    }
    project = Path(__file__).resolve().parents[4]
    for name in ['unity_transaction.py', 'unity_metadata.py', 'process_priority.py']:
        files['scripts/' + name] = (project / 'scripts' / name).read_text()
    for name, content in files.items():
        path = repo / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
    for arguments in [['init', '-q'], ['add', '.'], ['-c', 'user.name=Fixture', '-c', 'user.email=fixture@invalid', 'commit', '-qm', 'fixture']]:
        subprocess.run(['git', *arguments], cwd=repo, check=True)
    tools = root / 'tools'
    tools.mkdir()
    for name in ['cargo', 'rustc', 'unity', 'xcrun', 'xcodebuild', 'odiff']:
        path = tools / name
        path.write_text(TOOL)
        path.chmod(0o755)
    slots = root / 'slots'
    slots.mkdir()
    unrelated = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(90)'])
    reports = []
    try:
        for phase in ['queued', 'rules', 'content', 'player']:
            marker = root / (phase + '.pids')
            retained = root / (phase + '.result.json')
            holders = [(slots / f'machine-heavy-{i}.lock').open('w+') for i in (range(6) if phase == 'queued' else [5])]
            if phase == 'player':
                holders.extend((slots / f'native-player-{i}.lock').open('w+') for i in range(3))
            for handle in holders:
                fcntl.flock(handle, fcntl.LOCK_EX)
            environment = os.environ | {
                'PATH': str(tools) + os.pathsep + os.environ['PATH'],
                'UNITY_EDITOR': str(tools / 'unity'), 'DITTO_ODIFF_PATH': str(tools / 'odiff'),
                'DITTO_CACHE_ROOT': str(root / ('cache-' + phase)),
                'BATTLEMENT_RESOURCE_SLOTS': str(slots), 'FIXTURE_PHASE': phase,
                'FIXTURE_MARKER': str(marker), 'FIXTURE_PLAYER_MARKER': str(root / 'player-launched'),
            }
            stdout = (root / (phase + '.stdout')).open('w')
            stderr = (root / (phase + '.stderr')).open('w')
            process = subprocess.Popen([binary, 'ditto', '--config', 'ditto.toml', 'run', '--json', '--output', str(retained)], cwd=repo, env=environment, stdout=stdout, stderr=stderr, process_group=0)
            try:
                assert os.getsid(process.pid) == os.getsid(0), 'fixture escaped the supervising session'
                boundary = (lambda: any(slots.glob(f'.machine-heavy.queue.*.{process.pid:010d}.*.lock'))) if phase == 'queued' else marker.exists
                if phase == 'player':
                    boundary = lambda: 'DITTO_BUILD=created' in (root / 'player.stderr').read_text() and any(slots.glob(f'.machine-heavy.queue.*.{process.pid:010d}.*.lock'))
                wait_until(boundary, process)
                started = time.monotonic()
                process.send_signal(signal.SIGINT)
                try:
                    code = process.wait(timeout=8)
                except subprocess.TimeoutExpired:
                    raise AssertionError(f'{phase}: interrupt did not stop Ditto at its build boundary')
                assert code == 130, (phase, code, (root / (phase + '.stderr')).read_text())
                result = json.loads(retained.read_text())
                assert result['status'] == 'interrupted', result
                assert result['exit_code'] == 130 and result['player_sessions'] == [], result
                assert result['scenarios'][0]['status_reason'] == 'run-interrupted', result
                assert any(p['name'] == ('launch' if phase == 'player' else 'build') and p['status'] == 'interrupted' for p in result['phases']), result
                assert not (root / 'player-launched').exists(), 'interruption launched a player'
                if phase in ('queued', 'player'):
                    assert not marker.exists(), 'queued interruption launched a build'
                else:
                    assert all(not alive(pid) for pid in json.loads(marker.read_text())), 'owned descendant survived'
                assert unrelated.poll() is None, 'unrelated process was stopped'
                assert not list(slots.glob(f'.machine-heavy.queue.*.{process.pid:010d}.*.lock')), 'queue ticket retained'
                reports.append({'phase': phase, 'seconds': time.monotonic() - started, 'exit_code': code})
            finally:
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait()
                if marker.exists():
                    for pid in json.loads(marker.read_text()):
                        if alive(pid):
                            os.kill(pid, signal.SIGKILL)
                stdout.close()
                stderr.close()
                for handle in holders:
                    handle.close()
            for i in range(6):
                with (slots / f'machine-heavy-{i}.lock').open('r+') as handle:
                    fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
            assert subprocess.check_output(['git', 'status', '--porcelain'], cwd=repo) == b'', 'Unity source was not restored'
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
        with tempfile.TemporaryDirectory(prefix='ditto-interruption-') as temporary:
            run(sys.argv[1], Path(temporary))
