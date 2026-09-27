"""Attribute unique CI attempts and measured phases without inventing coverage."""

from collections import Counter, defaultdict
import math
from pathlib import Path

from perf_model import interval_difference_ms, interval_union_ms, parse_timestamp


def build(spans: list[dict], *, source_oid: str | None = None,
          source_tree_oid: str | None = None) -> dict:
    """Group retained CI runs by staged source and preserve exact evidence links."""
    by_id = {span['id']: span for span in spans}
    runs = sorted((span for span in by_id.values() if span['id'].startswith('ci-run:')),
                  key=lambda span: (span['started_at'], span['id']))
    attempts = [_attempt(run, by_id) for run in runs]
    if source_oid:
        attempts = [attempt for attempt in attempts if
                    source_oid == attempt['role_evidence'].get('source_oid') or
                    (source_tree_oid is not None and source_tree_oid == attempt['staged_tree_oid'])]
        selected = {attempt['run_id'] for attempt in attempts}
        runs = [run for run in runs if run['id'].removeprefix('ci-run:') in selected]
    sources = defaultdict(list)
    for attempt in attempts:
        # Missing identity never establishes equivalence between two runs.
        key = attempt['staged_tree_oid'] or f"unknown:{attempt['run_id']}"
        sources[key].append(attempt)
    groups = []
    for key, group in sources.items():
        for index, attempt in enumerate(group, 1):
            attempt['source_attempt'] = index
            attempt['repeated_source_attempt'] = index > 1
        groups.append({'staged_tree_oid': group[0]['staged_tree_oid'],
                       'run_count': len(group),
                       'status_counts': dict(sorted(Counter(a['status'] for a in group).items())),
                       'role_counts': dict(sorted(Counter(a['role'] for a in group).items())),
                       'attempts': group})
    return {'source_oid_filter': source_oid, 'source_tree_oid_filter': source_tree_oid,
            'run_count': len(attempts),
            'status_counts': dict(sorted(Counter(a['status'] for a in attempts).items())),
            'role_counts': dict(sorted(Counter(a['role'] for a in attempts).items())),
            'inclusive_run_ms': sum(run['duration_ms'] for run in runs),
            'run_union_ms': interval_union_ms([_interval(run) for run in runs]),
            'sources': groups,
            'interpretation': 'Repeated source attempts are observations, not unique deliveries or bugs. '
                              'Staged trees identify indexed inputs; unstaged runtime inputs remain unknown. '
                              'Phase intervals and CPU counters can overlap; do not sum them as a wall-time partition.'}


def _attempt(run: dict, by_id: dict) -> dict:
    children = [span for span in by_id.values() if _belongs(span, run, by_id)]
    top = [span for span in children if span['parent_id'] == run['id']
           and span['id'].startswith('ci-step:')]
    failed = sorted((span for span in top if span['status'] == 'failed'),
                    key=lambda span: (span['finished_at'], span['id']))
    attrs = run['attributes']
    role, role_evidence = _role(run, by_id.values())
    phases = defaultdict(list)
    for span in children:
        phase = _phase(span)
        if phase:
            phases[phase].append(span)
    measured = [span for group in phases.values() for span in group]
    bounds = _interval(run)
    measurements = {name: _measurement(group, bounds) for name, group in sorted(phases.items())}
    return {'run_id': run['id'].removeprefix('ci-run:'),
            'status': run['status'], 'role': role, 'role_evidence': role_evidence,
            'head_oid': attrs.get('head_oid'), 'staged_tree_oid': attrs.get('staged_tree_oid'),
            'full': attrs.get('full'), 'dirty': attrs.get('dirty'),
            'owner': {'thread_id': attrs.get('codex_thread_id') or run.get('session_id'),
                      'association': run.get('association')},
            'worktree_path': attrs.get('worktree_path'),
            'started_at': run['started_at'], 'finished_at': run['finished_at'],
            'duration_ms': run['duration_ms'],
            'cpu_user_ms': attrs.get('cpu_user_ms'), 'cpu_system_ms': attrs.get('cpu_system_ms'),
            'cpu_scope': attrs.get('cpu_scope'),
            'first_failed_boundary': _reference(failed[0]) if failed else None,
            'phases': measurements,
            'unmeasured_phases': [name for name in ('compiler_execution', 'build_execution',
                                                   'setup_execution', 'test_binary_execution',
                                                   'resource_wait') if name not in phases],
            'measured_phase_union_ms': interval_union_ms([_bounded(span, bounds) for span in measured]),
            'unattributed_run_ms': interval_difference_ms([bounds], [_bounded(span, bounds) for span in measured]),
            'evidence': _reference(run)}


def _belongs(span: dict, run: dict, by_id: dict) -> bool:
    parent = span.get('parent_id')
    seen = {span['id']}
    while parent and parent not in seen:
        if parent == run['id']:
            return True
        seen.add(parent)
        parent = by_id.get(parent, {}).get('parent_id')
    root = span['attributes'].get('root_operation_id')
    return root == run['id'].removeprefix('ci-run:') and span['id'] != run['id']


def _role(run: dict, spans) -> tuple[str, dict]:
    attrs = run['attributes']
    matches = []
    for span in spans:
        if not span['id'].startswith('tg-buildset:'):
            continue
        if not attrs.get('head_oid') or attrs['head_oid'] != span['attributes'].get('tested_oid'):
            continue
        start, end = _interval(span)
        first, last = _interval(run)
        if start - 2 <= first and last <= end + 2:
            matches.append(span)
    if len(matches) == 1:
        return 'certification', {'method': 'exact_tested_commit_and_attempt_interval',
                                **_reference(matches[0]),
                                'candidate_id': matches[0]['attributes'].get('candidate_id'),
                                'source_oid': matches[0]['attributes'].get('source_oid')}
    if matches:
        return 'unknown', {'method': 'ambiguous_certification_attempt',
                           'span_ids': [span['id'] for span in matches]}
    if attrs.get('codex_thread_id') and attrs.get('branch'):
        return 'local_validation', {'method': 'recorded_thread_and_branch'}
    return 'unknown', {'method': 'insufficient_native_evidence'}


def _phase(span: dict) -> str | None:
    attrs = span['attributes']
    event = attrs.get('event', '')
    if event == 'resource.acquired' or event == 'ci.cache_wait':
        return 'resource_wait'
    if span['category'] != 'process':
        return None
    executable = Path(span['name']).stem.lower()
    command = attrs.get('command')
    arguments = command[1:] if isinstance(command, list) else []
    if executable in {'rustc', 'clang', 'clang++', 'gcc', 'g++', 'cl'}:
        return 'compiler_execution'
    if executable == 'cargo':
        if arguments and arguments[0] == 'build':
            return 'build_execution'
        return 'cargo_mixed_unknown'
    if executable == 'rustup':
        return 'setup_execution'
    if executable == 'dotnet' and arguments[:2] == ['tool', 'restore']:
        return 'setup_execution'
    # Explicit producer classification can distinguish a directly observed test
    # binary; a Cargo interval or a CI step named "tests" cannot establish it.
    if attrs.get('phase') == 'test_binary_execution':
        return 'test_binary_execution'
    return 'process_execution_unknown'


def _measurement(spans: list[dict], bounds: tuple) -> dict:
    intervals = [_bounded(span, bounds) for span in spans]
    cpu = [span for span in spans if _has_cpu(span)]
    # Nested process counters include their waited descendants. Report each
    # observation separately; the sum is inclusive and is never unique CPU.
    return {'interval_union_ms': interval_union_ms(intervals),
            'inclusive_interval_ms': sum(max(0, round((b-a)*1000)) for a, b in intervals),
            'observation_count': len(spans), 'cpu_measured_count': len(cpu),
            'cpu_interval': 'Full retained observation; window clipping does not apportion CPU.',
            'inclusive_cpu_ms': sum(span['attributes']['cpu_user_ms'] + span['attributes']['cpu_system_ms']
                                    for span in cpu) if cpu else None,
            'observations': [{'evidence': _reference(span),
                              'cpu_user_ms': span['attributes'].get('cpu_user_ms'),
                              'cpu_system_ms': span['attributes'].get('cpu_system_ms'),
                              'cpu_scope': span['attributes'].get('cpu_scope')} for span in spans]}


def _has_cpu(span: dict) -> bool:
    attrs = span['attributes']
    values = [attrs.get('cpu_user_ms'), attrs.get('cpu_system_ms')]
    return all(type(value) in (int, float) and math.isfinite(value) and value >= 0 for value in values)


def _reference(span: dict) -> dict:
    source = span['attributes'].get('source_path')
    return {'span_id': span['id'], 'name': span['name'], 'source_path': source,
            'started_at': span['started_at'], 'finished_at': span['finished_at'],
            'status': span['status'],
            'source_link': Path(source).absolute().as_uri() if source else None}


def _interval(span: dict) -> tuple:
    return parse_timestamp(span['started_at']), parse_timestamp(span['finished_at'])


def _bounded(span: dict, bounds: tuple) -> tuple:
    start, end = _interval(span)
    return max(start, bounds[0]), min(end, bounds[1])


def print_summary(attempts: dict, top: int, duration) -> None:
    """Print bounded source rows; retain all attempt and phase evidence in JSON."""
    print(f"    CI attempts: {attempts['run_count']} unique runs · "
          f"{attempts['status_counts']} · {attempts['role_counts']}")
    print(f"    run union {duration(attempts['run_union_ms'])} · "
          f"inclusive sum {duration(attempts['inclusive_run_ms'])}; overlapping phases are not additive")
    sources = attempts['sources'][-top:]
    for source in sources:
        tree = source['staged_tree_oid'] or 'unknown'
        print(f"    staged source {tree}: {source['run_count']} attempt(s)")
        for attempt in source['attempts'][-top:]:
            failed = attempt['first_failed_boundary']
            boundary = f" · first failed: {failed['name']}" if failed else ''
            print(f"      {attempt['run_id']} · {attempt['role']} · {attempt['status']}{boundary}")
            for phase, measurement in attempt['phases'].items():
                cpu = measurement['inclusive_cpu_ms']
                cpu_text = 'unknown' if cpu is None else duration(cpu)
                print(f"        {phase}: union {duration(measurement['interval_union_ms'])} · "
                      f"inclusive CPU {cpu_text} · "
                      f"{measurement['cpu_measured_count']}/{measurement['observation_count']} measured")
            print(f"        unattributed interval: {duration(attempt['unattributed_run_ms'])}")
            print(f"        unmeasured phases: {', '.join(attempt['unmeasured_phases']) or 'none'}")
    if len(sources) < len(attempts['sources']):
        print(f"    {len(attempts['sources']) - len(sources)} earlier sources retained in JSON")
