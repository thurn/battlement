#!/usr/bin/env python3
"""Check exact buildset collection, failed-run retention, and CI error propagation."""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import uuid

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import ci
import ditto_evidence
from tollgate_evidence import Export


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        subprocess.run(['git', 'init', '-q', str(self.root)], check=True)
        subprocess.run(['git', '-c', 'user.name=Test', '-c', 'user.email=test@example.com',
                        'commit', '--allow-empty', '-qm', 'fixture'], cwd=self.root, check=True)
        self.oid = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=self.root, text=True).strip()
        self.environment = {
            'TOLLGATE_BUILDSET_ID': str(uuid.uuid4()),
            'TOLLGATE_ITEM_ID': str(uuid.uuid4()),
            'TOLLGATE_VALIDATION_GENERATION_ID': str(uuid.uuid4()),
            'TOLLGATE_TESTED_OID': self.oid,
        }
        context = patch.dict(os.environ, self.environment)
        context.start()
        self.addCleanup(context.stop)
        self.export = Export.begin(self.root)

    def invocation(self, status='passed'):
        invocation_id = str(uuid.uuid4())
        root = self.root / 'invocations' / invocation_id
        identity = ditto_evidence.begin(root, invocation_id, self.root, 'gate')
        sample = root / 'basic'
        sample.mkdir()
        (sample / 'result.json').write_text(json.dumps({'status': status}))
        if status != 'passed':
            (sample / 'run.tar.gz').write_bytes(b'failed execution archive')
        (root / 'gate.json').write_text(json.dumps({
            'invocation_id': invocation_id, 'artifact_root': str(root), 'status': status,
            'failures': [] if status == 'passed' else ['basic'],
            'expected_samples': ['basic'], 'samples': [{'sample': 'basic'}],
        }))
        return ditto_evidence.finish(root, identity, status), invocation_id

    def test_only_current_invocation_is_retained(self):
        stale, stale_id = self.invocation('failed')
        manifest, invocation = self.invocation()
        (manifest.parent / 'unlisted-stale.txt').write_text('do not collect')
        bundle = self.export.publish(manifest, invocation)
        with tarfile.open(bundle) as archive:
            names = set(archive.getnames())
            self.assertEqual(names, {
                'receipt.json', 'invocation/evidence.json', 'invocation/invocation.json',
                'invocation/gate.json', 'invocation/basic/result.json',
            })
            receipt = json.load(archive.extractfile('receipt.json'))
            self.assertEqual(receipt['buildset_id'], self.environment['TOLLGATE_BUILDSET_ID'])
            self.assertEqual(receipt['invocation_id'], invocation)
            self.assertNotEqual(receipt['invocation_id'], stale_id)
        self.assertTrue(stale.exists())
        with self.assertRaises(ValueError):
            self.export.publish(manifest, invocation)

    def test_failed_and_canceled_runs_retain_their_archive(self):
        for status in ('failed', 'canceled'):
            with self.subTest(status=status):
                with patch.dict(os.environ, {'TOLLGATE_BUILDSET_ID': str(uuid.uuid4())}):
                    export = Export.begin(self.root)
                    manifest, invocation = self.invocation(status)
                    with tarfile.open(export.publish(manifest, invocation)) as archive:
                        self.assertEqual(json.load(archive.extractfile('receipt.json'))['status'], status)
                        self.assertEqual(archive.extractfile('invocation/basic/run.tar.gz').read(),
                                         b'failed execution archive')

    def test_wrong_execution_and_forged_outer_context_are_rejected(self):
        for field in ('buildset_id', 'candidate_id', 'validation_generation_id', 'tested_oid'):
            with self.subTest(field=field):
                manifest, invocation = self.invocation()
                document = json.loads(manifest.read_text())
                document[field] = str(uuid.uuid4())
                manifest.write_text(json.dumps(document))
                with self.assertRaises(ValueError):
                    self.export.publish(manifest, invocation)
        with patch.dict(os.environ, {'TOLLGATE_BUILDSET_ID': str(uuid.uuid4())}):
            manifest, invocation = self.invocation()
        with self.assertRaisesRegex(ValueError, 'buildset_id'):
            self.export.publish(manifest, invocation)

    def test_corrupt_and_missing_members_do_not_publish(self):
        for missing in (False, True):
            manifest, invocation = self.invocation()
            member = manifest.parent / 'basic/result.json'
            if missing:
                member.unlink()
            else:
                member.write_text('corruption')
            with self.assertRaises(ValueError):
                self.export.publish(manifest, invocation)
        self.assertEqual(list(self.export.root.iterdir()), [])

    def test_reused_buildset_and_wrong_checkout_are_rejected(self):
        with self.assertRaises(FileExistsError):
            Export.begin(self.root)
        with patch.dict(os.environ, {'TOLLGATE_TESTED_OID': '0' * 40}):
            with self.assertRaisesRegex(ValueError, 'checkout'):
                Export.begin(self.root)

    def test_ci_retains_failure_without_masking_it(self):
        manifest, invocation = self.invocation('failed')
        failure = subprocess.CalledProcessError(1, ['ditto'])
        with patch.object(ci, 'REPOSITORY_ROOT', self.root), \
             patch.object(ci, 'run_step', side_effect=failure), \
             patch.object(ci.ci_steps, 'record_event'), \
             patch.dict(os.environ, {'DITTO_CI_ARTIFACT_ROOT': str(manifest.parent)}):
            with self.assertRaises(subprocess.CalledProcessError) as raised:
                ci.run_ditto_validation(0, invocation_id=invocation, evidence_export=self.export)
            self.assertIs(raised.exception, failure)
        self.assertTrue((self.export.root / 'evidence.tar.gz').exists())

    def test_export_failure_fails_successful_ci(self):
        with patch.object(ci, 'REPOSITORY_ROOT', self.root), \
             patch.object(ci, 'run_step'), patch.object(ci.ci_steps, 'record_event'), \
             patch.dict(os.environ, {'DITTO_CI_ARTIFACT_ROOT': str(self.root / 'missing')}):
            with self.assertRaises(FileNotFoundError):
                ci.run_ditto_validation(0, invocation_id='missing', evidence_export=self.export)

    def test_empty_native_selection_publishes_verified_gate(self):
        invocation = str(uuid.uuid4())
        with patch.object(ci, 'REPOSITORY_ROOT', self.root), \
             patch.object(ci.ci_steps, 'record_event') as event:
            ci.publish_empty_ditto_validation(
                self.export, invocation, ['scripts/perf_report.py']
            )
        bundle = self.export.root / 'evidence.tar.gz'
        with tarfile.open(bundle) as archive:
            gate = json.load(archive.extractfile('invocation/gate.json'))
            self.assertEqual(gate['expected_samples'], [])
            self.assertEqual(gate['selected_paths'], ['scripts/perf_report.py'])
            self.assertEqual(gate['status'], 'passed')
        event.assert_called_once()


if __name__ == '__main__':
    unittest.main()
