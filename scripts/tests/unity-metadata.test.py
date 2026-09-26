#!/usr/bin/env python3
"""Exercise retained Unity metadata and explicit adoption through both public CLIs."""

from pathlib import Path
import json
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
RUNNER = ROOT / 'scripts/unity_transaction.py'
ADOPT = ROOT / 'scripts/unity_metadata.py'


def git(root, *args):
    return subprocess.check_output(['git', *args], cwd=root)


def verify(extension):
    with tempfile.TemporaryDirectory(prefix='unity-metadata-test.') as temporary:
        root = Path(temporary).resolve()

        def source(name):
            return f'{name}.{extension}'

        assets = root / 'Assets'
        assets.mkdir()
        (root / '.gitignore').write_text('.logs\n')
        (assets / source('Existing')).write_text('class Existing {}\n')
        git(root, 'init', '--quiet')
        git(root, 'add', '.')
        git(root, '-c', 'user.name=Fixture', '-c', 'user.email=ci@example.invalid',
            'commit', '--quiet', '-m', 'fixture')
        for name in ['New A', 'NewB', 'Preexisting']:
            (assets / source(name)).write_text(f'// {name}\n')
            git(root, 'add', f'Assets/{source(name)}')
        (assets / (source('Preexisting') + '.meta')).write_text('my metadata\n')
        (assets / source('Untracked')).write_text('my untracked source\n')
        (assets / 'Unsupported.txt').write_text('staged unrelated source')
        git(root, 'add', 'Assets/Unsupported.txt')
        before = git(root, 'status', '--porcelain=v1', '-z', '--untracked-files=all')
        index = (root / '.git/index').read_bytes()
        script = f"""
from pathlib import Path
for name in ['New A', 'NewB', 'Preexisting', 'Untracked', 'Existing']:
    Path(f'Assets/{{name}}.{extension}.meta').write_text(f'guid: {{name}}\\n')
Path('Assets/unrelated.txt').write_text('not metadata')
Path('Assets/Unsupported.txt.meta').write_text('unrelated metadata')
Path('Assets/New A.{extension}').write_text('editor changed source')
raise SystemExit(7)
"""
        run = subprocess.run([sys.executable, str(RUNNER), '--project', str(root),
            '--', sys.executable, '-c', script], cwd=root)
        assert run.returncode == 7
        assert (root / '.git/index').read_bytes() == index
        assert git(root, 'status', '--porcelain=v1', '-z', '--untracked-files=all') == before
        index = (root / '.git/index').read_bytes()
        assert (assets / (source('Preexisting') + '.meta')).read_text() == 'my metadata\n'
        assert (assets / source('New A')).read_text() == '// New A\n'
        directory, = (root / '.logs/ci/unity-transactions').iterdir()
        records = json.loads((directory / 'generated-metadata.json').read_text())
        assert set(records) == {f'Assets/{source("New A")}.meta', f'Assets/{source("NewB")}.meta'}
        retained = directory / 'generated-metadata' / f'Assets/{source("New A")}.meta'
        assert retained.read_text() == 'guid: New A\n'

        def adopt(*paths, success=True):
            result = subprocess.run([sys.executable, str(ADOPT), str(directory), *paths],
                cwd=root, capture_output=True, text=True)
            assert (result.returncode == 0) == success, result.stderr
            assert (root / '.git/index').read_bytes() == index
            return result

        first = f'Assets/{source("New A")}.meta'
        second = f'Assets/{source("NewB")}.meta'
        adopt(first, 'Assets/unrelated.txt', success=False)
        assert not (root / first).exists()
        adopt(f'Assets/{source("Preexisting")}.meta', success=False)
        adopt(f'Assets/{source("Untracked")}.meta', success=False)
        adopt('Assets/Unsupported.txt.meta', success=False)
        adopt(f'../escape.{extension}.meta', success=False)
        (root / second).write_text('mine')
        adopt(first, second, success=False)
        assert not (root / first).exists()
        assert (root / second).read_text() == 'mine'
        (root / second).unlink()
        (assets / source('New A')).write_text('unstaged')
        adopt(first, success=False)
        (assets / source('New A')).write_text('// New A\n')
        original = retained.read_bytes()
        retained.write_text('tampered')
        adopt(first, success=False)
        retained.write_bytes(original)
        (root / first).symlink_to(retained)
        adopt(first, success=False)
        (root / first).unlink()
        saved_assets = root / 'Assets-saved'
        assets.rename(saved_assets)
        assets.symlink_to(saved_assets, target_is_directory=True)
        adopt(first, success=False)
        assets.unlink()
        saved_assets.rename(assets)
        retained.unlink()
        retained.symlink_to(assets / source('New A'))
        adopt(first, success=False)
        retained.unlink()
        retained.write_bytes(original)
        assert json.loads(adopt(first).stdout)['adopted'] == [first]
        assert (root / first).read_bytes() == original
        assert not (root / second).exists()
        assert json.loads(adopt(first).stdout)['adopted'] == []
        git(root, 'add', first)
        index = (root / '.git/index').read_bytes()
        assert json.loads(adopt(first).stdout)['adopted'] == []
        # A newly staged source revision invalidates the retained metadata selection.
        (assets / source('NewB')).write_text('new staged revision')
        git(root, 'add', f'Assets/{source("NewB")}')
        index = (root / '.git/index').read_bytes()
        adopt(second, success=False)
        assert not (root / second).exists()
        assert (assets / (source('Preexisting') + '.meta')).read_text() == 'my metadata\n'
        assert (assets / source('Untracked')).read_text() == 'my untracked source\n'
        print(f'Unity {extension} metadata CLI checks passed: restoration, selection, conflicts, index, repeat adoption.')


if __name__ == '__main__':
    for extension in ('cs', 'json'):
        verify(extension)
