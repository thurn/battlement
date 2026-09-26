#!/usr/bin/env python3

"""Exercise native sample selection without invoking build tools."""

import json
from pathlib import Path
import sys
import tempfile
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

import native_validation_selection as selection


SAMPLES = ["basic", "reactant", "chess"]


assert selection.select(ROOT, ["README.md"], SAMPLES) == []
assert selection.select(ROOT, ["samples/basic/rules/src/lib.rs"], SAMPLES) == ["basic"]
assert selection.select(ROOT, ["Packages/com.battlement.client/Runtime/Host.cs"], SAMPLES) == SAMPLES
assert selection.select(ROOT, ["contracts/native-abi.json"], SAMPLES) == SAMPLES
assert selection.select(ROOT, ["ProjectSettings/GraphicsSettings.asset"], SAMPLES) == SAMPLES
assert selection.select(ROOT, ["samples/basic/ProjectSettings/ProjectSettings.asset"], SAMPLES) == SAMPLES
assert selection.select(ROOT, ["samples/chess/Assets/Shaders/LegalSquare.shader"], SAMPLES) == SAMPLES
assert selection.select(ROOT, ["samples/ui/Assets/Resources/BattlementTextSettings.asset"], SAMPLES) == SAMPLES

audio_samples = ["basic", "chess", "reactant", "tictactoe", "ui", "new-game"]
for source in (
    "Packages/com.battlement.client/Runtime/Host/BattlementAudioSources.cs",
    "Packages/com.battlement.client/Runtime/Host/BattlementAudioInstance.cs",
):
    for path in (source, source + ".meta"):
        assert selection.select(ROOT, [path], audio_samples) == ["chess", "reactant", "new-game"]
        assert selection.select(ROOT, [path, "samples/ui/rules/src/app.rs"], audio_samples) == [
            "chess", "reactant", "ui", "new-game",
        ]
        assert selection.select(
            ROOT, [path, "Packages/com.battlement.client/Runtime/Host/BattlementRunner.cs"],
            audio_samples,
        ) == audio_samples

editor_test = "Packages/com.battlement.client/Tests/Editor/Host/BattlementCommandAdmissionTests.cs"
for path in (editor_test, editor_test + ".meta", editor_test.replace("/", "\\")):
    assert selection.select(ROOT, [path], SAMPLES) == []
    assert selection.select(ROOT, [path, "samples/basic/rules/src/lib.rs"], SAMPLES) == ["basic"]
    assert selection.select(
        ROOT, [path, "Packages/com.battlement.client/Runtime/Host.cs"], SAMPLES,
    ) == SAMPLES

for path in (
    "Packages/com.battlement.client/Tests/Editor/BattlementUiActionTests.cs",
    "Packages/com.battlement.client/Tests/Editor/Reactant/NewTests.cs",
    "Packages/com.battlement.client/Tests/Editor/Host/Removed/Subdirectory/Test.cs",
):
    assert selection.select(ROOT, [path], SAMPLES) == []

for path in (
    "Packages/com.battlement.client/Tests/Editor/Host/Battlement.HostEditorTests.asmdef",
    "Packages/com.battlement.client/Tests/Editor/Host/Battlement.HostEditorTests.asmdef.meta",
    "Packages/com.battlement.client/Tests/Editor/Host/Test.prefab",
    "Packages/com.battlement.client/Tests/Editor/Host/NewExtension.unknown",
    "Packages/com.battlement.client/Tests/Editor/CustomFixtures/FixtureFlatBufferResponseSchema.cs",
    "Packages/com.battlement.client/Tests/Runtime/NewTests.cs",
    "Packages/com.battlement.client/Editor/BattlementDittoBuild.cs",
    "Packages/com.battlement.client/Tests/Editor/../../Runtime/Host.cs",
    "scripts/native_validation_selection.py",
):
    assert selection.select(ROOT, [editor_test, path], SAMPLES) == SAMPLES, path

with tempfile.TemporaryDirectory() as temporary:
    repository = Path(temporary)
    directory = repository / "Packages/com.battlement.client/Tests/Editor"
    directory.mkdir(parents=True)
    assembly = directory / "Tests.asmdef"
    # The nearest declaration owns the source, regardless of its test-like name.
    for definition in (
        {}, [], {"includePlatforms": ["Editor"]},
        {"includePlatforms": [], "optionalUnityReferences": ["TestAssemblies"]},
        {"includePlatforms": ["Editor", "Android"], "optionalUnityReferences": ["TestAssemblies"]},
        {"includePlatforms": ["Editor"], "optionalUnityReferences": None},
    ):
        assembly.write_text(json.dumps(definition))
        assert selection.select(repository, [editor_test], SAMPLES) == SAMPLES
    assembly.write_text("broken JSON")
    assert selection.select(repository, [editor_test], SAMPLES) == SAMPLES
    assembly.write_text(json.dumps({
        "includePlatforms": ["Editor"], "optionalUnityReferences": ["TestAssemblies"],
    }))
    assert selection.select(repository, [editor_test], SAMPLES) == []
    nested = directory / "Host"
    nested.mkdir()
    reference = nested / "Shared.asmref"
    reference.write_text('{"reference":"Runtime"}')
    assert selection.select(repository, [editor_test], SAMPLES) == SAMPLES
    reference.unlink()
    boundary = nested / "Runtime.asmdef"
    boundary.write_text('{"name":"Runtime"}')
    assert selection.select(repository, [editor_test], SAMPLES) == SAMPLES
    boundary.unlink()
    (directory / "Ambiguous.asmdef").write_text(assembly.read_text())
    assert selection.select(repository, [editor_test], SAMPLES) == SAMPLES
    assembly.unlink()
    (directory / "Ambiguous.asmdef").unlink()
    assert selection.select(repository, [editor_test], SAMPLES) == SAMPLES


dependencies = {
    "basic": {"battlement"},
    "reactant": {"battlement", "reactant", "reactant-core", "reactant-ui"},
    "chess": {"battlement", "reactant", "reactant-core", "reactant-ui"},
}
with patch.object(selection, "sample_crates", side_effect=lambda _root, sample: dependencies[sample]):
    assert selection.select(ROOT, ["crates/reactant-core/src/lib.rs"], SAMPLES) == [
        "reactant",
        "chess",
    ]

print("native validation selection tests passed")
