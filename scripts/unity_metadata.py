#!/usr/bin/env python3
"""Inspect retained generated-metadata.json; adopt only explicitly named .cs.meta files."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess


def git(repository: Path, *arguments: str, index_file: Path | None = None) -> bytes:
    environment = os.environ.copy()
    if index_file is not None:
        environment['GIT_INDEX_FILE'] = str(index_file)
    return subprocess.check_output(['git', *arguments], cwd=repository, env=environment)


def regular_path(root: Path, relative: str) -> Path:
    """Reject noncanonical paths and symlinks, including symlinked parents."""
    path = Path(relative)
    if path.is_absolute() or '..' in path.parts or path.as_posix() != relative:
        raise ValueError(f'Expected a repository-relative path: {relative}')
    target = root / path
    for component in (target, *target.parents):
        if component.is_symlink():
            raise ValueError(f'Symlink is not eligible for metadata adoption: {relative}')
        if component == root:
            break
    return target


def digest(contents: bytes) -> str:
    return hashlib.sha256(contents).hexdigest()


def staged_sources(
    repository: Path, pathspecs: tuple[str, ...], index_file: Path | None = None,
) -> dict[str, str]:
    """Snapshot regular staged C# additions for a transaction or adoption check."""
    paths = git(
        repository, 'diff', '--cached', '--name-only', '--diff-filter=A',
        '--no-renames', '-z', '--', *pathspecs, index_file=index_file,
    ).split(b'\0')
    sources = {}
    for raw in paths:
        relative = raw.decode('utf-8', 'surrogateescape')
        if not relative.endswith('.cs'):
            continue
        try:
            path = regular_path(repository, relative)
        except ValueError:
            continue
        if not path.is_file():
            continue
        entry = git(repository, 'ls-files', '--stage', '--', f':(literal){relative}', index_file=index_file)
        if entry.split(maxsplit=1)[0] not in {b'100644', b'100755'}:
            continue
        staged = git(repository, 'show', f':{relative}', index_file=index_file)
        if path.read_bytes() == staged:
            sources[relative] = digest(staged)
    return sources


def retain(directory: Path, journal: dict, created: list[str]) -> None:
    """Preserve only newly generated metadata for the captured source additions."""
    repository = Path(journal['repository'])
    sources = journal.get('metadata_sources', {})
    retained = {}
    for relative in created:
        source = relative.removesuffix('.meta')
        if relative == source or source not in sources:
            continue
        try:
            path = regular_path(repository, relative)
        except ValueError:
            continue
        if not path.is_file():
            continue
        contents = path.read_bytes()
        destination = directory / 'generated-metadata' / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(contents)
        retained[relative] = {'source_sha256': sources[source], 'sha256': digest(contents)}
    (directory / 'generated-metadata.json').write_text(json.dumps(retained, indent=2) + '\n')
    if retained:
        print(f'Generated C# metadata retained for explicit adoption: {directory}')


def adopt(repository: Path, directory: Path, requested: list[str]) -> list[str]:
    """Restore selected metadata without overwriting differing files or staging it."""
    repository = repository.resolve()
    directory = directory.resolve()
    if directory.parent != repository / '.logs/ci/unity-transactions':
        raise ValueError('Select a transaction from this repository')
    journal = json.loads((directory / 'journal.json').read_text())
    if journal['repository'] != str(repository) or journal['state'] != 'verified':
        raise ValueError('Metadata adoption requires a verified transaction in this repository')
    retained = json.loads((directory / 'generated-metadata.json').read_text())
    sources = staged_sources(repository, tuple(journal['pathspecs']))
    pending = []
    # Validate the entire explicit selection before writing any destination.
    for relative in dict.fromkeys(requested):
        target = regular_path(repository, relative)
        record = retained.get(relative)
        if record is None or not relative.endswith('.cs.meta'):
            raise ValueError(f'No eligible retained metadata: {relative}')
        source = relative.removesuffix('.meta')
        if sources.get(source) != record['source_sha256']:
            raise ValueError(f'Source is no longer the same staged addition: {source}')
        contents = regular_path(directory / 'generated-metadata', relative).read_bytes()
        if digest(contents) != record['sha256']:
            raise ValueError(f'Retained metadata changed: {relative}')
        if target.exists():
            if not target.is_file() or target.read_bytes() != contents:
                raise ValueError(f'Existing metadata differs: {relative}')
        else:
            if git(repository, 'ls-files', '-z', '--', f':(literal){relative}'):
                raise ValueError(f'Metadata is already tracked: {relative}')
            pending.append((target, contents))
    for target, contents in pending:
        with target.open('xb') as output:
            output.write(contents)
    return [str(path.relative_to(repository)) for path, _ in pending]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('transaction', type=Path, help='Retained Unity transaction directory')
    parser.add_argument('metadata', nargs='+', help='Exact repository-relative .cs.meta paths to adopt')
    arguments = parser.parse_args()
    repository = Path(git(Path.cwd(), 'rev-parse', '--show-toplevel').decode().strip())
    try:
        adopted = adopt(repository, arguments.transaction, arguments.metadata)
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        parser.exit(1, f'{error}\n')
    print(json.dumps({'adopted': adopted, 'staged': False}))


if __name__ == '__main__':
    main()
