# Battlement standard setup implementation plan

Status: implementation companion to `docs/standard-setup-technical-design.md`

This plan implements the complete schema-1 standard-setup contract. The
technical design remains normative. If this plan and the technical design
disagree, the technical design wins except for the explicit clarifications
recorded below.

## Decisions and starting point

The repository already contains the reusable Rust protocol, native adapter,
Unity client, Addressables integration, macOS native-plugin installation, and
private visual-capture machinery. Standard setup changes how a game is
described, authored, packaged, launched, and tested; it does not introduce a
second gameplay protocol or replace the advanced bring-your-own-Unity-project
workflow.

The following decisions were resolved while preparing this plan:

- Unity 6000.5.8f1 is authoritative. The older version in the technical
  design's example `doctor` output is stale and must be corrected.
- `cargo battlement init [path]` requires `--battlement <checkout>`. Once initialized,
  Cargo metadata and the generated path dependencies identify the selected
  checkout for every other command.
- Shell, content, authoring, and run artifacts are project-local. They live
  below the game's `target/battlement/` directory rather than a shared user cache.
- The generic shell consumes a private, deterministic, versioned
  `startup-config.json`. It contains only normalized runtime configuration and
  never includes source paths, Cargo instructions, signing identities, or
  scenario definitions.
- UnityIngameDebugConsole's pinned source and MIT license are vendored into the
  Battlement-owned Unity project so shell builds do not depend on another network
  fetch.
- A dedicated minimal external-game fixture proves the platform contract before
  the real samples migrate.
- Basic, Tic-Tac-Toe, and Chess migrate in separate reviewable commits.
- Advanced-project `cargo battlement plugin` commands remain. The repository-local
  `sample` commands are removed after all samples use manifest discovery.

Tasks are sized as reviewable worktree commits. Every task names its
prerequisites and black-box acceptance. Visual evidence is required only when
it demonstrates player- or Editor-visible behavior.

## Public interfaces and artifact layout

The CLI gains global `--manifest-path` selection and the standard commands
`init`, `doctor`, `generate`, `build`, `run`, `author`, and `scenario run`.
Their options and behavior follow the technical design, with the explicit
`init --battlement <checkout>` clarification above.

The Rust address API gains a constant `AssetAddress::from_static` constructor
and the five handwritten `battlement::standard` constants. Owned and static
addresses remain indistinguishable in equality, hashing, display, and wire
serialization. Standard setup adds no native ABI entry points and no public C#
gameplay API.

Project-local artifacts use these roots:

```text
target/battlement/
├── cache/
│   ├── shell/<fingerprint>/
│   └── content/<fingerprint>/
├── author/
├── runs/<run-id>/
└── <Game Name>.app
```

App assembly installs `startup-config.json` and game Addressables below the
Unity StreamingAssets `battlement/` directory. Native rules retain the existing
`Contents/PlugIns/libbattlement_rules.dylib` location. Install paths are private
constants shared by the CLI assembly code and shell validation fixtures rather
than new author-facing configuration.

Development-only scenario controls live behind assembly and build-profile
boundaries. Release builds must omit their assemblies and assets entirely;
runtime flags alone are not sufficient.

## Dependency overview

Implementation proceeds in five waves. Work within a wave may run in parallel
only when its stated prerequisites are complete.

| Wave | Tasks | Result |
| --- | --- | --- |
| 1 | 01-05 | External game contract, checkout resolution, generation, and scaffolding |
| 2 | 06-13 | Standard shell, content pack, app assembly, and first build |
| 3 | 14-17 | Process diagnostics and disposable Unity authoring |
| 4 | 18-20 | Development-only scenario execution and screenshot comparison |
| 5 | 21A-22 | Sample migrations, legacy retirement, and complete validation |

## Testing conventions used by every task

Rust tests prefer public CLI execution and observable filesystem artifacts over
direct calls into orchestration internals. Unity tests use Edit Mode for
configuration, catalog, assembly-boundary, and input behavior where practical.
Packaged-player checks are reserved for the integration gates that need to
prove application layout, native loading, signing, rendered output, or release
stripping.

Every task runs its focused tests and normal `./scripts/ci.py`. Tasks that add
or change packaged artifacts also run the relevant `./scripts/ci.py --full`
gate. Failed builds must not replace a valid app or cache entry, and failed
scenarios must not replace golden images.

## Wave 1: game contract and generation

### Task 01 - Parse and validate `battlement.toml`

**Prerequisites:** none.

Implement upward manifest discovery, explicit-manifest handling, strict
schema-1 deserialization, documented defaults, identifier and enum validation,
cross-field constraints, normalized-name collision detection, and canonical
path confinement. Keep parsing, validation, and resolved paths distinct so
commands consume one checked game model rather than repeat validation.

**Black-box acceptance:** table-driven tests cover every manifest section,
unknown tables and fields, invalid enums and identifiers, missing required
values, conditional frame-pacing rules, symlink escape, parent traversal,
control characters, and case-insensitive path collisions. Relative paths
resolve from the selected manifest rather than the invoking directory.

### Task 02 - Resolve Cargo rules and the selected Battlement checkout

**Prerequisites:** Task 01.

Use Cargo metadata to locate the selected package, its `battlement_rules` `cdylib`,
crate root, configured features, and its `battlement` and `battlement-native` path
dependencies. Require both dependencies to resolve to one checkout and match
that checkout's package versions. Add `doctor` with manifest, generated source,
Cargo, checkout, Unity 6000.5.8f1, signing, and project-local cache diagnostics.

**Black-box acceptance:** games copied outside the Battlement repository resolve
their selected checkout. Mixed checkouts, version mismatches, absent packages,
wrong crate types, missing Unity installations, and unavailable signing
identities fail with the responsible path or package. `doctor` prints effective
manifest values and every disposable artifact location.

### Task 03 - Add static and standard Rust addresses

**Prerequisites:** none.

Change `AssetAddress` storage so `from_static` can construct real constants
without weakening the existing owned constructor. Add handwritten constants in
`battlement::standard` for the empty scene, default font, white lit material,
white unlit material, and white texture under reserved `battlement/` keys.

**Black-box acceptance:** public API tests prove const construction, type
safety, owned/static equality and hashing, display, conversion, and identical
MessagePack serialization. Dynamic callers retain the owned constructor.

### Task 04 - Generate checked-in typed address bindings

**Prerequisites:** Tasks 01-03.

Implement `generate` with deterministic ID-to-constant normalization, sorting,
schema and generator metadata, the binding-only fingerprint, and atomic source
replacement beside the rules crate root. Report added, removed, renamed, and
retyped constants. Build, run, author, and scenario entry points use the same
comparison logic to reject stale generated source without rewriting it.

**Black-box acceptance:** golden tests prove byte-identical cross-directory
output, stable ordering, reserved or normalized-name collision rejection,
binding-only fingerprint behavior, stale detection, and safe replacement.
After regeneration, removing or retyping an address causes an ordinary Rust
compiler error at an unchanged caller.

### Task 05 - Scaffold external games with `init`

**Prerequisites:** Tasks 01-04.

Implement non-overwriting `init [path] --battlement <checkout>` output: strict
manifest, rules crate and `battlement_rules` target, checkout-relative path
dependencies, declared generated module, initial generated bindings, starter
content, starter scenario and golden, and ignore rules for `target/battlement/`.
Add a small checked-in standard-game fixture that later tests copy outside the
Battlement repository.

**Black-box acceptance:** initialization refuses every existing conflicting
path without leaving partial output. A newly initialized game passes `doctor`
and a no-diff `generate` without manual editing. The dedicated fixture resolves
its checkout after being copied to a temporary external directory.

## Wave 2: shell, content, and app construction

### Task 06 - Define standard startup configuration and bootstrap

**Prerequisites:** Tasks 01 and 03.

Generate deterministic JSON containing the normalized schema, content format,
profile, catalog checksum and ID/kind table, display settings, and diagnostics
settings. Add strict private C# loading. Extend the standard bootstrap to load
and validate the standard and optional game catalogs before creating the Rust
engine, then report ready only after the initial snapshot has produced one
rendered frame.

**Black-box acceptance:** Unity tests cover valid configuration, unknown or
missing fields, unsupported versions, malformed JSON, checksum and type
mismatches, duplicate IDs, absent catalogs, startup-stage diagnostics, and zero
rules calls before catalog validation succeeds.

### Task 07 - Create standard assets and shell profiles

**Prerequisites:** Tasks 03 and 06.

Create the production bootstrap scene and the five standard Addressables roots.
Add focused Editor build entry points for development and release shells, fixed
configuration/content/native slots, separate scenario-capability assembly
boundaries, and build-report inspection. Neither profile contains game rules or
game content.

**Black-box acceptance:** Unity 6000.5.8f1 builds both profiles. Catalog
inspection finds exactly the reserved standard keys, and application inspection
finds the expected bootstrap and empty install slots. Release has no dependency
on the development-only assembly even before scenario controls are added.

### Task 08 - Build and cache standard shells

**Prerequisites:** Tasks 02 and 07.

Fingerprint the Unity version, package lock, profile, requested architectures,
and all relevant Battlement Unity source bytes. Build through the standard-shell
Editor entry point, use an exclusive per-entry lock, publish only a verified
completed entry, and discard incomplete or unreadable entries. Print whether
the shell was reused or rebuilt.

**Black-box acceptance:** controlled-input tests prove every specified
invalidation and exclude Git history and unrelated bytes. A packaged integration
test records that the first request invokes Unity, the second identical request
does not, corrupt entries rebuild, and a concurrent writer receives an
actionable retry error.

### Task 09 - Validate and snapshot game content

**Prerequisites:** Tasks 01 and 02.

Validate declared roots, required `.meta` companions, forbidden executable and
project content, missing dependencies visible without import, authoring locks,
and source immutability. Copy the content root into a private build snapshot and
fingerprint content and `.meta` bytes, declared roots, and the relevant Battlement
Unity environment.

**Black-box acceptance:** tests reject C#, assembly definitions, plugins,
Editor scripts, packages, project settings, custom shaders, missing metadata,
path escape, and case collisions with the responsible path. Changes during
snapshotting fail rather than producing a mixed pack. `.meta`-only edits
invalidate content; Rust and application metadata edits do not.

### Task 10 - Build and cache game Addressables

**Prerequisites:** Tasks 07-09.

Assemble the private content-build project from the selected checkout and the
immutable content snapshot. Configure only manifest-declared roots as public
Addressables, validate their imported broad Unity types and dependency closure,
and atomically publish one catalog and bundle set with a checksum and completion
marker.

**Black-box acceptance:** Unity tests prove transitive scene and prefab
dependencies are packaged without public aliases, public keys equal declared
IDs, broad-kind mismatches and missing scripts fail, source content remains
byte-identical, and a matching completed content fingerprint avoids Unity.

### Task 11 - Build rules and compile app metadata

**Prerequisites:** Tasks 02, 04, and 06.

Reuse the native-plugin build and architecture verification path for the
selected package, features, profile, and requested macOS architectures. Compile
the checked manifest into normalized `startup-config.json`, including the
verified game-catalog checksum and ID/kind table. Keep rules, content, shell,
metadata, and signing decisions as independent build-plan inputs.

**Black-box acceptance:** planning tests prove the technical design's
invalidation matrix: Rust changes rebuild rules only, content or `.meta` changes
rebuild content only, relevant Battlement Unity changes rebuild shell and content,
and display, diagnostics, application metadata, or signing changes only rerun
assembly where applicable.

### Task 12 - Assemble, patch, and sign applications

**Prerequisites:** Tasks 08, 10, and 11.

Copy the cached shell into staging, install the rules library, startup
configuration, game catalog, and bundles, and patch supported macOS metadata.
Remove the shell signature, sign embedded code in dependency order, sign the
outer app with ad hoc or the configured Developer ID and permitted release
entitlements, verify with `codesign`, and atomically replace the published app.

**Black-box acceptance:** tests cover missing architectures, invalid or absent
Keychain identities, unsupported entitlements, sandbox/outbound-network
constraints, hardened-runtime selection, nested signing order, metadata-only
changes, and failures at every staging step. Every failure preserves the
previous valid app and cached shell.

### Task 13 - Expose `cargo battlement build` and prove the first vertical slice

**Prerequisites:** Tasks 04 and 08-12.

Orchestrate generated-source validation, shell/content/rules reuse, assembly,
status reporting, and `--release` behind the public `build` command. Copy the
dedicated standard-game fixture outside the repository and build it only
through public CLI behavior.

**Black-box acceptance:** the first build invokes each missing producer, an
identical build reuses every cache, and isolated Rust, content, `.meta`, Battlement
Unity, display metadata, and signing edits trigger only their documented
stages. The resulting app uses its packaged dylib and content without the
Editor, repository root, or dynamic-library environment overrides.

**Visual evidence:** retain one development-player screenshot showing the
fixture's initial rendered state.

## Wave 3: running, diagnostics, and authoring

### Task 14 - Run players and preserve diagnostics

**Prerequisites:** Task 13.

Implement `run` by launching the app's internal executable directly. Allocate
a unique private run directory, route Unity's player log and native stdout and
stderr to preserved files, tail Unity and stderr with source labels, record the
exit status and matching macOS crash-report path when present, and clean up only
processes started by the command.

**Black-box acceptance:** successful exit, startup failure, Rust panic, forced
crash, timeout, and interruption retain all available artifacts and report the
failed stage or status. Cleanup never targets an unrelated process, and a
player that never becomes ready still leaves useful logs.

### Task 15 - Vendor and integrate in-game diagnostics

**Prerequisites:** Tasks 07 and 14.

Vendor UnityIngameDebugConsole v1.8.9 at the approved commit with its MIT
license. Configure it over Unity's ordinary log stream and add the compact
current/rolling-average FPS and connection panel. Apply the manifest toggle,
suppress Battlement keyboard actions while the console has focus, and use UI hit
testing to prevent pointer actions from reaching the game world.

**Black-box acceptance:** Unity tests verify enablement, toggle-key mapping,
FPS sampling, connection transitions, keyboard focus suppression, and pointer
blocking in development and release. Native stderr never appears in the viewer.

**Visual evidence:** capture development and release players with the console,
FPS values, and connection state visible.

### Task 16 - Create the disposable authoring workspace

**Prerequisites:** Tasks 07, 09, and 10.

Implement `author` by assembling `target/battlement/author/` from Battlement-owned
packages, settings, renderer, bootstrap, and standard assets. Mount game content
at one fixed `Assets` location without a writable copy or copy-back step,
validate Unity's resolved paths, enforce one authoring Editor per game, and open
the first declared scene for a new workspace.

**Black-box acceptance:** scene, imported asset, and `.meta` edits persist
directly in game content. Packages, ProjectSettings, Library, logs, user
settings, and incidental Editor changes remain workspace-owned. Content builds
fail clearly while the authoring lock is held, and an unsupported mount fails
rather than falling back to synchronization.

**Visual evidence:** show the Project window ownership boundary and one saved
game-content edit visible in the source repository.

### Task 17 - Keep authoring Play Mode current

**Prerequisites:** Tasks 13 and 16.

Add an authoring-project pre-Play hook that checks generated bindings, builds
current development rules and content, installs them in the workspace, and
selects the standard bootstrap as the Play Mode start scene. Surface refresh
failures in Unity and cancel Play Mode rather than using previously installed
artifacts.

**Black-box acceptance:** Unity tests cover current-input installation,
Rust-build failure, stale generation, content-build failure, bootstrap
selection, and repeated Play Mode entry. A changed Rust rule is visible on the
next successful Play Mode entry without rebuilding the shell.

**Visual evidence:** record a short authoring-to-Play-Mode run using current
Rust rules and edited content.

## Wave 4: scenario automation

### Task 18 - Add development-only scenario controls

**Prerequisites:** Tasks 07 and 14.

Adapt the existing private file-command, simulated Input System device,
readiness, frame-waiting, and framebuffer-capture mechanisms to the standard
development shell. Use a CLI-owned private directory, accept only one command
at a time, acknowledge completed output publication, and open no network
listener.

**Black-box acceptance:** a packaged development player handles readiness,
click, key press, frame wait, time wait, capture, and shutdown through normal
Unity input and rendering. Release build-report and assembly inspection proves
the handler, simulated input, and capture implementation and assets are absent.

### Task 19 - Parse and execute TOML scenarios

**Prerequisites:** Tasks 01, 14, and 18.

Implement strict scenario parsing, filename/name agreement, window overrides,
ordered step validation, normalized clicks, key presses, frame/time waits,
fresh-player isolation, one overall timeout, `--all` filename ordering, and
continue-after-failure behavior. Reuse the run artifact and owned-process
lifecycle rather than creating a second launcher.

**Black-box acceptance:** tests cover every step and invalid field, input key,
coordinate, and timeout. Packaged tests prove pointer and keyboard events cross
Unity's Input System and Battlement action path into Rust and change rendered
state. Success, command failure, crash, timeout, and interruption stop the
owned player and preserve evidence.

**Visual evidence:** record one multi-step pointer-and-keyboard scenario from
ready state through its rendered result.

### Task 20 - Compare, retain, and accept screenshots

**Prerequisites:** Task 19.

Decode captured and golden PNGs to RGBA, require equal dimensions, calculate
normalized mean absolute channel error, create a difference image on mismatch,
and apply step, manifest, then exact-default tolerance precedence. Stage all
`--accept` replacements and publish them only after the whole scenario passes.

**Black-box acceptance:** tests cover exact and tolerant matches, dimension
mismatch, capture-only steps, illegal capture-only tolerance, missing goldens,
multiple screenshots, a late non-capture failure, and atomic replacement. PNG
metadata and compression do not affect comparison, and FFmpeg is not required.

**Visual evidence:** retain one representative golden, mismatching actual, and
difference image alongside the machine-readable score.

## Wave 5: migration and completion

### Task 21A - Migrate Basic

**Prerequisites:** Tasks 15-20.

Remove Basic's Unity project, adopt `battlement.toml`, use the standard empty scene
and default font, retain only required game-authored materials below its content
root, replace address strings with generated typed constants, and add its
standard scenario and golden.

**Black-box acceptance:** Basic retains its fake-client tests and builds, runs,
and passes its scenario after being copied outside Battlement. It owns no packages,
project settings, generated Unity infrastructure, or C#.

**Visual evidence:** capture the migrated Basic app's representative result.

### Task 21B - Migrate Tic-Tac-Toe

**Prerequisites:** Tasks 15-20.

Remove Tic-Tac-Toe's Unity project, move its PNG files and `.meta` companions
into the content root, declare only directly addressed textures, replace
address literals with generated typed constants, and add its standard scenario
and golden.

**Black-box acceptance:** Tic-Tac-Toe retains its fake-client tests and builds,
runs, and passes its scenario after being copied outside Battlement. Catalog
inspection exposes only declared roots while packaging their dependencies.

**Visual evidence:** capture a completed migrated Tic-Tac-Toe interaction.

### Task 21C - Migrate Chess

**Prerequisites:** Tasks 21A and 21B.

Retain Chess's authored main scene and required KayKit models, textures,
materials, and `.meta` files as game content. Remove packages, project and input
settings, URP infrastructure, bootstrap assets, and other Battlement-owned files.
Repeat the exact dependency audit, declare the main scene root, replace public
address literals with generated constants, and add the opening-move scenario
and reviewed golden.

**Black-box acceptance:** Chess retains its Rust tests, builds from an external
copy, opens and saves through `author`, enters Play Mode with current rules, and
passes its packaged opening-move scenario. Catalog inspection proves the scene's
dependency closure is present without additional public aliases.

**Visual evidence:** capture the authored scene and packaged opening move and
compare them with the reviewed visual-equivalence golden.

### Task 22 - Retire the legacy workflow and close the validation matrix

**Prerequisites:** Tasks 21A-21C.

Remove repository-specific `sample` CLI and build code and update CI to use the
migrated manifest-driven games. Keep fast contract checks in normal
`./scripts/ci.py`; add shell/content cache, external packaged-game, scenario,
migration, and release-inspection gates to `./scripts/ci.py --full`. Update
supporting documentation, correct the stale Unity-version example, and verify
the vendored license inventory.

**Black-box acceptance:** normal and full CI pass from a clean checkout. The
full suite covers every automated acceptance criterion in the technical design,
including external discovery, incremental rebuilds, source-safe authoring,
logs after failure, simulated input, screenshot acceptance, signing, and
release stripping. Complete the technical design's manual QA once against the
final combined implementation and retain only its prescribed evidence.

## Completion criteria

- Every public command works from a game repository located outside Battlement.
- Rust-only iteration can rebuild and reassemble without invoking Unity.
- Content authoring preserves only intentional game-source changes.
- Startup validates installed configuration and catalogs before running rules.
- Development and release diagnostics work, while scenario capabilities are
  absent from release.
- Scenarios use fresh players, normal Unity input, deterministic capture, and
  atomic golden acceptance.
- Basic, Tic-Tac-Toe, and Chess own no redundant Unity infrastructure.
- Advanced Unity projects retain the native-plugin workflow.
- Normal and full CI, sample visual evidence, and the complete manual QA pass.
