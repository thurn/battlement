"""Lower only compiler and batch-editor child trees, leaving controllers interactive."""

from __future__ import annotations

import os
import subprocess
from typing import Any


BUILD_NICE = 10


def prepare(command: list[str], options: dict[str, Any]) -> list[str]:
    """Apply build priority without changing the caller or compounding inherited niceness."""
    if os.name == "nt":
        options["creationflags"] = options.get("creationflags", 0) | subprocess.BELOW_NORMAL_PRIORITY_CLASS
        return command
    increment = max(0, BUILD_NICE - os.getpriority(os.PRIO_PROCESS, 0))
    return ["nice", "-n", str(increment), *command]


def run(command: list[str], **options: Any) -> subprocess.CompletedProcess[Any]:
    """Run a heavy child with subprocess.run semantics at reduced scheduling priority."""
    return subprocess.run(prepare(command, options), **options)
