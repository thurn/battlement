"""Exercise diagnostics admission while half the machine capacity is occupied."""

from pathlib import Path
import subprocess
import sys
import tempfile
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import ci
import resource_slots
from ci_cache import CiCache


def verify_diagnostics_capacity() -> None:
    subprocess.run([sys.executable, __file__], check=True, timeout=5)


def exercise() -> None:
    with tempfile.TemporaryDirectory(prefix="diagnostics-capacity.") as temporary:
        root = Path(temporary)
        slots = root / "slots"
        cache = CiCache(root, root / "cache", {}, enabled=False)
        selection = ci.unity_test_selection.select(ci.REPOSITORY_ROOT, ["scripts/ci.py"])

        def step(name, *args, **options):
            if name == "Check .NET diagnostics":
                options["function"]()

        with (
            patch.object(resource_slots, "GLOBAL_RESOURCE_ROOT", slots),
            patch.object(cache, "maintain", return_value=False),
            patch.object(ci, "run_step", side_effect=step),
            patch.object(ci, "generate_unity_project_files"),
            patch.object(ci, "unity_analyzer_environment", return_value={}),
            patch.object(ci.process_priority, "run"),
            resource_slots.SlotLease(slots, "machine-heavy", 6, 3),
        ):
            ci.run_csharp_preflight([], selection, cache)


if __name__ == "__main__":
    exercise()
