#!/usr/bin/env python3

"""Data structures and interval calculations for performance reports."""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any


@dataclass
class Span:
    """One normalized interval of agent, tool, CI, or waiting activity."""

    id: str
    parent_id: str | None
    session_id: str | None
    source: str
    category: str
    name: str
    started_at: float
    finished_at: float
    status: str
    association: str | None = None
    attributes: dict[str, Any] = field(default_factory=dict)
    content: dict[str, Any] = field(default_factory=dict)
    container: bool = False

    @property
    def duration_ms(self) -> int:
        """Return the non-negative span duration."""
        return max(0, round((self.finished_at - self.started_at) * 1000))

    def as_dict(self, exclusive_duration_ms: int | None = None) -> dict[str, Any]:
        """Return a JSON-compatible representation."""
        result = {
            "id": self.id,
            "parent_id": self.parent_id,
            "session_id": self.session_id,
            "source": self.source,
            "category": self.category,
            "name": self.name,
            "started_at": format_timestamp(self.started_at),
            "finished_at": format_timestamp(self.finished_at),
            "duration_ms": self.duration_ms,
            "status": self.status,
            "association": self.association,
            "attributes": self.attributes,
            "content": self.content,
            "container": self.container,
        }
        if exclusive_duration_ms is not None:
            result["exclusive_duration_ms"] = exclusive_duration_ms
        return result


@dataclass
class SessionTrace:
    """One top-level Codex task and all activity associated with it."""

    thread_id: str
    title: str
    rollout_path: Path
    repository_url: str
    parent_thread_id: str | None = None
    agent_name: str | None = None
    agent_path: str | None = None
    first_user_at: float | None = None
    completed_at: float | None = None
    latest_event_at: float | None = None
    completed: bool = False
    spans: list[Span] = field(default_factory=list)
    transcript: list[dict[str, Any]] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)
    candidate_ids: set[str] = field(default_factory=set)
    token_usage: dict[str, Any] = field(default_factory=dict)
    time_to_first_token_ms: list[int] = field(default_factory=list)

    def as_metadata(self) -> dict[str, Any]:
        """Return identifying session metadata without trace contents."""
        return {
            "thread_id": self.thread_id,
            "title": self.title,
            "rollout_path": str(self.rollout_path),
            "repository_url": self.repository_url,
            "completed": self.completed,
            "first_user_at": format_optional_timestamp(self.first_user_at),
            "completed_at": format_optional_timestamp(self.completed_at),
            "agent_name": self.agent_name,
            "agent_path": self.agent_path,
        }


@dataclass(frozen=True)
class Thresholds:
    """Deterministic duration thresholds used by workflow findings."""

    slow_tool_ms: int
    slow_subagent_ms: int
    slow_ci_step_ms: int
    long_wait_ms: int


def parse_timestamp(value: Any) -> float | None:
    """Parse Codex ISO timestamps and Tollgate date tuples."""
    if isinstance(value, (int, float)):
        return float(value) / 1000 if value > 10_000_000_000 else float(value)
    if isinstance(value, str):
        try:
            return datetime.fromisoformat(value.replace("Z", "+00:00")).timestamp()
        except ValueError:
            return None
    if not isinstance(value, list) or len(value) < 6:
        return None
    try:
        year, ordinal, hour, minute, second, nanoseconds = value[:6]
        base = datetime(int(year), 1, 1, tzinfo=timezone.utc)
        return (
            base
            + timedelta(
                days=int(ordinal) - 1,
                hours=int(hour),
                minutes=int(minute),
                seconds=int(second),
                microseconds=int(nanoseconds) / 1000,
            )
        ).timestamp()
    except (TypeError, ValueError, OverflowError):
        return None


def format_timestamp(value: float) -> str:
    """Format a Unix timestamp as UTC ISO-8601."""
    return datetime.fromtimestamp(value, timezone.utc).isoformat(
        timespec="milliseconds"
    ).replace("+00:00", "Z")


def format_optional_timestamp(value: float | None) -> str | None:
    """Format a timestamp when present."""
    return None if value is None else format_timestamp(value)


def interval_union_ms(intervals: list[tuple[float, float]]) -> int:
    """Return the union length of possibly overlapping intervals."""
    return round(sum(end - start for start, end in _merged_intervals(intervals)) * 1000)


def interval_difference_ms(
    included: list[tuple[float, float]],
    excluded: list[tuple[float, float]],
) -> int:
    """Return the union of included intervals after removing excluded time."""
    remaining = 0.0
    excluded_union = _merged_intervals(excluded)
    for start, end in _merged_intervals(included):
        cursor = start
        for cut_start, cut_end in excluded_union:
            if cut_end <= cursor:
                continue
            if cut_start >= end:
                break
            remaining += max(0, min(cut_start, end) - cursor)
            cursor = max(cursor, cut_end)
            if cursor >= end:
                break
        remaining += max(0, end - cursor)
    return round(remaining * 1000)


def _merged_intervals(
    intervals: list[tuple[float, float]],
) -> list[tuple[float, float]]:
    valid = sorted((start, end) for start, end in intervals if end > start)
    merged: list[tuple[float, float]] = []
    for start, end in valid:
        if not merged or start > merged[-1][1]:
            merged.append((start, end))
            continue
        merged[-1] = (merged[-1][0], max(merged[-1][1], end))
    return merged


def exclusive_durations(spans: list[Span]) -> dict[str, int]:
    """Subtract the union of direct child intervals from every span."""
    children: dict[str, list[Span]] = {}
    for span in spans:
        if span.parent_id is not None:
            children.setdefault(span.parent_id, []).append(span)
    return {
        span.id: max(
            0,
            span.duration_ms
            - interval_union_ms(
                [
                    (
                        max(span.started_at, child.started_at),
                        min(span.finished_at, child.finished_at),
                    )
                    for child in children.get(span.id, [])
                ]
            ),
        )
        for span in spans
    }
