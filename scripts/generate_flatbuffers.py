#!/usr/bin/env python3
"""Regenerate the pinned Battlement FlatBuffers bindings."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import shutil
import subprocess
import tempfile
import urllib.request
import zipfile
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
TOOLCHAIN = ROOT / "schemas" / "flatbuffers-toolchain.json"
SCHEMAS = ROOT / "schemas" / "flatbuffers"
RUST_OUTPUT = ROOT / "crates" / "battlement-flatbuffers" / "src" / "generated"
CSHARP_OUTPUT = (
    ROOT
    / "Packages"
    / "com.battlement.client"
    / "Runtime"
    / "FlatBuffers"
    / "Generated"
)
CSHARP_RUNTIME = (
    ROOT / "Packages" / "com.battlement.client" / "Runtime" / "FlatBuffers" / "Google"
)
FIXTURE_SCHEMA = (
    ROOT
    / "crates"
    / "battlement-native"
    / "tests"
    / "fixtures"
    / "exported-engine"
    / "schema"
    / "fixture_response.fbs"
)
FIXTURE_RUST_OUTPUT = FIXTURE_SCHEMA.parent.parent / "src" / "fixture_response_generated.rs"
FIXTURE_CSHARP_OUTPUT = (
    ROOT
    / "Packages"
    / "com.battlement.client"
    / "Tests"
    / "Editor"
    / "CustomFixtures"
    / "fixture_response_generated.cs"
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--flatc", type=Path, help="use an already installed pinned compiler")
    parser.add_argument("--check", action="store_true", help="fail when generated files differ")
    args = parser.parse_args()
    specification = json.loads(TOOLCHAIN.read_text())
    verify_csharp_runtime(specification)
    with tempfile.TemporaryDirectory(prefix="battlement-flatbuffers-") as temporary:
        temporary_path = Path(temporary)
        compiler = args.flatc or acquire_compiler(specification, temporary_path)
        verify_compiler(compiler, specification["version"])
        rust = temporary_path / "rust"
        csharp = temporary_path / "csharp"
        rust.mkdir()
        csharp.mkdir()
        schemas = sorted(SCHEMAS.glob("*.fbs"))
        options = specification["generation_options"]
        subprocess.run(
            [compiler, "--rust", *options, "-o", rust, *schemas], check=True
        )
        subprocess.run(
            [compiler, "--csharp", *options, "-o", csharp, *schemas], check=True
        )
        fixture_rust = temporary_path / "fixture-rust"
        fixture_csharp = temporary_path / "fixture-csharp"
        fixture_rust.mkdir()
        fixture_csharp.mkdir()
        fixture_arguments = ["-I", str(SCHEMAS), str(FIXTURE_SCHEMA)]
        subprocess.run(
            [compiler, "--rust", *options, "-o", fixture_rust, *fixture_arguments],
            check=True,
        )
        subprocess.run(
            [compiler, "--csharp", *options, "-o", fixture_csharp, *fixture_arguments],
            check=True,
        )
        configure_csharp_sources(csharp)
        configure_csharp_sources(fixture_csharp)
        normalize_generated_sources(rust)
        normalize_generated_sources(csharp)
        normalize_generated_sources(fixture_rust)
        normalize_generated_sources(fixture_csharp)
        fixture_rust_source = fixture_rust / "fixture_response_generated.rs"
        configure_fixture_rust_source(fixture_rust_source)
        format_rust_sources(rust)
        format_rust_sources(fixture_rust)
        fixture_csharp_source = fixture_csharp / "fixture_response_generated.cs"
        if args.check:
            return int(
                not (
                    same_tree(rust, RUST_OUTPUT)
                    and same_tree(csharp, CSHARP_OUTPUT)
                    and same_file(fixture_rust_source, FIXTURE_RUST_OUTPUT)
                    and same_file(fixture_csharp_source, FIXTURE_CSHARP_OUTPUT)
                )
            )
        replace_tree(rust, RUST_OUTPUT)
        replace_tree(csharp, CSHARP_OUTPUT)
        replace_file(fixture_rust_source, FIXTURE_RUST_OUTPUT)
        replace_file(fixture_csharp_source, FIXTURE_CSHARP_OUTPUT)
    return 0


def acquire_compiler(specification: dict[str, object], temporary: Path) -> Path:
    key = f"{platform.system().lower()}-{platform.machine().lower()}"
    archive = specification["compiler_archives"].get(key)
    if archive is None:
        raise SystemExit(f"no pinned flatc archive for {key}; pass --flatc")
    destination = temporary / "flatc.zip"
    urllib.request.urlretrieve(archive["url"], destination)
    actual = hashlib.sha256(destination.read_bytes()).hexdigest()
    if actual != archive["sha256"]:
        raise SystemExit(f"flatc archive digest mismatch: {actual}")
    with zipfile.ZipFile(destination) as source:
        source.extractall(temporary / "compiler")
    compiler = temporary / "compiler" / ("flatc.exe" if os.name == "nt" else "flatc")
    compiler.chmod(0o755)
    return compiler


def verify_compiler(compiler: Path, expected: str) -> None:
    version = subprocess.run(
        [compiler, "--version"], check=True, capture_output=True, text=True
    ).stdout.strip()
    if version != f"flatc version {expected}":
        raise SystemExit(f"flatc version mismatch: {version}")


def verify_csharp_runtime(specification: dict[str, object]) -> None:
    for name, expected in specification["csharp_runtime_artifact_sha256"].items():
        actual = hashlib.sha256((CSHARP_RUNTIME / name).read_bytes()).hexdigest()
        if actual != expected:
            raise SystemExit(f"C# FlatBuffers runtime digest mismatch for {name}: {actual}")


def configure_csharp_sources(directory: Path) -> None:
    """Compile generated accessors against the vendored unsafe Span runtime."""
    definitions = "#define ENABLE_SPAN_T\n#define UNSAFE_BYTEBUFFER\n"
    for source in directory.glob("*.cs"):
        source.write_text(definitions + source.read_text())


def configure_fixture_rust_source(source: Path) -> None:
    """Resolve the fixture schema's core imports through the transport crate."""
    contents = source.read_text()
    contents = contents.replace(
        "use crate::", "use battlement_flatbuffers::schema_generated::"
    )
    marker = "pub mod flat_buffers {\n\n"
    generated_alias = """pub mod generated {
  pub use battlement_flatbuffers::schema_generated::common_generated::battlement::flat_buffers::generated::*;
  pub use battlement_flatbuffers::schema_generated::client_message_generated::battlement::flat_buffers::generated::*;
  pub use battlement_flatbuffers::schema_generated::response_generated::battlement::flat_buffers::generated::*;
}

"""
    if contents.count(marker) != 1:
        raise SystemExit("fixture Rust namespace shape changed")
    contents = contents.replace(marker, marker + generated_alias)
    source.write_text(contents)


def normalize_generated_sources(directory: Path) -> None:
    """Remove compiler-emitted trailing whitespace deterministically."""
    for source in directory.iterdir():
        if source.is_file():
            lines = (line.rstrip() for line in source.read_text().rstrip().splitlines())
            source.write_text("\n".join(lines) + "\n")


def format_rust_sources(directory: Path) -> None:
    """Apply the workspace Rust format to generated sources before comparison."""
    sources = sorted(str(path) for path in directory.glob("*.rs"))
    if sources:
        subprocess.run(
            [
                "rustfmt",
                "--edition",
                "2024",
                "--config-path",
                str(ROOT / "rustfmt.toml"),
                *sources,
            ],
            check=True,
        )


def same_tree(actual: Path, expected: Path) -> bool:
    actual_files = {path.relative_to(actual) for path in actual.rglob("*") if path.is_file()}
    expected_files = {
        path.relative_to(expected)
        for path in expected.rglob("*")
        if path.is_file() and path.suffix not in {".asmdef", ".meta"}
    }
    return actual_files == expected_files and all(
        (actual / path).read_bytes() == (expected / path).read_bytes()
        for path in actual_files
    )


def same_file(actual: Path, expected: Path) -> bool:
    return expected.is_file() and actual.read_bytes() == expected.read_bytes()


def replace_tree(source: Path, destination: Path) -> None:
    destination.mkdir(parents=True, exist_ok=True)
    for path in destination.iterdir():
        if path.suffix not in {".asmdef", ".meta"}:
            if path.is_dir():
                shutil.rmtree(path)
            else:
                path.unlink()
    for path in source.iterdir():
        shutil.move(path, destination / path.name)


def replace_file(source: Path, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.move(source, destination)


if __name__ == "__main__":
    raise SystemExit(main())
