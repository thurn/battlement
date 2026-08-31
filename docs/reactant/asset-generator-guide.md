# Reactant generated assets

Reactant generated assets turn static CSS-style paint into Unity `Texture2D`
assets before a game builds or runs. Use them for gradients, clipping, masks,
shadows, filters, transforms, and advanced text that Unity UI Toolkit cannot
represent natively. Continue to use ordinary Reactant styles for layout,
interaction, accessibility, animation, and paint that UI Toolkit supports.

The [technical design](asset-generator.md) is the complete language and
behavior contract. This guide covers the normal authoring and operations
workflow.

## Author an asset

Import the public Reactant module and declare one asset per `generate!` call:

```rust
use battlement_reactant::asset_generator;

asset_generator::generate! {
  @nine-slice ACTION_BUTTON {
    @canvas 760px 140px;
    @slices 24px 26px 24px 26px;
    @allow-clipping top right bottom left;

    clip-path: polygon(
      2.4% 0%, 97.6% 0%, 100% 12%, 100% 88%,
      97.6% 100%, 2.4% 100%, 0% 88%, 0% 12%
    );
    background: linear-gradient(110deg, #b9fbff 0%, #ff4bd1 100%);
  }
}
```

The declaration emits a typed `ACTION_BUTTON` static. A background or text
asset can supply an `Image` with `image()` or native background paint with
`background_style()`. A nine-slice asset additionally exposes its logical
slice insets and produces a background style with the correct Unity source
pixel values and scale.

Every declaration is static. Runtime parameters, selectors, arbitrary CSS,
URLs, JavaScript, SVG, and custom pixel callbacks are deliberately absent.
Create a named declaration for each runtime variant. Reactant discovers all
linked variants and prepares them in the first authoritative snapshot, even
when the first screen does not display them.

Local dependencies use Unity-project-relative paths:

```rust
asset_generator::generate! {
  @text-image STATUS_TITLE {
    @canvas 420px 72px;
    @font-file unity("Assets/Fonts/Status.ttf");
    content: "Systems Ready";
    font-size: 42px;
    color: transparent;
    background: linear-gradient(90deg, #fff, #74e5ff);
    background-clip: text;
    text-shadow: 0 3px 8px rgba(0, 0, 0, 0.65);
  }
}
```

Font and image paths may not escape the selected Unity project. The generator
records their bytes in the cache identity but keeps the public Addressables
address stable when those bytes change.

## Generate, check, and preview

Run commands from the Unity project or pass it explicitly:

```text
cargo battlement reactant assets generate
cargo battlement reactant assets check
cargo battlement reactant assets preview
cargo battlement reactant assets generate --project samples/reactant
```

The default rules manifest is `rules/Cargo.toml`. Use `--manifest-path` when a
project uses another package. `--features`, `--all-features`, and
`--no-default-features` apply identically to the host and WebAssembly Cargo
graphs. A declaration that differs between those graphs is rejected.

`generate` discovers the complete catalog, renders only cache misses in one
Chrome or Chromium process and one isolated browser context, then replaces the
generated set transactionally. A current invocation is a true no-op: it starts
no subprocess, opens no source, dependency, generated PNG, or browser
executable, and changes no generated modification time.

`check` performs the same validation without repairing or writing project
state. Use it in read-only verification and before packaging a project by a
custom workflow. It exits unsuccessfully when any declaration, dependency,
browser identity, PNG, metadata file, manifest, or sidecar is stale.

`preview` first makes the catalog current, writes a local HTML gallery, and
opens it with the system browser. The gallery shows the public address, canvas,
raster dimensions, subject and alpha bounds, clipping policy, warnings, and
nine-slice resizing controls. It is a review tool, not a golden pixel test:
browser raster output can change with the recorded Chrome or Chromium version.

Pass `--browser PATH` to select an installed Chrome or Chromium explicitly.
The executable must satisfy the real debugging-protocol identity check; a
lookalike executable is rejected. Without the option, the command searches the
documented platform locations. A supported stable browser is required for a
cache miss and for recurring validation.

Pass `--work-report PATH` to write canonical JSON counters for filesystem,
Cargo, subprocess, and browser work. Reports are useful for proving warm no-op
behavior and investigating unexpected invalidation.

## Generated output

The owned output root is:

```text
Assets/Generated/BattlementReactant/
```

It contains deterministic PNG paths, Unity `.meta` files, `manifest.json`, and
`Resources/BattlementReactantAssetCatalog.json`. The sibling
`BattlementReactant.meta` is owned by the same transaction. Do not edit,
rename, label, bundle, or partially copy these files.

The manifest is authoritative for editor import and diagnostics. The runtime
sidecar contains the exact sorted address set plus the manifest hash. PNG names
and Unity GUIDs derive deterministically from canonical requests. Directory
GUIDs are deterministic as well, so a clean regeneration does not churn Unity
references.

Generation stages a complete replacement beside the installed root, validates
it, preserves the old root, and activates the new root. On interruption, the
next mutating command recovers the last manifest-complete set. A failed render,
dependency read, browser session, staging operation, or install validation
never publishes a partial catalog.

Generator caches and discovery indexes live below
`Library/BattlementReactant`. They are local derived state and may be removed
to force clean discovery or rendering. Removing them does not change public
addresses. Do not commit the cache or generated output.

## Diagnostics

Diagnostics identify a stable category and the relevant symbol, property,
path, and source location. Common repairs are:

- Replace native-only paint with an ordinary Reactant `Style`. A solid rounded
  rectangle, for example, does not need raster generation.
- Add generator-only paint when a composite genuinely requires it, such as a
  gradient, mask, unsupported clip, filter, or advanced text treatment.
- Increase the canvas or subject padding when paint reaches an undeclared
  edge. Use `@allow-clipping` only when contact with that edge is intentional.
- Keep local font and image dependencies within the Unity project and ensure
  the font contains every authored glyph.
- Regenerate after changing dependencies, raster scale, the selected browser,
  generated bytes, or Unity import metadata.
- Remove user Addressables entries that claim the reserved
  `battlement-reactant/generated/` prefix or a generated GUID.

Warnings call attention to valid but review-worthy output, including large
raster allocations, lossy compression with transparency, and paint close to a
permitted edge. They do not relax validation.

## Addressables and runtime behavior

Authoring and convention-based sample build/run commands generate assets
before compiling rules. Unity imports every PNG as a non-sprite `Texture2D`
with the exact filter, wrap, compression, alpha, color-space, mip, and
nine-slice settings from its metadata.

Build and authoring operations temporarily register the generated entries in a
dedicated Addressables group. User-owned groups, entries, labels, schemas,
dirty state, and serialized settings are restored on success, failure, and
interruption. An address or GUID conflict fails before user state is changed.

The linked Rust registration set, bundled runtime sidecar, imported texture
types, and every authoritative snapshot must agree exactly. Initial connection
and authoritative replacement reject a missing address, unexpected address,
wrong asset type, or catalog hash mismatch before asset loading or command
execution. Reactant never substitutes a placeholder for a generated texture.

## Sample gallery

The Reactant sample's **Assets** screen demonstrates gradient text, layered and
clipped backgrounds, explicit nine-slicing, gradients, clips, shadows, masks,
filters, and skew. Use its size controls to stretch the nine-slice specimen
above and below the authored size, then restore the initial state before a
Ditto capture.

The ordinary sample commands generate the catalog automatically:

```text
cargo battlement sample run reactant
cargo battlement sample run reactant --web --web-unthreaded --port 4179
```

The native and WebAssembly players bundle the same public address set. Later
gallery states require no runtime asset-load command because all linked assets
were prepared in the initial snapshot.

## Validation tiers

Run the focused public selection with:

```text
python3 scripts/reactant_asset_validation.py fast
```

It reports in-process, compile, consolidated CLI/browser, Unity, and total
timings. Repository `ci.py --full` reuses its existing Rust and Unity processes
and adds only the consolidated CLI/browser portion.

Release validation is intentionally separate:

```text
python3 scripts/reactant_asset_validation.py exhaustive
python3 scripts/reactant_asset_validation.py performance
```

The exhaustive tier runs the complete ignored CLI fixture corpus, the
exhaustive Unity category, and native and WebAssembly Reactant builds. The
macOS performance tier creates a disposable 1,000-file, 100-declaration
fixture, performs 20 warm no-op invocations, validates the work counters, and
writes every sample plus its aggregate report under
`artifacts/reactant-asset-validation`. Temporary fixture projects are removed
when the command exits.
