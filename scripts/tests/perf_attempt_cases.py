"""Retained audit and native-event cases for CI attempt attribution."""

import json
from pathlib import Path

import perf_attempts
import perf_ci
from perf_model import Span


def verify(root: Path) -> None:
    """Exercise duplicate runs, source selection, overlap and native provenance."""
    fixture = json.loads(Path(__file__).with_name('fixtures').joinpath('ci-attempt-audit.json').read_text())
    for cohort in fixture.values():
        report = perf_attempts.build(cohort['spans'] + cohort['spans'])
        assert report['run_count'] == cohort['expected_count']
        assert report['status_counts'] == cohort['expected_statuses']
        assert all(attempt['owner']['thread_id'] == cohort['thread_id']
                   for group in report['sources'] for attempt in group['attempts'])
        assert sum(group['run_count'] for group in report['sources']) == report['run_count']
        assert report['run_union_ms'] <= report['inclusive_run_ms']
    _native_details(root)


def _native_details(root: Path) -> None:
    records = [
        {'event': 'ci.run_started', 'timestamp': 10, 'run_id': 'one',
         'codex_thread_id': 'owner', 'head_oid': 'local', 'staged_tree_oid': 'tree', 'branch': 'task'},
        {'event': 'ci.step_started', 'timestamp': 11, 'run_id': 'one',
         'span_id': 'fail-first', 'parent_span_id': 'one', 'name': 'Build and test'},
        {'event': 'ci.step_finished', 'timestamp': 19, 'run_id': 'one',
         'span_id': 'fail-first', 'parent_span_id': 'one', 'name': 'Build and test', 'outcome': 'failed'},
        {'event': 'ci.step_started', 'timestamp': 12, 'run_id': 'one',
         'span_id': 'fail-second', 'parent_span_id': 'one', 'name': 'Parallel check'},
        {'event': 'ci.step_finished', 'timestamp': 20, 'run_id': 'one',
         'span_id': 'fail-second', 'parent_span_id': 'one', 'name': 'Parallel check', 'outcome': 'failed'},
        {'event': 'ci.run_finished', 'timestamp': 30, 'run_id': 'one', 'outcome': 'failed'},
    ]
    ci, warnings = perf_ci.parse_ci_records(records, root/'ci.jsonl')
    assert not warnings
    native = [
        {'event': 'operation.started', 'timestamp': 11, 'operation_id': 'fail-first',
         'context': {'root_operation_id': 'one', 'task_id': 'owner'},
         'metadata': {'command': ['cargo', 'test']}, 'parent_operation_id': 'one'},
        {'event': 'resource.queued', 'timestamp': 11, 'resource': 'compiler'},
        {'event': 'resource.acquired', 'timestamp': 13, 'resource': 'compiler'},
        {'event': 'process.started', 'timestamp': 13, 'process': {'pid': 1}, 'executable': 'cargo'},
        {'event': 'process.finished', 'timestamp': 19, 'process': {'pid': 1},
         'exit_code': 1, 'cpu_user_ms': 100, 'cpu_system_ms': 20,
         'cpu_scope': 'process_and_waited_descendants'},
        {'event': 'resource.released', 'timestamp': 19, 'resource': 'compiler'},
        {'event': 'operation.finished', 'timestamp': 19, 'outcome': 'failed'},
    ]
    operation, warnings = perf_ci.parse_operation_records(native, root/'operation.jsonl',
                                                          {'fail-first': 'ci-step:fail-first'})
    assert not warnings
    assert not any(span.category == 'operation' for span in operation)
    assert all(span.parent_id == 'ci-step:fail-first' for span in operation)
    second = Span('ci-run:two', None, 'owner', 'ci', 'ci', 'CI', 20, 40, 'passed',
                  attributes={'head_oid': 'tested', 'staged_tree_oid': 'tree', 'full': True})
    cert = Span('tg-buildset:cert', None, 'owner', 'tollgate', 'ci', 'Certification', 20, 40, 'passed',
                attributes={'tested_oid': 'tested', 'source_oid': 'source', 'candidate_id': 'candidate'})
    compiler = Span('process:compiler', 'ci-run:one', None, 'operation', 'process', 'rustc', 15, 18,
                    'passed', attributes={'cpu_user_ms': 5, 'cpu_system_ms': 1, 'source_path': str(root/'compiler.jsonl')})
    unknown_run = Span('ci-run:unknown', None, None, 'ci', 'ci', 'CI', 40, 50, 'incomplete')
    spans = [span.as_dict() for span in [*ci, *operation, second, cert, compiler, unknown_run]]
    report = perf_attempts.build(spans + spans)
    assert report['run_count'] == 3
    assert report['inclusive_run_ms'] == 50_000 and report['run_union_ms'] == 40_000
    group = report['sources'][0]
    assert group['run_count'] == 2
    one, two = group['attempts']
    assert one['first_failed_boundary']['name'] == 'Build and test'
    assert one['role'] == 'local_validation' and two['role'] == 'certification'
    assert two['role_evidence']['source_oid'] == 'source'
    assert two['source_attempt'] == 2 and two['repeated_source_attempt']
    assert one['phases']['resource_wait']['interval_union_ms'] == 2000
    assert one['phases']['cargo_mixed_unknown']['inclusive_cpu_ms'] == 120
    assert one['phases']['compiler_execution']['inclusive_cpu_ms'] == 6
    assert one['measured_phase_union_ms'] == 8000
    assert one['unattributed_run_ms'] == 12000
    assert 'test_binary_execution' in one['unmeasured_phases']
    assert one['evidence']['source_link'].endswith('/ci.jsonl')
    assert report['sources'][1]['attempts'][0]['role'] == 'unknown'
    assert perf_attempts.build(spans, source_oid='source')['run_count'] == 1
    assert perf_attempts.build(spans, source_oid='local', source_tree_oid='tree')['run_count'] == 2
    assert perf_attempts.build(spans, source_oid='local', source_tree_oid='different')['run_count'] == 0
    assert perf_attempts.build(spans, source_oid='absent')['run_count'] == 0
    binary = Span('process:test', 'ci-run:one', None, 'operation', 'process', 'observed-test', 24, 26,
                  'passed', attributes={'phase': 'test_binary_execution'})
    setup = Span('process:setup', 'ci-run:one', None, 'operation', 'process', 'dotnet', 27, 29,
                 'passed', attributes={'command': ['dotnet', 'tool', 'restore']})
    explicit = perf_attempts.build(spans + [binary.as_dict(), setup.as_dict()])
    phases = explicit['sources'][0]['attempts'][0]['phases']
    assert phases['test_binary_execution']['interval_union_ms'] == 2000
    assert phases['setup_execution']['interval_union_ms'] == 2000
    assert phases['test_binary_execution']['inclusive_cpu_ms'] is None
    ambiguous = cert.as_dict(); ambiguous['id'] = 'tg-buildset:other'
    assert perf_attempts.build(spans + [ambiguous])['sources'][0]['attempts'][1]['role'] == 'unknown'
    compiler.attributes['cpu_user_ms'] = 'invalid'
    bad = perf_attempts.build([span.as_dict() for span in [*ci, compiler]])
    assert bad['sources'][0]['attempts'][0]['phases']['compiler_execution']['inclusive_cpu_ms'] is None
