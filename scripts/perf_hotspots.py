#!/usr/bin/env python3

"""Aggregate repeated performance operations into actionable hotspots."""

from __future__ import annotations

from collections import defaultdict
import math
from typing import Any


def ci_step_hotspots(
    spans: list[dict[str, Any]],
    top: int,
) -> list[dict[str, Any]]:
    """Aggregate top-level ci.py steps by their stable display name."""
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for span in spans:
        if not _is_top_level_ci_step(span):
            continue
        grouped[span["name"]].append(span)
    hotspots = []
    for name, group in grouped.items():
        durations = sorted(int(span["duration_ms"]) for span in group)
        run_ids = {
            span.get("attributes", {}).get("run_id")
            for span in group
            if span.get("attributes", {}).get("run_id")
        }
        hotspots.append(
            {
                "name": name,
                "occurrence_count": len(group),
                "run_count": len(run_ids),
                "failed_count": sum(
                    span.get("status") != "passed" for span in group
                ),
                "total_duration_ms": sum(durations),
                "average_duration_ms": round(sum(durations) / len(durations)),
                "p50_duration_ms": _nearest_rank(durations, 0.50),
                "p95_duration_ms": _nearest_rank(durations, 0.95),
                "max_duration_ms": durations[-1],
                "span_ids": [span["id"] for span in group],
            }
        )
    return sorted(
        hotspots,
        key=lambda item: (
            -item["total_duration_ms"],
            -item["max_duration_ms"],
            item["name"],
        ),
    )[:top]


def _is_top_level_ci_step(span: dict[str, Any]) -> bool:
    return (
        span.get("source") == "ci"
        and str(span.get("id", "")).startswith("ci-step:")
        and str(span.get("parent_id", "")).startswith("ci-run:")
    )


def _nearest_rank(values: list[int], percentile: float) -> int:
    return values[max(0, math.ceil(percentile * len(values)) - 1)]
