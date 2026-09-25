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


def main():
    with tempfile.TemporaryDirectory(prefix='unity-metadata-test.') as temporary:
        root = Path(temporary).resolve()
        assets = root / 'Assets'
        assets.mkdir()
        (root / '.gitignore').write_text('.logs\n')
        (assets / 'Existing.cs').write_text('class Existing {}\n')
        git(root, 'init', '--quiet')
        git(root, 'add', '.')
        git(root, '-c', 'user.name=Fixture', '-c', 'user.email=ci@example.invalid',
            'commit', '--quiet', '-m', 'fixture')
        for name in ['New A', 'NewB', 'Preexisting']:
            (assets / f'{name}.cs').write_text(f'// {name}\n')
            git(root, 'add', f'Assets/{name}.cs')
        (assets / 'Preexisting.cs.meta').write_text('my metadata\n')
        (assets / 'Untracked.cs').write_text('my untracked source\n')
        before = git(root, 'status', '--porcelain=v1', '-z', '--untracked-files=all')
        index = (root / '.git/index').read_bytes()
        script = """
from pathlib import Path
for name in ['New A', 'NewB', 'Preexisting', 'Untracked', 'Existing']:
    Path(f'Assets/{name}.cs.meta').write_text(f'guid: {name}\\n')
Path('Assets/unrelated.txt').write_text('not metadata')
Path('Assets/New A.cs').write_text('editor changed source')
raise SystemExit(7)
"""
        run = subprocess.run([sys.executable, str(RUNNER), '--project', str(root),
            '--', sys.executable, '-c', script], cwd=root)
        assert run.returncode == 7
        assert (root / '.git/index').read_bytes() == index
        assert git(root, 'status', '--porcelain=v1', '-z', '--untracked-files=all') == before
        index = (root / '.git/index').read_bytes()
        assert (assets / 'Preexisting.cs.meta').read_text() == 'my metadata\n'
        assert (assets / 'New A.cs').read_text() == '// New A\n'
        directory, = (root / '.logs/ci/unity-transactions').iterdir()
        records = json.loads((directory / 'generated-metadata.json').read_text())
        assert set(records) == {'Assets/New A.cs.meta', 'Assets/NewB.cs.meta'}
        retained = directory / 'generated-metadata/Assets/New A.cs.meta'
        assert retained.read_text() == 'guid: New A\n'

        def adopt(*paths, success=True):
            result = subprocess.run([sys.executable, str(ADOPT), str(directory), *paths],
                cwd=root, capture_output=True, text=True)
            assert (result.returncode == 0) == success, result.stderr
            assert (root / '.git/index').read_bytes() == index
            return result

        first = 'Assets/New A.cs.meta'
        second = 'Assets/NewB.cs.meta'
        adopt(first, 'Assets/unrelated.txt', success=False)
        assert not (root / first).exists()
        adopt('Assets/Preexisting.cs.meta', success=False)
        adopt('Assets/Untracked.cs.meta', success=False)
        adopt('../escape.cs.meta', success=False)
        (root / second).write_text('mine')
        adopt(first, second, success=False)
        assert not (root / first).exists()
        assert (root / second).read_text() == 'mine'
        (root / second).unlink()
        (assets / 'New A.cs').write_text('unstaged')
        adopt(first, success=False)
        (assets / 'New A.cs').write_text('// New A\n')
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
        retained.symlink_to(assets / 'New A.cs')
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
        (assets / 'NewB.cs').write_text('new staged revision')
        git(root, 'add', 'Assets/NewB.cs')
        index = (root / '.git/index').read_bytes()
        adopt(second, success=False)
        assert not (root / second).exists()
        assert (assets / 'Preexisting.cs.meta').read_text() == 'my metadata\n'
        assert (assets / 'Untracked.cs').read_text() == 'my untracked source\n'
        print('Unity metadata CLI checks passed: restoration, selection, conflicts, index, repeat adoption.')


if __name__ == '__main__':
    main()
