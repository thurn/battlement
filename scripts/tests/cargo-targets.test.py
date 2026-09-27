#!/usr/bin/env python3
"""Exercise cross-checkout Cargo reuse, stale inputs, and real writer leases."""

import json
import hashlib
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import tomllib
import unittest
from unittest.mock import patch

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import cargo_targets
import cargo_target
from ci_cache import CiCache
import resource_slots

ITOA = next(package['version'] for package in tomllib.loads(
    (SCRIPTS.parent / 'Cargo.lock').read_text())['package'] if package['name'] == 'itoa')


class CargoCommandTests(unittest.TestCase):
    def test_cargo_separator_and_flags_are_forwarded_unchanged(self):
        command = ['cargo', 'clippy', '--all-targets', '--', '-D', 'warnings']
        manifest = 'samples/hearts/rules/Cargo.toml'
        for prefix, workspace in [([], None), ([manifest], Path(manifest))]:
            with self.subTest(workspace=workspace):
                with patch.object(sys, 'argv', ['cargo_target.py', *prefix, '--run', *command]), \
                     patch.object(cargo_target.ci, 'run_cargo') as run:
                    cargo_target.main()
                run.assert_called_once_with(workspace, command)


@unittest.skipUnless(os.name == "posix", "warm target leases require inherited POSIX descriptors")
class CargoTargetsTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.cache = self.root / 'cache'
        self.resources = self.root / 'resources'
        self.patch = patch.object(resource_slots, 'GLOBAL_RESOURCE_ROOT', self.resources)
        self.patch.start()
        self.addCleanup(self.patch.stop)
        self.old = self.checkout('old', 7)
        self.new = self.checkout('new', 8)

    def checkout(self, name, value):
        root = self.root / name
        (root / 'src').mkdir(parents=True)
        (root / 'local/src').mkdir(parents=True)
        (root / 'Cargo.toml').write_text('[package]\nname="target-fixture"\nversion="0.1.0"\nedition="2021"\n'
            '[dependencies]\nlocal-fixture={path="local"}\n' + f'itoa="={ITOA}"\n')
        (root / 'src/lib.rs').write_text('#[test]\nfn checkout_input_is_current() {\n'
            'assert_eq!(itoa::Buffer::new().format(7), "7");\n'
            'assert_eq!(local_fixture::value(), 7, "current path dependency executed");\n}\n')
        (root / 'local/Cargo.toml').write_text('[package]\nname="local-fixture"\nversion="0.1.0"\nedition="2021"\n')
        (root / 'local/src/lib.rs').write_text('include!(concat!(env!("OUT_DIR"), "/value.rs"));\n')
        (root / 'local/value.txt').write_text(str(value))
        (root / 'local/build.rs').write_text(
            'fn main() { println!("cargo:rerun-if-changed=value.txt"); '
            'let value = std::fs::read_to_string("value.txt").unwrap(); '
            'let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()); '
            'std::fs::write(out.join("value.rs"), format!("pub fn value() -> i32 {{{}}}", value)).unwrap(); }\n')
        subprocess.run(['cargo', 'generate-lockfile', '--offline'], cwd=root, check=True, capture_output=True)
        return root

    def build(self, root):
        with cargo_targets.environment(root, self.cache, None) as env:
            result = subprocess.run(['cargo', 'test', '--offline', '--message-format=json'],
                cwd=root, **env.process_options(), capture_output=True, text=True)
            artifacts = [json.loads(line) for line in result.stdout.splitlines()
                         if line.startswith('{"reason":"compiler-artifact"')]
            return Path(env['CARGO_TARGET_DIR']), result, artifacts

    def test_registry_reuse_never_reuses_older_checkout_or_path_dependency(self):
        target, old, artifacts = self.build(self.old)
        self.assertEqual(old.returncode, 0, old.stdout + old.stderr)
        self.assertTrue(any(item['target']['name'] == 'itoa' and not item['fresh'] for item in artifacts))
        for path in self.new.rglob('*'):
            if path.is_file():
                os.utime(path, ns=(1, 1))
        other_target, new, artifacts = self.build(self.new)
        self.assertEqual(other_target, target)
        self.assertNotEqual(new.returncode, 0)
        self.assertIn('current path dependency executed', new.stdout)
        self.assertTrue(any(item['target']['name'] == 'itoa' and item['fresh'] for item in artifacts))
        self.assertTrue(any(item['target']['name'] == 'local_fixture' and not item['fresh'] for item in artifacts))
        _, old_again, _ = self.build(self.old)
        self.assertEqual(old_again.returncode, 0, old_again.stdout + old_again.stderr)

    def test_failure_releases_target_and_existing_pruner_accounts_for_pool(self):
        with self.assertRaises(RuntimeError):
            with cargo_targets.environment(self.old, self.cache, None) as env:
                target = Path(env['CARGO_TARGET_DIR'])
                (target / 'allocated').write_bytes(b'x' * 8192)
                raise RuntimeError('cancelled fixture')
        with cargo_targets.environment(self.new, self.cache, None) as env:
            self.assertEqual(Path(env['CARGO_TARGET_DIR']), target)
        cache = CiCache(self.old, self.cache, {})
        with resource_slots.compiler_maintenance_lease():
            result = cache.prune(target_bytes=0, high_water_bytes=1)
        self.assertIn(target, result.removed)
        self.assertFalse(target.exists())

    def test_identical_source_keeps_the_current_manifest_directory(self):
        source = '''#[test]
fn current_checkout() {
    assert_eq!(env!("CARGO_MANIFEST_DIR"), std::env::current_dir().unwrap().to_str().unwrap());
}
'''
        for root in (self.old, self.new):
            (root / 'src/lib.rs').write_text(source)
        _, old, _ = self.build(self.old)
        self.assertEqual(old.returncode, 0, old.stdout + old.stderr)
        for path in self.new.rglob('*'):
            if path.is_file():
                os.utime(path, ns=(1, 1))
        _, new, artifacts = self.build(self.new)
        self.assertEqual(new.returncode, 0, new.stdout + new.stderr)
        self.assertTrue(any(item['target']['name'] == 'itoa' and item['fresh'] for item in artifacts))

    def test_reused_checkout_detects_committed_and_staged_backdated_inputs(self):
        def git(*arguments):
            return subprocess.run(['git', '-c', 'core.hooksPath=/dev/null',
                '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
                *arguments], cwd=self.old, check=True, capture_output=True).stdout
        git('init', '--quiet')
        git('add', '.')
        git('commit', '--quiet', '-m', 'Fixture baseline')
        _, first, _ = self.build(self.old)
        self.assertEqual(first.returncode, 0, first.stdout + first.stderr)
        value = self.old / 'local/value.txt'
        value.write_text('8')
        os.utime(value, ns=(1, 1))
        git('add', '.')
        git('commit', '--quiet', '-m', 'Different backdated input')
        _, committed, artifacts = self.build(self.old)
        self.assertNotEqual(committed.returncode, 0, committed.stdout + committed.stderr)
        self.assertTrue(any(item['target']['name'] == 'itoa' and item['fresh'] for item in artifacts))
        value.write_text('7')
        os.utime(value, ns=(1, 1))
        # Rehash even when the same-size, backdated edit matches Git's stat cache.
        git('add', '--renormalize', '.')
        self.assertEqual(git('show', ':local/value.txt'), b'7')
        _, staged, _ = self.build(self.old)
        self.assertEqual(staged.returncode, 0, staged.stdout + staged.stderr)

    def test_compiler_wrapper_preserves_stable_language_gates(self):
        source = self.old / 'src/lib.rs'
        source.write_text('#![feature(negative_impls)]\n#![allow(unstable_features)]\n')
        with cargo_targets.environment(self.old, self.cache, None) as env:
            result = subprocess.run([
                env['RUSTC_WRAPPER'], 'rustc', '--crate-name', 'language_gate',
                '--crate-type', 'lib', str(source), '--cap-lints', 'allow',
                '--out-dir', str(self.root),
            ], **env.process_options(), capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('unstable_features', result.stderr)

    def test_native_wrapper_format(self):
        subprocess.run(['rustfmt', '--check', '--edition', '2024',
                        str(SCRIPTS / 'cargo_rustc.rs')], check=True, capture_output=True)

    def test_explicit_wrapper_survives_a_nested_checkout_handoff(self):
        wrapper = self.root / 'outer-wrapper'
        marker = self.root / 'outer-invocations'
        wrapper.write_text(f'#!/bin/sh\necho invoked >> "{marker}"\nexec "$@"\n')
        wrapper.chmod(0o700)
        with patch.dict(os.environ, {'RUSTC_WRAPPER': str(wrapper)}):
            with cargo_targets.environment(self.old, self.cache, None) as env:
                subprocess.run([env['RUSTC_WRAPPER'], 'rustc', '--version'],
                    **env.process_options(), check=True, capture_output=True)
                inherited = dict(env)
        inherited.pop(resource_slots.INHERITED_COMPILER_CAPACITY, None)
        inherited.pop(resource_slots.INHERITED_COMPILER_CAPACITY_ROOT, None)
        with patch.dict(os.environ, inherited, clear=True):
            # A new context owns a new descriptor; it must not chain the prior
            # Battlement wrapper or retain its already-closed descriptor.
            with cargo_targets.environment(self.new, self.cache, None) as env:
                self.assertEqual(env['BATTLEMENT_CARGO_OUTER_WRAPPER'], str(wrapper))
                subprocess.run([env['RUSTC_WRAPPER'], 'rustc', '--version'],
                    **env.process_options(), check=True, capture_output=True)
        self.assertGreaterEqual(len(marker.read_text().splitlines()), 2)

    def test_explicit_stable_compiler_override_disables_remapping(self):
        with patch.dict(os.environ, {'RUSTC_BOOTSTRAP': '-1'}):
            with cargo_targets.environment(self.old, self.cache, None) as env:
                self.assertNotIn(cargo_targets.OWN_WRAPPER, env)
                source = self.old / 'src/lib.rs'
                source.write_text('#![feature(negative_impls)]\n')
                result = subprocess.run(['rustc', '--crate-type', 'lib', str(source),
                    '--out-dir', str(self.root)], **env.process_options(), capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('stable release channel', result.stderr)

    def test_compiler_wrapper_leaves_external_sources_and_queries_unchanged(self):
        source = self.root / 'outside.rs'
        source.write_text('#![feature(negative_impls)]\nfn main() {}\n')
        with cargo_targets.environment(self.old, self.cache, None) as env:
            env.pop('RUSTC_BOOTSTRAP', None)
            direct = subprocess.run(['rustc', '--version'], env=env,
                capture_output=True, text=True, check=True)
            wrapped = subprocess.run([env['RUSTC_WRAPPER'], 'rustc', '--version'],
                **env.process_options(), capture_output=True, text=True, check=True)
            self.assertEqual(wrapped.stdout, direct.stdout)
            external = subprocess.run([env['RUSTC_WRAPPER'], 'rustc',
                '--crate-name', 'outside', str(source), '--out-dir', str(self.root)],
                **env.process_options(), capture_output=True, text=True)
        self.assertNotEqual(external.returncode, 0)
        self.assertIn('stable release channel', external.stderr)

    def test_child_retains_target_when_controller_exits_and_pruner_skips_it(self):
        ready, release = self.root / 'orphan-ready', self.root / 'orphan-release'
        grandchild = "import sys,time;from pathlib import Path;Path(sys.argv[1]).write_text('ready')\nwhile not Path(sys.argv[2]).exists(): time.sleep(.01)"
        child_program = f"import sys;sys.path.insert(0,{str(SCRIPTS)!r});import reactant_asset_validation;reactant_asset_validation.run([sys.executable,'-c',{grandchild!r},*sys.argv[1:]])"
        program = f"""import os,subprocess,sys,time
from pathlib import Path
sys.path.insert(0,sys.argv[1])
import cargo_targets
root,cache,ready,release=map(Path,sys.argv[2:])
with cargo_targets.environment(root,cache,None) as env:
    Path(env['CARGO_TARGET_DIR']).joinpath('pinned').write_text('live child')
    child = subprocess.Popen([sys.executable,'-c',{child_program!r},str(ready),str(release)],
        **env.process_options(), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    while not ready.exists(): time.sleep(.01)
    child.kill()
    child.wait()
    os._exit(0)
"""
        environment = os.environ | {'BATTLEMENT_RESOURCE_SLOTS': str(self.resources)}
        environment.pop(resource_slots.INHERITED_COMPILER_CAPACITY, None)
        environment.pop(resource_slots.INHERITED_COMPILER_CAPACITY_ROOT, None)
        try:
            subprocess.run([sys.executable, '-c', program, str(SCRIPTS), str(self.old),
                str(self.cache), str(ready), str(release)], env=environment, check=True, timeout=15)
            target = next((self.cache / 'cargo-targets/shared').glob('pool-*'))
            self.assertFalse(cargo_targets.idle(target, self.cache))
            cache = CiCache(self.old, self.cache, {})
            with resource_slots.compiler_maintenance_lease():
                cache.prune(target_bytes=0, high_water_bytes=1)
            self.assertEqual((target / 'pinned').read_text(), 'live child')
            with cargo_targets.environment(self.new, self.cache, None) as env:
                self.assertNotEqual(Path(env['CARGO_TARGET_DIR']), target)
        finally:
            release.touch()
        deadline = time.monotonic() + 15
        while not cargo_targets.idle(target, self.cache):
            self.assertLess(time.monotonic(), deadline, 'child did not release inherited descriptor')
            time.sleep(.01)

    def test_exception_cleanup_preserves_a_live_child_target(self):
        child = None
        try:
            with self.assertRaises(RuntimeError):
                with cargo_targets.environment(self.old, self.cache, None) as env:
                    target = Path(env['CARGO_TARGET_DIR'])
                    child = subprocess.Popen([sys.executable, '-c', 'import time;time.sleep(30)'],
                        **env.process_options())
                    raise RuntimeError('controller canceled')
            self.assertIsNone(child.poll())
            self.assertFalse(cargo_targets.idle(target, self.cache))
            with cargo_targets.environment(self.new, self.cache, None) as env:
                self.assertNotEqual(Path(env['CARGO_TARGET_DIR']), target)
        finally:
            if child is not None:
                child.terminate()
                child.wait(timeout=15)
        self.assertTrue(cargo_targets.idle(target, self.cache))

    def test_concurrent_processes_never_share_a_mutable_target(self):
        program = '''import json,sys,time
from pathlib import Path
sys.path.insert(0,sys.argv[1])
import cargo_targets
root,cache,ready,release=map(Path,sys.argv[2:])
with cargo_targets.environment(root,cache,None) as env:
    ready.write_text(env['CARGO_TARGET_DIR'])
    while not release.exists(): time.sleep(.01)
'''
        children = []
        def start(index):
            ready = self.root / f'ready-{index}'
            release = self.root / f'release-{index}'
            environment = os.environ | {'BATTLEMENT_RESOURCE_SLOTS': str(self.resources)}
            environment.pop(resource_slots.INHERITED_COMPILER_CAPACITY, None)
            environment.pop(resource_slots.INHERITED_COMPILER_CAPACITY_ROOT, None)
            child = subprocess.Popen([sys.executable, '-c', program, str(SCRIPTS),
                str(self.old if index == 0 else self.new), str(self.cache), str(ready), str(release)],
                env=environment, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            children.append(child)
            return ready, release
        def wait_ready(path):
            deadline = time.monotonic() + 15
            while not path.exists():
                self.assertLess(time.monotonic(), deadline, 'target lease did not become available')
                time.sleep(.01)
            return path.read_text()
        try:
            ready_a, release_a = start(0)
            target_a = wait_ready(ready_a)
            ready_b, release_b = start(1)
            target_b = wait_ready(ready_b)
            self.assertNotEqual(target_a, target_b)
            ready_c, release_c = start(2)
            # Both slots are held until their explicit release, so a third writer cannot enter.
            time.sleep(.15)
            self.assertFalse(ready_c.exists())
            release_a.touch()
            self.assertEqual(children[0].wait(timeout=15), 0)
            target_c = wait_ready(ready_c)
            self.assertEqual(target_c, target_a)
            self.assertNotEqual(target_c, target_b)
            release_b.touch()
            release_c.touch()
            for child in children:
                stdout, stderr = child.communicate(timeout=15)
                self.assertEqual(child.returncode, 0, stdout + stderr)
        finally:
            for child in children:
                if child.poll() is None:
                    child.terminate()
                child.communicate(timeout=15)

    def test_waiting_for_a_target_does_not_reserve_compiler_capacity(self):
        identity = hashlib.sha256(b'Cargo.toml').hexdigest()[:16]
        program = '''import sys
from pathlib import Path
sys.path.insert(0,sys.argv[1])
import cargo_targets
with cargo_targets.environment(Path(sys.argv[2]),Path(sys.argv[3]),None):
    raise RuntimeError("held targets unexpectedly became available")
'''
        environment = os.environ | {'BATTLEMENT_RESOURCE_SLOTS': str(self.resources)}
        environment.pop(resource_slots.INHERITED_COMPILER_CAPACITY, None)
        environment.pop(resource_slots.INHERITED_COMPILER_CAPACITY_ROOT, None)
        with resource_slots.SlotLease(self.cache / 'locks', f'cargo-{identity}', 2, units=2):
            child = subprocess.Popen([sys.executable, '-c', program, str(SCRIPTS),
                str(self.old), str(self.cache)], env=environment,
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            try:
                deadline = time.monotonic() + 15
                while resource_slots.active_admission_ticket(child.pid, self.cache / 'locks') is None:
                    self.assertLess(time.monotonic(), deadline, 'writer did not wait for its target')
                    time.sleep(.01)
                probe = "import resource_slots\nwith resource_slots.compiler_maintenance_lease(): print('available')"
                result = subprocess.run([sys.executable, '-c',
                    f"import sys;sys.path.insert(0,{str(SCRIPTS)!r});" + probe],
                    env=environment, check=True, capture_output=True, text=True, timeout=5)
                self.assertIn('available', result.stdout)
            finally:
                child.terminate()
                child.wait(timeout=15)


if __name__ == '__main__':
    # Real compiler fixtures share the host budget; isolated lock directories
    # below exercise ownership without consuming additional host admission.
    with resource_slots.compiler_capacity_lease():
        unittest.main()
