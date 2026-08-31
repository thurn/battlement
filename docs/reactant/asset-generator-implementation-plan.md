# Reactant Asset Generator implementation plan

When a task is complete, append `[DONE]` to its task heading.

Status: implementation companion to
[`asset-generator.md`](asset-generator.md).

This plan implements the approved Reactant asset generator contract without
adding paint features. The technical design is normative. If this plan and the
design disagree, the technical design wins.

## Related information

- [`asset-generator.md`](asset-generator.md) defines the complete authoring,
  discovery, rendering, output, Unity, runtime, failure, and performance
  contract implemented here.
- [`reactant-technical-design.md`](reactant-technical-design.md) defines the
  Reactant session and snapshot boundaries extended by generated assets.
- [`host-facades.md`](host-facades.md) defines the Reactant façade
  prerequisite, including `Image`, private `UiImage` lowering, and host-method
  ordering.
- [`reactant-implementation-plan.md`](reactant-implementation-plan.md) records
  the completed runtime, sample, testing, and evidence conventions on which
  this project builds.
- [`../battlement-ui-technical-design.md`](../battlement-ui-technical-design.md)
  defines the texture, image, style, prepared-asset, and lease behavior reused
  without an asset-generator-specific runtime path.
- [`../ditto-implementation-plan.md`](../ditto-implementation-plan.md) defines
  the sample coverage and release-evidence workflow used by the final gallery.

## Decisions and starting point

The repository already contains Reactant, typed texture addresses, prepared
asset validation, UI asset leases, the Cargo-facing CLI, Unity Addressables
build hooks, and the Reactant sample. It does not contain an asset declaration
macro, shared CSS parser, source discovery cache, browser renderer, generated
texture importer, or runtime generated-asset catalog.

The Reactant host-façade migration is an established prerequisite. Reactant
authoring uses `Image`; Battlement UI documents and commands use `UiImage`.
Supporting values such as `ImageSource`, `Style`, `TextureAddress`, and
`PreparedAsset` retain their existing names and protocol behavior.

Before Task 07 begins, the implementation must pass the core migration suite in
the host-façade contract. Motion-specific integration criteria are not an asset
generator prerequisite. The passing core suite is the prerequisite evidence;
this plan does not infer completion from a document status label.

The following decisions govern implementation:

- The **rules package** is the selected Rust crate that builds the game-rules
  plugin; its `Cargo.toml` is the **rules manifest**, and its native or
  WebAssembly artifact is the **rules plugin**.
- An **authoritative snapshot** is a complete Reactant session state, including
  each later authoritative replacement. A **linked registration** is the const
  inventory record emitted by one declaration. The **runtime sidecar** is the
  generated Resources JSON containing the address set expected by Unity.
- A **manifest-complete root** contains a valid manifest, runtime sidecar, and
  every referenced texture and `.meta` file. A **convention-based sample** is a
  direct `samples/<name>` project with its ordinary `sample.toml`, independent
  of the sample's name.

- Add `battlement-reactant-asset-syntax` as the ordinary library shared by the
  procedural macro and host tooling. It owns the closed token grammar, typed
  request model, canonical encoding, native-support classification, and stable
  diagnostic categories.
- Add `battlement-reactant-asset-macros` as the procedural-macro crate. It
  parses through the shared library and emits only a typed handle static plus
  one registration. It performs no filesystem access or subprocess work.
- Add `battlement-reactant-assets` as the host-only discovery, browser,
  generation, manifest, transaction, and preview library. The existing
  `battlement-cli` remains a thin command adapter.
- Keep `inventory` as the linked registration mechanism. Cross-crate native
  and `wasm32-unknown-unknown` fixtures must pass before public macro expansion
  depends on it. It provides runtime enumeration without filesystem discovery
  or a wire-protocol change, including a startup mismatch for declarations that
  source scanning cannot see. Runtime enumeration always sorts and deduplicates
  registrations. If either target fixture fails, stop and revise the registry
  design instead of adding a target-specific workaround.
- Reserve `battlement-reactant/generated/` for generated
  `PreparedAsset::Texture` values. Reactant adds all linked registrations to
  every authoritative snapshot without changing the core wire protocol.
- Generate a non-Addressable
  `Resources/BattlementReactantAssetCatalog.json` sidecar containing the sorted
  public addresses and authoritative manifest hash. Unity compares it with
  reserved-prefix prepared textures before accepting any authoritative
  snapshot.
- Validate linked-catalog parity at session startup and authoritative snapshot
  replacement. Source discovery, generated imports, and user-owned address
  conflicts still fail before authoring or a player build starts.
- Use real Cargo, Chrome or Chromium, filesystem, and Unity boundaries in
  automated tests. Do not add fixture executables, test-only production
  commands, reflection, friend assemblies, or private implementation snapshots.
- Give every public command an optional aggregate work report. Tests use its
  stable counts to observe Cargo runs, file work, subprocesses, browser
  launches, browser contexts, and writes without exposing or asserting
  implementation structure.
- Automated browser tests validate PNG structure, dimensions, alpha bounds,
  hashes, manifest records, and Unity importability. They do not compare exact
  pixels or maintain browser-specific golden images.
- Visual paint correctness, browser termination, transaction interruption,
  destructive filesystem failures, and the full performance fixture are final
  Manual QA responsibilities.
- Keep exhaustive automated validation available through an asset-specific
  one-off runner. It may take minutes and is required by the relevant tasks and
  final release validation, but it is not called by `ci.py --full`.
- The asset generator may increase the median wall-clock duration of a warm
  `ci.py --full` run by at most 15 seconds. The recurring subset targets 12
  seconds so process-startup and host-load variance do not consume the limit.
  A missing supported stable browser fails the recurring real-browser batch
  rather than skipping it.
- Tasks target 300–500 non-test lines. Split a task that would exceed 500
  lines unless the larger transaction is necessary to leave a usable public
  contract. No source file may grow beyond the repository's size limits.

## Task and testing conventions

Implementation is a mostly linear stack. Each task depends on the preceding
task unless its prerequisites say otherwise, leaves every workspace compiling,
and exposes only behavior that works through a public boundary.

Task numbers are coordination metadata used only in this plan. Never put them
in source comments, diagnostics, filenames, generated assets, sample text, or
public documentation.

Every public Rust or C# API added or changed in a task receives concise
documentation in the same change. Authoring examples compile through the
public reexport rather than depending directly on the macro crate.

Black-box Rust tests use one of these boundaries:

- The shared syntax crate's public parse, canonicalize, identity, and native-
  support classification API.
- A fixture crate compiling a public `asset_generator::generate!` declaration.
- The public `cargo battlement reactant assets` command against a temporary
  Unity project and real Cargo workspace.
- Public Reactant snapshot conversion followed by `battlement-fake` state.
- A real supported Chrome or Chromium process selected by the public CLI.
- Public Unity editor assemblies and visible imported or runtime state.

The grammar corpus is batched. One compile-pass crate may contain many valid
declarations, while focused compile-fail crates isolate diagnostic categories
that cannot coexist in one build. The CLI runs the same corpus in as few Cargo
graph resolutions as practical. Do not create one compiler invocation for each
CSS spelling.

Tests assert stable categories and relevant symbol, path, property, and source
location context. They do not freeze complete diagnostic prose or internal
syntax-tree serialization.

Task acceptance describes the complete evidence required for that task; it
does not imply that every acceptance fixture belongs in recurring CI. Before
repository validation, stage every intended change, run the task's focused
tests, and run `./scripts/ci.py`. Tasks that change Unity editor or runtime
behavior run the relevant asset-specific one-off EditMode selection. Tasks that
change the authoring API also run doctests and documentation builds with
warnings denied. The complete project receives the repository-mandated
independent review once because it will exceed 500 non-test lines.

### Validation tiers and recurring-CI budget

The implementation maintains three validation entry points:

| Entry point | Purpose | Time policy |
|---|---|---|
| `python3 scripts/reactant_asset_validation.py fast` | Broad regression signal used by `ci.py --full` | Target 12 seconds; the measured suite delta must remain at or below 15 seconds |
| `python3 scripts/reactant_asset_validation.py exhaustive` | Complete automated compile, CLI, browser, filesystem, target, and Unity fixture corpus | One-off; minutes are acceptable |
| `python3 scripts/reactant_asset_validation.py performance` | The 1,000-file warm-path fixture and resource-count assertions | One-off on the designated performance runner |

Build this runner alongside the fixtures and connect its fast tier to CI in
Task 19. It uses the same fixtures and public commands for all tiers; tier
selection chooses cases, not an alternate product implementation. It never
enables a test-only production command or substitutes fake Cargo, browser,
filesystem, or Unity tools.

The fast tier maximizes semantic coverage per external process:

- Run the complete table-driven grammar, canonicalization, identity,
  native-support, manifest-schema, sidecar-schema, snapshot, and diagnostic-
  category corpora in process. These cases are cheap, so recurring CI does not
  sample them down.
- Compile one batched public macro pass fixture containing every declaration
  kind and one batched diagnostic fixture containing a representative case from
  each compile-time diagnostic family. The exhaustive tier retains isolated
  compile-fail fixtures for every spelling, span, target, and indirection case.
- Use one reusable public CLI fixture workspace, one Cargo graph resolution,
  and one Chrome or Chromium process with one context. In one ordered scenario,
  render a compact batch spanning every paint family, validate PNG and manifest
  structure, run read-only `check`, prove a warm no-op, mutate one declaration
  and one dependency, and prove selective invalidation. Canvas sizes and text
  runs stay intentionally small; feature breadth comes from batched requests,
  not separate browser launches.
- Exercise corruption and failure categories that can be produced by mutating
  bytes or metadata after the successful batch. Cases requiring a new Cargo
  graph, browser process, filesystem failure mode, or transaction interruption
  remain in the exhaustive or Manual QA tiers.
- Run generated-asset import and runtime-catalog smoke assertions inside the
  repository's existing EditMode invocation. Do not add a second Unity launch
  to recurring CI. The exhaustive tier owns the complete importer,
  Addressables ownership, restoration, and catalog-mismatch matrix in a
  separately selectable public EditMode assembly or category.
- Run native snapshot and fake-client cases in the existing Rust test process.
  Cross-target WebAssembly linkage, native/WebAssembly player builds, and the
  complete sample and Ditto matrices remain one-off because they add external
  compilation or player startup rather than unique per-commit semantic breadth.

The standalone fast runner and `ci.py --full` log elapsed time for the
in-process, compile, CLI/browser, and Unity portions plus the total. They fail
on a missing browser and on any behavioral assertion. A timing overrun is
evaluated by the median budget gate, not hidden by omitting failed or slow
samples.

Task 19 measures the incremental cost on the designated CI reference host with
toolchains, Cargo outputs, Unity Library state, generated fixtures, and the
repository CI cache warm. Run at least seven alternating baseline and candidate
`ci.py --full` samples, retain every wall-clock result, and subtract the two
medians. The candidate median may exceed the baseline median by no more than 15
seconds. First-time compilation is validated by the exhaustive tier but is not
part of this warm recurring-cost measurement. If the limit is missed, reduce
process launches or move redundant cases to the exhaustive tier; do not weaken
the public contracts covered by the combined tiers or raise the budget.

### Evidence contract

Each task retains the smallest public artifact that proves its acceptance:

- Compiler tasks retain the fixture name and pass or diagnostic transcript.
- CLI tasks retain the command transcript plus generated or inspected paths.
- Runtime tasks retain the public snapshot and resulting fake-client facts.
- Unity tasks retain the EditMode result and exact generated fixture paths.
- Browser tasks retain the command transcript, browser identity, manifest, and
  decoded PNG metadata rather than visual baseline images.

Visual screenshots are not required per task. The final release task captures
and reviews the complete preview and in-player gallery after every paint family
is present.

## Dependency overview

| Wave | Tasks | Result |
|---|---|---|
| 1 | 01–06 | Shared syntax, complete paint grammar, and classification |
| 2 | 07–10 | Public macro, linked catalog, source discovery, and warm index |
| 3 | 11–15 | Real-browser rendering, deterministic output, and CLI commands |
| 4 | 16–18 | Reactant snapshots, Unity integration, workflows, and gallery |
| 5 | 19 | CI, documentation, performance evidence, and release signoff |

## Wave 1: contract and authoring grammar

### Task 01 — Establish the generator crates and empty command surface [DONE]

**Prerequisites:** none. **Target:** 300–400 non-test lines.

Add the shared syntax, macro, and host generator crates to the workspace. Add
the `reactant assets generate`, `check`, and `preview` command hierarchy to
`cargo battlement`, with shared Unity project, rules manifest, feature, target,
browser, and optional work-report selections. Implement project containment and
the complete empty declaration-set behavior. Discovery may use real Cargo when
its graph cache is cold. After discovery, `generate` removes the generated root
and sibling `.meta`, writes no manifest or sidecar, and launches no browser or
Unity. `check` is read-only and succeeds only when all four generated artifacts
are absent. `preview` opens its empty page without starting a renderer.

**Black-box acceptance:** public CLI tests resolve the same project and rules
manifest from the repository root and nested directories, reject non-Unity
projects and escaped paths, print stable help, and make `generate`, `check`, and
`preview` obey the empty-set contract.

**Evidence:** command transcript for all three empty-set commands and the
resulting absent generated root, sibling `.meta`, manifest, and sidecar, plus
their public work reports.

### Task 02 — Parse declaration envelopes and generator metadata [DONE]

**Prerequisites:** Task 01. **Target:** 350–500 non-test lines.

Implement the shared token cursor, source spans, stable diagnostic categories,
the three declaration at-rules, non-raw identifiers, statement uniqueness, and
all generator metadata. Validate canvas, subject, slices, clipping order,
raster scale, filter, wrap, compression, and font-file placement, including
default and redundant-default rules.

**Black-box acceptance:** batched calls through the shared syntax crate's public
API cover every declaration kind, metadata default, order permutation, geometry
boundary, and required or forbidden statement. Each invalid body reports its
stable diagnostic category, symbol, property, and source span.

**Evidence:** fixture transcript containing one valid declaration of each kind
and representative metadata diagnostics with symbol and location.

### Task 03 — Add scalar CSS values and canonical encoding [DONE]

**Prerequisites:** Task 02. **Target:** 400–500 non-test lines.

Implement finite numbers, dimensions, percentages, angles, colors, strings,
functions, comma and space lists, and typed `calc`, `min`, `max`, and `clamp`
expressions. Add the deterministic tagged canonical encoder, shortest
round-tripping decimal serialization, negative-zero normalization, and stable
ordering rules.

**Black-box acceptance:** the public syntax API proves equivalent number and
color spellings produce one identity, source locations do not affect identity,
unordered values canonicalize identically, and ordered lists produce different
identities when reordered. Invalid arithmetic and non-finite results fail
before generation.

**Evidence:** identity table from the syntax fixture plus diagnostic output for
dimension and arithmetic failures.

### Task 04 — Parse backgrounds, gradients, and local image values [DONE]

**Prerequisites:** Task 03. **Target:** 400–500 non-test lines.

Implement background colors and ordered image layers, linear, radial, conic,
and repeating gradients, color stops and hints, CSS positions and sizes,
repeat, origin, clip, and `unity-url`. Preserve semantic layer ordering while
rejecting unsupported or explicitly redundant shorthand forms.

**Black-box acceptance:** batched public syntax calls exercise every image
family, single and multiple layers, viewport and font-relative units, local PNG
references, and representative invalid shape, stop, position, and shorthand
combinations. Accepted declarations expose their canonical dependency
references without resolving files or launching a browser.

**Evidence:** syntax corpus transcript and canonical identities for the complete
background corpus.

### Task 05 — Parse advanced box paint [DONE]

**Prerequisites:** Task 04. **Target:** 400–500 non-test lines.

Implement border shorthands and longhands, elliptical radii, ordered inset and
outer shadows, clip basic shapes and typed path commands, mask layers and
composition, background blending, isolation, and opacity. Reject external
context paint and all syntax outside the closed catalog.

**Black-box acceptance:** public syntax fixtures cover each supported border
style, shadow order, clip shape, path command, mask mode, mask composition, and
blend mode. Permuting nonoverlapping declarations preserves canonical output.
Every overlapping shorthand and longhand combination fails in either order.
Degenerate polygons, malformed paths, external blending, and unsupported
properties fail with the declaring symbol and property.

**Evidence:** public syntax transcript for the advanced-box corpus.

### Task 06 — Complete effects, text paint, and native-only rejection [DONE]

**Prerequisites:** Task 05. **Target:** 400–500 non-test lines.

Implement the complete filter and 2D transform catalogs, transform origins,
text content, font metadata, spacing, alignment, white space, solid and
advanced fills, stroke, shadows, and text-level effects. Add the closed native
support table and reject a complete request when Battlement UI can reproduce
it without generator-only paint.

**Black-box acceptance:** public syntax and classification fixtures cover every
filter and transform function, plain and advanced text, exact face metadata,
missing required text fields, invalid property combinations, and every native-
support row. A native-only request names the corresponding `Style` or `Image`
replacement in the Reactant authoring API, while adding one generator-only
feature makes the full composition valid.

**Evidence:** complete grammar corpus transcript and representative native-only
diagnostic output.

## Wave 2: macro, discovery, and identity

### Task 07 — Generate public handles and linked registrations [DONE]

**Prerequisites:** Task 06 and the completed Reactant host-façade migration.
**Target:** 350–500 non-test lines.

Expose `battlement_reactant::asset_generator`, reexport `generate!`, and add the
three copyable handle types and logical geometry metadata. Expansion emits one
public static with const address and geometry plus one `inventory`
registration. Add image, source, background-style, and nine-slice style methods
without layout side effects. `image()` returns the Reactant `Image` façade,
`image_source()` returns the shared `ImageSource` value, and
`background_style()` returns the shared `Style` value. The façade privately
lowers to exactly one `UiImage` without a wrapper.

Before using the registry in production expansion, link declarations from two
dependency crates into native and `wasm32-unknown-unknown` fixtures. Prove
enumeration is complete and duplicate-free in both targets. Export a count and
address-hash function from the WebAssembly fixture and invoke it through Node's
standard WebAssembly API so the test exercises constructor initialization
rather than merely inspecting the compiled module.

**Black-box acceptance:** public doctests compile each handle kind through the
Reactant authoring API and inspect shared support values or the lowered
`UiImage`, as applicable. The image façade lowers to one host, and generated
styles produce equivalent documents across host-method permutations that
preserve repeatable-layer order. Target fixtures enumerate both dependency
registrations once when executed natively and through Node. A consumer fixture
compiles with no Unity project, browser, generated output, or
generator-specific environment configuration.

**Recurring selection:** compile the batched public declaration fixture and
run native enumeration in the fast tier. Keep the separate native dependency,
WebAssembly target build, and Node execution in the exhaustive tier.

**Evidence:** passing host-façade suite transcript, doctest output, and
native/WebAssembly registry fixture results.

### Task 08 — Discover declarations through both Cargo graphs [DONE]

**Prerequisites:** Task 07. **Target:** 400–500 non-test lines.

Resolve the selected rules package through real Cargo metadata for the host and
`wasm32-unknown-unknown` targets. Walk reachable Rust module items, identify the
exact invocation path, reuse the shared parser, and compare canonical
declaration sets and local package identities across targets.

Reject aliases, reexports, nested invocations, macro definitions, declarative
wrappers, conditional compilation, target-only declarations, and outside-path
packages without portable coordinates. Do not execute or expand game code.

**Black-box acceptance:** fixture workspaces cover workspace, registry, Git,
and local dependency coordinates; nested modules; feature selections; host and
WebAssembly parity; and every unsupported indirection. Diagnostics name both
graph origins when parity fails. Replaying the Wave 1 corpus through public
macro compilation and CLI discovery produces the same identities or diagnostic
categories as the shared syntax API.

**Recurring selection:** reuse one already resolved fixture graph for parser
parity and representative placement diagnostics. The exhaustive tier owns the
complete coordinate, graph mutation, target parity, and unsupported-indirection
matrix.

**Evidence:** CLI discovery transcript for passing multi-crate and failing
cross-target workspaces.

### Task 09 — Resolve dependencies, identities, and deduplication [DONE]

**Prerequisites:** Task 08. **Target:** 400–500 non-test lines.

Resolve `unity()` and `unity-url()` paths against the selected Unity project,
validate containment and supported decoded formats, hash normalized dependency
bytes, and validate font metadata and character coverage available without a
browser. Compute public addresses, deterministic Unity GUIDs, dependency
identity records, and domain-separated directory metadata. Final output cache
keys wait for the browser and renderer identities established in Task 11.

Deduplicate identical requests across symbols and packages. Fail when one
canonical request resolves different dependency bytes, when distinct canonical
bytes collide, or when a generated address conflicts with the reserved
namespace contract.

**Black-box acceptance:** real temporary workspaces prove path normalization,
symlink containment, format and extension agreement, stable addresses across
dependency-byte changes, changed dependency identity, successful
deduplication, and complete collision diagnostics.

**Evidence:** generated identity table and diagnostics for escaped, mismatched,
and conflicting dependencies.

### Task 10 — Add incremental discovery and output fingerprints [DONE]

**Prerequisites:** Task 09. **Target:** 400–500 non-test lines.

Implement the ignored local index keyed by rules, feature, and target
selection. Record graph inputs, absent configuration probes, source module
edges, declarations, diagnostics, dependency fingerprints, output
fingerprints, and the semantic output-set hash. Treat corrupt or unknown index
shapes as cache misses.

Reuse unchanged Cargo graphs and file records. Reopen only changed sources or
dependencies, and atomically update discovery state only when the command is
allowed to write it.

**Black-box acceptance:** repeated real CLI invocations prove a no-op leaves
generated output untouched, an unrelated Rust edit updates only discovery
state, a declaration move changes no file below `Assets`, and graph-affecting
manifest, lockfile, feature, target, configuration, and environment changes
rerun resolution.

**Recurring selection:** the shared fast CLI scenario proves the warm no-op,
one source edit, and one dependency edit without resolving another graph. Run
the complete graph-affecting input matrix in the exhaustive tier and the
1,000-file resource-count fixture in the performance tier.

**Evidence:** before-and-after file inventory, timestamps, and public command
work reports for each incremental case. A warm no-op report proves zero source,
dependency, PNG, or browser-executable opens and zero Cargo runs, subprocesses,
browser launches, browser contexts, and writes other than the report.

## Wave 3: browser rendering and generated output

### Task 11 — Select and control one real browser session [DONE]

**Prerequisites:** Task 10. **Target:** 400–500 non-test lines.

Implement deterministic supported-browser discovery on macOS, Windows, and
Linux plus the explicit override. Launch one isolated stable Chrome or Chromium
process, connect through a synchronous browser-protocol adapter, record the
executable and protocol identities, block network traffic, and reuse one
context for all cache misses.

Combine the canonical request, dependency identity, effective scale, selected
browser identity, and embedded renderer identity into the final output cache
key. Browser and renderer identity changes invalidate affected requests without
changing their public addresses.

Configure locale, time zone, color scheme, reduced motion, root font size,
profile isolation, caching, service workers, and animation time exactly as the
design requires. Do not download, bundle, or launch a separate browser family.

**Black-box acceptance:** the public CLI selects the documented installed
browser, records its identity, renders multiple minimal requests through one
reported launch and one reported context, produces the same per-request hashes
in reversed order, rejects an explicit non-Chrome executable, and fails rather
than skips when no supported browser is available. Browser and renderer
identity changes alter cache keys.

**Recurring selection:** the fast tier performs one default-browser selection,
one launch, one context, the compact paint batch, and the missing-browser
diagnostic without launching a second renderer. Explicit executable rejection,
identity invalidation, reversed-order regeneration, and the platform selection
matrix run in the exhaustive tier or cross-platform Manual QA.

**Evidence:** CLI transcript with selected browser identity and one-session
request count.

### Task 12 — Render CSS and validate deterministic PNG metadata [DONE]

**Prerequisites:** Task 11. **Target:** 400–500 non-test lines.

Serialize typed requests into isolated generated documents, install validated
local images and fonts using internal blob URLs made from their validated
bytes, revoke those URLs after each request, and apply device scale per request,
wait for font and layout stability, capture exact device dimensions, and
re-encode through one fixed Rust PNG configuration.

Validate browser shaping, fallback, `.notdef` glyphs, variation selectors,
joiners, combining sequences, alpha bounds, permitted canvas-edge contact,
sRGB RGBA shape, dimensions, and deterministic ancillary-chunk policy. Abort
the complete render set on any failed request.

**Black-box acceptance:** one real-browser batch covers representative
gradient, clip, mask, inset and outer shadow, filter, transform, nine-slice,
local-image, and advanced-text declarations. Tests inspect dimensions, alpha
bounds, PNG structure, and repeat hashes but do not judge visual pixel values.
Separate valid fixtures report the stable warning categories for large raster
allocation, lossy translucent compression, and near-edge paint.

**Recurring selection:** use the same compact real-browser batch for every
paint family and cheap metadata assertion. The exhaustive tier reruns the
larger geometry, shaping, warning, order-independence, and per-family edge-case
corpora; visual judgment remains Manual QA.

**Evidence:** browser identity plus decoded metadata and hashes for the batch.

### Task 13 — Write manifests, Unity metadata, and the runtime sidecar [DONE]

**Prerequisites:** Task 12. **Target:** 400–500 non-test lines.

Implement the strict canonical manifest, asset records, browser identity,
dependency records, import settings, deterministic PNG, sidecar, and directory
`.meta` files, manifest metadata, and the Resources runtime catalog. Validate
all path, hash, address, GUID, raster, geometry, import, and sidecar
relationships while rejecting missing and unknown fields.

Compute the sidecar only after canonical manifest bytes are final. It contains
the manifest hash and sorted address set but is not itself an Addressable entry
or manifest asset record.

**Black-box acceptance:** public `check` accepts a complete generated fixture,
rejects one-field corruptions across every record family, accepts irrelevant
YAML key ordering, and rejects extra importer overrides or labels. Unity
imports the sidecar as `TextAsset` and every PNG as `Texture2D`. The generated
tree includes deterministic `.meta` files for the Resources directory and its
sidecar.

**Recurring selection:** mutate every manifest and sidecar field in process,
then import one representative texture and the sidecar inside the existing
EditMode invocation. The exhaustive tier imports the complete texture-settings
matrix and exercises Addressables metadata variations.

**Evidence:** generated tree listing, canonical manifest and sidecar hashes, and
focused Unity import result.

### Task 14 — Make generated-set replacement transactional [DONE]

**Prerequisites:** Task 13. **Target:** 350–500 non-test lines.

Stage and validate a complete sibling generated set, flush the authoritative
manifest, preserve the previous root, install the staged root with same-volume
renames, and recover the last manifest-complete root on startup. Validate the
stable sibling directory metadata only after a successful swap.

An ordinary parse, dependency, browser, paint-bound, PNG, metadata, or
permission failure must return without modifying the prior successful root.
No-op generation must preserve bytes and modification times.

**Black-box acceptance:** real-filesystem command tests cover first install,
complete replacement, stale-file removal, no-op generation, recovery from
preexisting staged and backup directories, and ordinary pre-swap failures.
Process termination during individual phases remains final Manual QA.

**Evidence:** byte and timestamp comparison across install, no-op, replacement,
and recovery runs.

### Task 15 — Complete `generate`, `check`, and `preview` [DONE]

**Prerequisites:** Task 14. **Target:** 400–500 non-test lines.

Assemble discovery, cache validation, browser rendering, transaction, and
diagnostics into the three public commands. Report discovered, deduplicated,
current, rendered, and stale counts. Classify added, changed, missing, corrupt,
and stale assets without repairing them in `check`.

Generate a temporary preview gallery with checkerboards, metadata, authored
property summaries, source locations, bounds, dependencies, and interactive
nine-slice controls. Use the operating system's ordinary URL opener and do not
bundle preview code with a player.

**Black-box acceptance:** `check` is read-only, current `generate` starts no
browser and touches no generated file, changed inputs rerender only affected
requests, and `preview` first generates then exposes every declaration and all
required metadata. Empty preview starts no renderer.

**Evidence:** command transcripts, changed-path inventory, and preview metadata
page captured for final manual visual review.

## Wave 4: Reactant, Unity, workflows, and sample

### Task 16 — Merge linked generated assets into Reactant snapshots [DONE]

**Prerequisites:** Task 15. **Target:** 350–500 non-test lines.

Enumerate, sort, and deduplicate linked registrations during every
`SessionUi::into_parts` and `into_response` conversion. Merge them as
`PreparedAsset::Texture` cases without changing caller-owned assets outside the
reserved prefix. Reject every caller-authored prepared asset inside the prefix,
including an identical Texture, and name its address, case, source symbol, and
the registry's exclusive ownership. Panic on conflicting linked registrations
with both symbols and metadata.

Keep handle use on the Reactant `Image` façade, shared style and image-source
values, and ordinary Battlement UI prepared-asset and lease paths. Private
lowering must produce one `UiImage`; do not add asset-generator-specific
loading, reconciliation, wrapper hosts, or late preparation commands.

**Black-box acceptance:** public fake-client snapshots contain unused and
later-state registrations in sorted order, deduplicate identical linked
registrations, preserve unrelated caller assets, reject every caller-authored
reserved-prefix case, and restore the complete generated union on an
authoritative replacement.

**Evidence:** public snapshot and fake prepared-set facts for initial and
replacement conversions.

### Task 17 — Register and validate generated assets in Unity [DONE]

**Prerequisites:** Task 16. **Target:** 400–500 non-test lines.

Add an editor owner that reads and strictly validates the manifest and runtime
sidecar, imports each texture with exact settings, detects user-owned address
conflicts, and temporarily registers generated entries for authoring and player
builds. Follow the Opus capture-and-restore ownership pattern without adopting
or overwriting user entries.

At runtime, load the Resources sidecar before accepting an authoritative
snapshot. Compare its sorted address set with reserved-prefix
`PreparedAsset::Texture` cases before starting asset loads or commands, and
repeat the comparison for authoritative replacements.

**Black-box acceptance:** public EditMode tests cover exact import type and
settings, temporary registration, conflict rejection, normal restoration,
absent empty-set behavior, and an unchanged semantic manifest causing no Asset
Database refresh, reimport, settings serialization, or Addressables-cache
invalidation. Runtime tests cover initial startup mismatch, an opaque procedural
macro's extra linked registration, and atomic replacement mismatch. Normal
preparation still rejects missing or wrong-type textures.

**Recurring selection:** add representative import settings, catalog parity,
and restoration smoke assertions to the repository's existing EditMode launch;
keep them independent of test order. Run the complete import, conflict,
restoration, refresh, reimport, and mismatch matrix through the separately
selectable exhaustive EditMode assembly or category.

**Evidence:** focused EditMode result and runtime session failure transcript for
both catalog mismatch directions.

### Task 18 [DONE] — Integrate authoring, sample builds, and the asset gallery

**Prerequisites:** Task 17. **Target:** 400–500 non-test lines.

Run generation before building the rules plugin in `cargo battlement author`,
`sample build`, and `sample run`. Pass already resolved Cargo graphs downward
when available and skip generated registration for an empty catalog. Apply the
hook to every convention-based sample rather than a Reactant name allowlist.

Add an eighth Reactant sample screen named Assets. Include advanced gradient
text, one clipped layered background, an explicit resizable nine-slice control,
and representative gradient, clip, shadow, mask, filter, and skew cases. Keep
layout and interaction native, update the screen inventory and feature ledger,
and add complete initial and resize-restored Ditto coverage.

**Black-box acceptance:** a fixture sample with a non-Reactant name invokes
generation generically; native and WebAssembly Reactant players expose the same
linked catalog and addresses; later-state assets are prepared initially; and
the sample's public screen inventory, feature ledger, and Ditto ledger remain
exhaustive.

**Recurring selection:** fast Rust and Unity assertions cover the generic hook,
linked catalog, screen inventory, and feature ledger without building players.
Native and WebAssembly player builds plus the complete Ditto gallery remain in
the exhaustive runner and final Manual QA.

**Evidence:** author/build command transcripts, native and WebAssembly catalog
records, and final gallery Ditto result paths.

## Wave 5: release proof

### Task 19 — Complete CI, performance evidence, documentation, and release QA [DONE]

**Prerequisites:** Task 18. **Target:** 300–450 non-test lines.

Complete `scripts/reactant_asset_validation.py` with `fast`, `exhaustive`, and
`performance` tiers. Integrate only the fast selection into `ci.py --full`:
keep its Rust cases in the existing root test process, keep its Unity smoke
cases in the existing EditMode launch, and add only the consolidated
CLI/browser scenario as a new external-process lane. Running the standalone
fast command selects the same cases for focused validation. Do not add a
per-fixture Cargo, browser, Unity, sample-build, or player-launch loop to
recurring CI. CI fails when no supported stable browser is present.

Print per-portion and total fast-tier timings. Measure at least seven
alternating warm baseline and candidate `ci.py --full` runs on the designated
reference host, retain all samples, and report the median delta. The asset
generator may add no more than 15 seconds of median wall-clock time, with a
12-second fast-tier target. Parallel execution may reduce the observed delta,
but does not excuse adding redundant external processes or omitting the raw
timing evidence.

Finish public authoring, command, generated-output, diagnostics, cache,
Addressables, runtime, and sample documentation. Validate every checked example
through the public crates and commands. Run the repository's one independent
review for the complete implementation and repair confirmed findings.

Run the exhaustive automated tier, then every numbered item in the technical
design's authoritative `Manual QA` section, including visual gallery review,
platform browser selection, real-tool failure and interruption exercises,
native and WebAssembly players, and the performance tier's 20-invocation
fixture. Remove temporary test projects and retain only the named release
evidence.

**Black-box acceptance:** recurring repository CI passes within the measured
median budget. Documentation builds, the exhaustive public fixture corpus,
real-browser metadata batch, exhaustive Unity selection, Reactant sample
suites, runtime catalog parity, performance tier, and every manual release
check also pass from the final staged tree without being added to every
`ci.py --full` invocation.

**Evidence:** raw baseline and candidate CI timing samples with median delta,
fast-tier portion timings, exhaustive-run transcript, documentation transcript,
independent review, gallery result paths, failure-recovery record, and
performance report.

## Completion criteria

The Reactant asset generator is complete when all tasks are marked done and all
of the following are true:

- Every declaration kind and every supported paint family has public compile,
  discovery, generation, and diagnostic coverage.
- Macro and CLI parsing agree, canonical identity is stable, and dependency or
  browser changes rerender only affected requests at unchanged public addresses.
- Warm no-op generation starts no subprocess, opens no unchanged source,
  dependency, or PNG, writes nothing, and meets the release performance budget.
- Generated PNGs, metadata, manifest, sidecar, GUIDs, and import settings form
  one strict transactionally replaced set.
- Every linked registration is prepared in every authoritative snapshot and
  exactly matches the bundled runtime catalog before Unity accepts that
  snapshot.
- Authoring and every convention-based sample build or run generate assets
  before compiling rules, without permanently changing user Addressables state.
- The Assets sample screen works in native and WebAssembly players and its
  complete gallery has been visually reviewed in the installed browser.
- The warm median `ci.py --full` wall-clock delta is no more than 15 seconds,
  the fast tier requires a real supported browser, and no validation tier
  contains a fixture external-tool executable or exact pixel baseline.
- The exhaustive and performance runners remain documented and runnable after
  release so later changes can repeat the complete automated and scale
  validation without placing it on every commit.

## Manual QA

The technical design's `Manual QA` section is the complete, authoritative
checklist. First run the exhaustive automated tier from clean Rust outputs,
then run every numbered item from a clean checkout and retain its command
transcripts, generated manifests, Unity results, player results, screenshots,
and performance reports. The groups below are only a release-signoff index;
they do not replace or narrow that checklist.

At minimum, release signoff must confirm these grouped outcomes:

1. Generate, check, and preview a clean Reactant project, then repeat each warm.
   Confirm one browser session handles misses, the warm path starts no browser,
   and no-op commands preserve generated modification times.
2. Inspect every preview asset visually. Confirm gradients, clips, masks,
   shadows, filters, transforms, advanced text, transparency, bounds, and
   nine-slice resizing match the authored declarations.
3. Change one source declaration, dependency, raster scale, browser identity,
   generated PNG, and importer field in separate runs. Confirm only affected
   assets become stale or rerender and public addresses remain stable where the
   design requires.
4. Terminate Chrome and the generator at the rendering, staging, preserved-root,
   and install boundaries. Confirm recovery always selects the last
   manifest-complete root and never activates partial output.
5. Exercise unreadable and escaped dependencies, browser protocol failure, PNG
   corruption, unavailable output storage, user address conflict, and Unity
   import failure with real tools. Confirm the previous successful generated
   root and user Addressables state remain intact.
6. Run authoring plus native and WebAssembly Reactant sample players. Confirm
   the bundled runtime catalog matches linked registrations, unused variants
   are prepared before first use, and wrong-type or missing textures fail the
   session without placeholders.
7. Corrupt each side of runtime parity separately and repeat on an authoritative
   replacement. Confirm Unity rejects the snapshot before asset loading or
   command execution and retains the previous authoritative state.
8. Run the complete Assets sample screen and Ditto suite. Resize nine-slice
   specimens above and below their authored dimensions, restore the initial
   state, and inspect the accepted native and WebAssembly output.
9. On the designated macOS performance runner, warm the 1,000-file,
   100-declaration fixture and run 20 no-op invocations. Confirm median below
   200 milliseconds, 95th percentile below 300 milliseconds, no more than 1,250
   stat calls, eight file opens, and one MiB read. Confirm no source,
   dependency, PNG, or browser-executable opens, Cargo runs, subprocesses,
   browser launches, contexts, or writes beyond the report itself.
