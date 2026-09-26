#!/usr/bin/env python3
"""Generate or check declared imported materials, addresses, and a sample shell."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import tempfile

import ci
from resource_slots import unity_editor_lease
from unity_transaction import UnityProjectTransaction


def prepare(project: Path, check: bool) -> None:
    project = project.resolve()
    if not (project / "project-assets.json").is_file():
        raise RuntimeError(f"No project-assets.json in {project}")
    bootstrap = not (project / "Assets/AddressableAssetsData/AddressableAssetSettings.asset").exists()
    with tempfile.TemporaryDirectory(prefix="battlement-project-assets-") as temporary:
        output = Path(temporary) / "output"
        output.mkdir()
        log = Path(temporary) / "unity.log"
        transaction = UnityProjectTransaction(project, "project-assets")
        with unity_editor_lease():
            transaction.prepare()
            try:
                # Addressables runs its schema migrations on editor initialization.
                # A newly created settings asset must cross that boundary once.
                for attempt in range(2 if bootstrap else 1):
                    result = transaction.run([
                        str(ci.unity_editor()), "-batchmode", "-nographics", "-quit",
                        "--burst-disable-compilation", "-projectPath", str(project),
                        "-executeMethod", "Battlement.Editor.BattlementProjectAssets.Generate",
                        "-logFile", str(log),
                    ], cwd=project, env=os.environ | {"BATTLEMENT_PROJECT_ASSETS_OUTPUT": str(output)})
                    shutil.copy2(log, transaction.directory / f"project-assets-{attempt}.log")
                    if result.returncode:
                        break
            finally:
                transaction.restore()
            shutil.copy2(log, transaction.directory / "project-assets.log")
        if result.returncode or not (output / "inventory.json").is_file():
            raise RuntimeError(f"Project asset preparation failed; inspect {transaction.directory}")
        inventory = json.loads((output / "inventory.json").read_text())
        stale = []
        for relative in inventory["files"]:
            path = Path(relative)
            if path.is_absolute() or ".." in path.parts:
                raise RuntimeError(f"Invalid generated path: {relative}")
            source, destination = output / path, project / path
            if destination.is_file() and source.read_bytes() == destination.read_bytes():
                continue
            stale.append(relative)
            if not check:
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(source, destination)
        shutil.copy2(output / "inventory.json", transaction.directory / "project-assets-inventory.json")
        if check and stale:
            raise RuntimeError("Declared project assets are stale: " + ", ".join(stale))
        print(f"Project assets {'checked' if check else 'generated'}: {len(inventory['addresses'])} addresses")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["generate", "check"])
    parser.add_argument("--project", type=Path, required=True)
    args = parser.parse_args()
    prepare(args.project, args.mode == "check")
