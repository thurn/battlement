#!/usr/bin/env python3
"""Verify real child usage, serial aggregation, and unknown parallel attribution."""

from contextlib import redirect_stdout
import io
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import ci_steps
import operation_log
import perf_ci
import perf_hotspots
import perf_log
import perf_report
import process_usage

BUSY = "import time; memory=bytearray(16*1024*1024); end=time.process_time()+.15\nwhile time.process_time()<end: pass"


class UsageTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.logs = self.root / 'logs'
        metadata = patch.object(perf_log, 'git_metadata', return_value={'dirty': False, 'head_oid': 'fixture'})
        metadata.start()
        self.addCleanup(metadata.stop)
        environment = patch.dict(os.environ, {'BATTLEMENT_LOG_ROOT': str(self.logs)})
        environment.start()
        self.addCleanup(environment.stop)
        self.addCleanup(ci_steps.configure, self.root, None)

    def run_child(self, program):
        with operation_log.Operation(self.root, 'child') as operation:
            operation_log.run([sys.executable, '-c', program], cwd=self.root)
        return self.records(operation.path)

    @staticmethod
    def records(path):
        return [json.loads(line) for line in path.read_text().splitlines()]

    @unittest.skipUnless(hasattr(os, 'wait4'), 'wait4 is unavailable')
    def test_waited_grandchild_cpu_and_memory(self):
        program = f'import subprocess,sys;subprocess.run([sys.executable,"-c",{BUSY!r}],check=True)'
        records = self.run_child(program)
        usage = next(item for item in records if item['event'] == 'process.finished')
        self.assertGreaterEqual(usage['cpu_user_ms'] + usage['cpu_system_ms'], 140)
        self.assertGreater(usage['max_rss'], 16 * 1024 * 1024)
        self.assertEqual(usage['cpu_scope'], 'process_and_waited_descendants')
        sleep = self.run_child('import time;time.sleep(.2)')
        sleep = next(item for item in sleep if item['event'] == 'process.finished')
        self.assertLess(sleep['cpu_user_ms'] + sleep['cpu_system_ms'],
                        usage['cpu_user_ms'] + usage['cpu_system_ms'])

    def test_nonzero_exit_is_preserved(self):
        with operation_log.Operation(self.root, 'failure') as operation:
            with self.assertRaises(subprocess.CalledProcessError) as caught:
                operation_log.run([sys.executable, '-c', 'raise SystemExit(7)'], cwd=self.root)
        self.assertEqual(caught.exception.returncode, 7)
        terminal = next(item for item in self.records(operation.path) if item['event'] == 'process.finished')
        self.assertEqual(terminal['exit_code'], 7)
        self.assertIn('cpu_user_ms', terminal)

    @unittest.skipUnless(os.name == 'posix', 'Unix signals required')
    def test_signal_exit_is_preserved(self):
        with self.assertRaises(subprocess.CalledProcessError) as caught:
            self.run_child('import os,signal;os.kill(os.getpid(),signal.SIGTERM)')
        self.assertEqual(caught.exception.returncode, -signal.SIGTERM)

    @unittest.skipUnless(process_usage.resource is not None, 'resource is unavailable')
    def test_trace_aggregates_parallel_children_without_worker_double_counting(self):
        trace = perf_log.CiTrace(self.root, {}, log_root=self.logs)
        ci_steps.configure(self.root, trace)
        def child():
            operation_log.run([sys.executable, '-c', BUSY], cwd=self.root)
        ci_steps.run_step('aggregate', function=lambda: ci_steps.run_parallel_steps(
            [('one', child), ('two', child)]))
        trace.finish('passed', 0)
        records = self.records(trace.path)
        finished = {item['name']: item for item in records if item['event'] == 'ci.step_finished'}
        total = finished['aggregate']
        self.assertGreaterEqual(total['cpu_user_ms'] + total['cpu_system_ms'], 280)
        for name in ('one', 'two'):
            self.assertIsNone(finished[name]['cpu_user_ms'])
            self.assertIsNone(finished[name]['max_rss'])
        run = records[-1]
        self.assertGreaterEqual(run['cpu_user_ms'], total['cpu_user_ms'])
        self.assertGreaterEqual(run['cpu_system_ms'], total['cpu_system_ms'])
        spans, warnings = perf_ci.parse_ci_records(records, trace.path)
        self.assertEqual(warnings, [])
        top = perf_hotspots.ci_step_hotspots([span.as_dict() for span in spans], 10)
        self.assertEqual(len(top), 1)
        self.assertEqual(top[0]['cpu_user_ms'], total['cpu_user_ms'])
        self.assertEqual(top[0]['cpu_measured_count'], 1)
        text = io.StringIO()
        with redirect_stdout(text):
            perf_report._print_ci_hotspots(top)
        self.assertIn('CPU-s', text.getvalue())
        self.assertIn('1/1 measured', text.getvalue())

    def test_peak_rss_is_not_assigned_from_an_earlier_child(self):
        earlier = SimpleNamespace(ru_utime=1, ru_stime=1, ru_maxrss=1000)
        unchanged = SimpleNamespace(ru_utime=2, ru_stime=2, ru_maxrss=1000)
        higher = SimpleNamespace(ru_utime=3, ru_stime=3, ru_maxrss=2000)
        with patch.object(process_usage, 'snapshot', return_value=unchanged):
            self.assertIsNone(process_usage.elapsed(earlier)['max_rss'])
        for platform, expected in [('darwin', 2000), ('linux', 2000 * 1024)]:
            with patch.object(process_usage, 'snapshot', return_value=higher), \
                 patch.object(process_usage.sys, 'platform', platform):
                self.assertEqual(process_usage.elapsed(earlier)['max_rss'], expected)

    def test_unknown_measurements_are_not_zero_or_inherited(self):
        records = [
            {'event': 'ci.run_started', 'timestamp': '2026-09-25T00:00:00Z', 'run_id': 'run'},
            {'event': 'ci.step_started', 'timestamp': '2026-09-25T00:00:01Z', 'span_id': 'step'},
            {'event': 'ci.step_finished', 'timestamp': '2026-09-25T00:00:02Z', 'span_id': 'step',
             'parent_span_id': 'run', 'name': 'step', 'outcome': 'passed'},
            {'event': 'ci.run_finished', 'timestamp': '2026-09-25T00:00:03Z',
             'cpu_user_ms': 500, 'cpu_system_ms': 100},
        ]
        spans, _ = perf_ci.parse_ci_records(records, self.root / 'trace')
        self.assertEqual(spans[0].attributes['cpu_user_ms'], 500)
        self.assertIsNone(spans[1].attributes['cpu_user_ms'])
        unknown = spans[1].as_dict()
        measured = {**unknown, 'id': 'ci-step:known', 'attributes': {
            'cpu_user_ms': 3, 'cpu_system_ms': 2, 'max_rss': 4096}}
        top = perf_hotspots.ci_step_hotspots([unknown, measured], 10)[0]
        self.assertEqual(top['cpu_measured_count'], 1)
        self.assertEqual(top['cpu_user_ms'], 3)
        self.assertEqual(top['occurrence_count'], 2)
        top = perf_hotspots.ci_step_hotspots([unknown], 10)[0]
        self.assertIsNone(top['cpu_user_ms'])
        self.assertEqual(perf_report._cpu(top), 'CPU unknown')
        with patch.object(process_usage, 'resource', None):
            self.assertIsNone(process_usage.snapshot())
            self.assertEqual(process_usage.elapsed(None), process_usage.unknown())


if __name__ == '__main__':
    unittest.main()
