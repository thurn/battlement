# Masonry Standard Setup Technical Design

Status: proposed extension to the Masonry client and authoring model

## Summary

Masonry is a thin Unity rendering and input client for games whose rules and
authoritative state live in Rust. Today a game can avoid game-specific C# but
must still own a complete Unity project. The Basic and Tic-Tac-Toe samples each
carry project settings, package manifests, startup scenes, asset-build
configuration, rendering assets, fonts, and Rust-library installation that are
nearly identical.

The **standard setup** is the model defined here: a game owns no Unity project
and uses one Masonry-owned **standard shell**, an immutable macOS application
template containing Unity and the Masonry client but no game rules. See
[Standard shell architecture](#standard-shell-architecture). A standard game
repository contains Rust rules, a `masonry.toml` manifest, checked-in
**generated asset bindings**, Rust functions that expose declared assets by
type, and **scenarios**, TOML files that drive the packaged game through input,
pixels, and logs. See
[Generated Rust asset bindings](#generated-rust-asset-bindings) and
[Declarative scenario contract](#declarative-scenario-contract).

This is not a universal game launcher. Each output application contains one
rules engine and one game configuration. The shell is reused as an immutable
input, but **app assembly**, copying a shell and installing one game's rules,
content, metadata, and signature, produces an independently named application.
See [Game app assembly and signing](#game-app-assembly-and-signing). Games that
need custom C#, shaders, packages, rendering pipelines, or unsupported Unity
settings continue to use Masonry's advanced bring-your-own-Unity-project path.

The standard path optimizes the common Rust iteration loop. Changing Rust
rebuilds and installs only the native rules library. Changing content rebuilds
only the game's **Addressables content**, Unity asset bundles plus a catalog
that maps stable keys to their contents. See
[Content and Addressables design](#content-and-addressables-design). Unity
builds a shell only when no exact
cached or published shell exists, and Unity compiles game content only when the
game declares custom assets. A game using only standard assets can build and
run without invoking Unity once an exact shell is available. Generated or
game-supplied assets also require an exact cached content pack; a content cache
miss invokes Unity without rebuilding the shell.

**Development** and **release** are separate shell profiles. Development adds
authenticated automation, **diagnostic records**, structured log messages with
a level, source, message, and fields, plus an in-game console, simulated input,
screenshots, videos, and TOML scenarios. Release retains warning and error
records in ordinary logs but contains no console, automation listener,
diagnostic stream, simulated input, or capture code. The native rules
**application binary interface (ABI)**, the exported
function and memory-ownership contract between Unity and Rust, plus shell
metadata, content-format version, generated bindings, and installed CLI must
have exactly matching versions. This design adds no translation layer for the
old sample layout or native ABI v1.

## Related information

- [Masonry Technical Design](technical-design.md) defines the normative
  **Rust/Unity protocol**, the messages and memory rules through which Rust owns
  game state while Unity renders it. This document changes setup and
  distribution, not gameplay semantics unless it defines a new interface.
- [Native plugin development](native-plugin-development.md) documents the
  existing **dylib**, the macOS dynamic library containing Rust rules, plus its
  verification, replacement, multi-architecture handling, and signing behavior
  reused by app assembly.
- [Visual evidence capture](visual-capture.md) documents the current simulated
  Unity Input System devices, acknowledged capture messages, screen capture,
  system **FFmpeg** video encoding, and cleanup behavior generalized by
  scenarios.
- [Masonry v1 implementation plan](implementation-plan.md) records the current
  sample-owned Unity workflow that this design supersedes for Basic and
  Tic-Tac-Toe.
- [Fake client design](fake-client-design.md) defines the **fake client**, a
  Rust-only substitute for Unity used to validate protocol behavior. Standard
  player scenarios do not replace those tests.
- [UnityIngameDebugConsole v1.8.9][console] at commit
  `4467c225eaaf5c0db62a11e2c6851a9fdb64763c` is the pinned console user
  interface. Its [MIT license][console-license] is retained with distributions.
  Masonry owns the diagnostic model and uses the dependency only as a
  development viewer.

[console]: https://github.com/yasirkula/UnityIngameDebugConsole/tree/4467c225
[console-license]:
  https://github.com/yasirkula/UnityIngameDebugConsole/blob/4467c225/LICENSE.txt

## Problem and current state

The reusable Unity package already provides `MasonryRunner`,
`MasonryBootstrap`, communication with embedded or HTTP-hosted rules,
**MessagePack** binary serialization, Addressables storage, object
construction, grouped command execution, pointer and keyboard input, and error
reporting. A game with no custom C# still has to put those components into a
Unity scene and maintain the project around them.

The Basic and Tic-Tac-Toe samples demonstrate the duplication. Each sample
owns a complete `Assets`, `Packages`, and `ProjectSettings` set. Both include a
**bootstrap scene**, the first Unity scene that creates and connects the
Masonry runtime, an Addressables settings graph, TextMesh Pro (TMP) resources,
render-pipeline configuration, input configuration, and a location into which
the Rust dylib is copied. Tic-Tac-Toe additionally owns three ordinary PNGs.
Basic owns colored materials whose only purpose is to supply build-safe Unity
materials for Rust-created primitives.

The current `cargo masonry sample` command is repository-specific. It searches
parent directories for `samples/<name>`, reads `sample.toml`, locates the
sample's rules manifest, builds `libmasonry_rules.dylib` for the current
machine's macOS architecture, and copies it into `Assets/Plugins/macOS`. It
then invokes the
Unity Editor without its user interface. Unity configures how it loads the
dylib, cleans and builds Addressables, and builds the selected sample scene
into a `.app`.

This flow proves that a Rust-authored standalone game works, but every sample
is responsible for configuration that is logically part of Masonry. It also
makes a Rust edit look like a Unity project edit to the rebuild detector. An
external repository cannot use the command because discovery assumes the
Masonry repository and its `samples` directory.

The root repository is already a Unity project pinned to the supported Unity
and package versions. It owns the Universal Render Pipeline (URP) settings,
integration assets, a performance-check scene, and visual-capture code. That
project is the appropriate single authoritative source for the standard shell,
provided test and capture content is isolated from release-player compilation.

In this design, "no Unity project" means a game does not own Unity
infrastructure. Unity remains an implementation dependency of two operations:
building a missing shell from exact source, and importing or building custom
game content. Those operations happen in Masonry-owned disposable workspaces.
They do not create a Unity project that the game must understand, update, or
commit.

## Goals and architectural invariants

The following statements are required properties of standard mode.

- A game repository is usable from outside the Masonry source repository.
- `masonry.toml` is the source of truth for packaging and supported settings.
- Rust owns rules and authoritative game state exactly as it does today.
- Standard mode compiles and embeds one native Rust engine per application.
- The output is a game-specific application, not a runtime game selector.
- Masonry owns the bootstrap scene, Unity packages, project settings,
  Addressables settings, render pipeline, and standard content.
- One Masonry URP baseline supports both existing 2D-style and 3D content.
- A standard game cannot add C#, a custom Unity component (`MonoBehaviour`), a
  custom shader, a node-authored shader (`Shader Graph`), or an additional
  Unity package.
- Raw Unity serialization is never accepted as a project-settings override.
- Typed manifest fields expose the supported settings contract.
- The same logical asset declaration produces the catalog key and Rust binding.
- Ordinary build and run commands never rewrite checked-in generated Rust.
- The installed CLI defines every exact standard-setup version requirement.
- A rules crate that resolves incompatible Masonry crates fails before Unity or
  app assembly runs.
- Development automation is local, authenticated, limited in size and rate,
  and absent from a release shell.
- Each declarative scenario starts a fresh player process.
- Scenario input traverses Unity's normal Input System and Masonry action path.
- Scenarios observe pixels, player start/connection/exit events, and diagnostic
  records, not arbitrary Unity or Masonry object state.
- Failed builds use **atomic publication**: a complete verified replacement
  becomes visible in one filesystem operation, so no partial shell, content
  pack, or application overwrites a previously valid artifact.
- Existing advanced Unity projects remain supported by the reusable package.

The initial standard platform is native macOS. Windows, Linux, iOS, Android,
WebGL, HTTP-hosted rules, multiple render pipelines, and Apple's external app
review service are not part of this contract. The manifest avoids macOS-specific
concepts outside its explicit metadata and signing sections so later platform
adapters do not need to reinterpret game behavior.

## Developer experience

The CLI discovers a project by searching from the working directory toward the
filesystem root for `masonry.toml`. Commands that accept an explicit
`--manifest-path` do not search. Every relative path in the manifest resolves
from the directory containing that manifest, never from the caller's current
directory.

A **golden image** is a checked-in expected screenshot. Scenario comparison
produces an actual image and a highlighted difference image; accepting a
successful result explicitly replaces the golden. See
[Media capture and visual comparison](#media-capture-and-visual-comparison).

The project commands and their ownership are:

- `init [path]` creates the manifest, Rust rules package, generated-bindings
  destination, starter scenario, and ignore rules without overwriting work.
- `doctor` checks the manifest, Rust targets, bindings, shell, Unity, FFmpeg,
  and signing requirements, distinguishing required from optional tools.
- `generate` is the only command that rewrites checked-in Rust asset bindings.
  It reports asset-ID or kind changes before atomic replacement.
- `build [--release]` resolves the shell, validates bindings, builds changed
  rules/content, assembles the app, signs it, and prints its final path.
- `run [--release]` performs that incremental build and attaches terminal logs.
- `author` opens an **authoring workspace**, a disposable, non-playable Unity
  project used only to edit supported content. See
  [Authored content](#authored-content).
- `scenario run <name>` launches a fresh development player and runs TOML
  steps. `--all` runs all scenarios; `--accept` updates otherwise-passing
  golden captures.

Tool requirements are command- and cache-dependent. A missing optional tool
must not make unrelated commands fail.

A **cache hit** reuses an exact previously verified build result; a **cache
miss** must download or rebuild it. See
[Incremental build and cache semantics](#incremental-build-and-cache-semantics).

| Command | Rust toolchain | Unity | FFmpeg | Signing identity |
|---|---|---|---|---|
| `init` | no | no | no | no |
| `doctor` | inspected | inspected | inspected | inspected |
| `generate` | no | no | no | no |
| `build` | yes | only for a cache miss | no | optional |
| `run` | yes | only for a cache miss | no | optional |
| `author` | no | yes | no | no |
| `scenario run` | yes | only for a cache miss | only for video | no |

`build --release` always signs. **Ad hoc signing** creates a locally valid
signature without a trusted developer certificate and is the default.
**Developer ID signing** uses an Apple-issued certificate selected by name.
Both use the system `codesign` tool. A named identity must resolve through the
macOS Keychain certificate store, and its certificate and architecture
requirements are checked before expensive build work begins.

A representative session covers Rust-only creation and iteration, content,
authoring, visual evidence, and release signing:

```console
$ cargo masonry init hello-board && cd hello-board
Created masonry.toml, rules, generated bindings, and scenarios/smoke.toml
$ cargo masonry generate
Generated rules/src/masonry_assets.rs (0 game assets)
$ cargo masonry run
Shell: cache hit (CLI 0.2.0, macOS arm64, development)
Rules: built hello-board-rules; Content: standard assets only
Built target/masonry/debug/Hello Board.app; signing: ad hoc verified
$ cargo masonry run                 # after editing only Rust
Shell: hit; Rules: rebuilt; Content: hit; Unity Editor: not invoked
$ cargo masonry generate            # after declaring assets/Board.png
Generated rules/src/masonry_assets.rs (+1 texture: board)
$ cargo masonry run
Shell: hit; Rules: rebuilt; Content: rebuilt 1 Addressable entry
$ cargo masonry run                 # after changing only Board.png bytes
Shell: hit; Rules: hit; Content: rebuilt; Unity player: not rebuilt
$ cargo masonry author
Mounted ./content/authored; validation passed; opening Unity 6000.5.8f1
Play Mode is disabled; use `cargo masonry run` to execute the game
$ cargo masonry scenario run basic-pointer
Steps: 9 passed; golden mismatch 0.0184 > tolerance 0.0100
Actual: target/masonry/scenarios/basic-pointer/actual/basic-after.png
Diff: target/masonry/scenarios/basic-pointer/diff/basic-after.png
$ cargo masonry scenario run basic-pointer --accept
Scenario behavior passed; accepted goldens/basic-after.png
$ cargo masonry build --release
Shell: release hit; Rules: arm64+x86_64; Content: embedded
Signing: Developer ID Application: Example Studio (TEAMID); verified
Built target/masonry/release/Hello Board.app
```

`--accept` never changes a golden when startup, input, capture, diagnostics, or
another assertion failed. Release build stops after signing verification; it
does not notarize or staple the application.

Errors name the mismatched version, hash, or architecture and an actionable
recovery, for example:

```console
$ cargo masonry doctor --operation scenario --video
FAIL shell: no exact CLI 0.2.0/macOS arm64/development shell
FAIL Unity: 6000.5.8f1 is needed to build it; install or use --shell-source
FAIL FFmpeg: video needs ffmpeg and ffprobe; install or use --ffmpeg
FAIL bindings: board changed and marker_o was removed
Recovery: run `cargo masonry generate`, then rerun doctor
```

## Game manifest contract

The root TOML document rejects unknown fields. This prevents misspelled
settings from silently receiving defaults. The **manifest schema** is the
versioned set of allowed fields and meanings controlled by the installed CLI.
The top-level `schema` value selects it, not the Masonry package version.
Version 1 is the schema defined here.

Logical asset IDs are lowercase ASCII snake case and are unique across every
asset kind. Game IDs use **reverse-DNS syntax**, identifiers such as
`org.masonry.basic` ordered from organization to product. They are also the
default macOS **bundle identifier**, the stable operating-system identity of
the app. Display names are Unicode. Versions use three numeric components
because they must project predictably into macOS bundle metadata.

### Field semantics and defaults

`game.id` is permanent package identity. Changing it changes every game asset
key, the default bundle identifier, caches, bindings, and application identity.
`game.name` is display metadata and the default outer app name; it never
participates in an internal lookup. `game.version` supplies the short bundle
version. `macos.build` is a positive integer used as the macOS bundle version
and defaults to 1. Releases increase it explicitly; identical inputs remain
reproducible on clean machines and in concurrent builds.

`rules.manifest` and `rules.package` select exactly one Cargo package.
`rules.bindings` defaults to `rules/src/masonry_assets.rs`.
`rules.default_features` defaults to `true`, and `rules.features` defaults to
an empty list. Development uses Cargo's `dev` profile and release uses
`release`; the manifest cannot substitute arbitrary Cargo commands, targets,
environment variables, link flags, or output paths.

The CLI runs `cargo metadata` and requires exactly one resolved version of the
public `masonry` crate and the `masonry-native` adapter. Their package versions,
source kind, and source revision must equal the versions recorded by that CLI.
For path dependencies, the CLI hashes the same source files Cargo includes in
the package. A mismatch fails before compiling rules or invoking Unity.

`display.mode` is `windowed` or `borderless_fullscreen`. Width and height are
positive rendered pixel dimensions for windowed launch and scenario
capture. `resizable` defaults to `false`. The one `rendering.quality` value in
schema 1 is `standard`. Frame pacing is `vsync`, `unlimited`, or `fixed`; only
`fixed` requires a positive `target_fps`.

`diagnostics.level` is the minimum Rust and Unity-client storage level. The
toggle, and verbose message-logging fields affect development runtime
configuration.
They cannot cause their implementations to appear in a release shell.

`macos.bundle_identifier` defaults to `game.id`. Category and minimum version
default to Masonry release values recorded in the CLI schema. Architectures
default to the current machine for development and `arm64` plus `x86_64` for
release. An explicit architecture list is sorted and validated against the
rules target, shell, signing identity, and minimum supported macOS version.

Each `assets` entry has a stable `id`, a typed `kind`, and exactly one of a
`source` or `generate` table. Import options are legal only for a portable raw
source. Unity-serialized extensions identify authored content and require a
Unity **`.meta` file**, the companion file that preserves an asset's stable
Unity identifier and import metadata. They do not accept importer overrides.

`scenarios.directory` defaults to `scenarios`.
`default_timeout_seconds` defaults to 20, and `golden_tolerance` defaults to
zero. A scenario may reduce or override those values explicitly. The signing
table is optional and defaults to ad hoc identity `-` with no macOS capability
declaration file.

Paths are UTF-8 paths relative to the manifest directory. After conversion to
absolute paths and resolution of symbolic links, they must remain inside the
game root. Output and cache locations are CLI-owned and cannot be redirected.
Unknown tables, fields, enum values, and importer properties are errors.

The following limits complete the non-asset portion of schema 1:

- `game.id` has 3 to 255 ASCII characters and at least three dot-separated
  components. Components start with a lowercase letter and contain lowercase
  letters, digits, or hyphens. IDs beginning with `masonry.` are reserved.
- `game.name` has 1 to 128 Unicode scalar values and contains no control
  character, slash, colon, or leading/trailing whitespace. `game.version` is
  three dot-separated unsigned 32-bit integers.
- `rules.manifest`, `rules.bindings`, and `rules.package` are required to
  resolve to one Cargo dynamic-library (`cdylib`) target named
  `masonry_rules`. Feature names must exist in Cargo metadata. Duplicate
  features are errors.
- `display.width` and `height` are integers from 64 through 16384. Fullscreen
  uses the active display size at runtime; the declared values remain the
  deterministic scenario and fallback-window size.
- Fixed `target_fps` is an integer from 1 through 240. `target_fps` is absent
  for `vsync` and `unlimited`. The only schema-1 quality value is `standard`.
- Diagnostic levels are `trace`, `debug`, `info`, `warn`, and `error`.
  `console` defaults to `false`. `console_toggle` is one Unity Input System key
  name, defaults to `Backquote`, and is legal only when the console is enabled.
  `protocol_events` defaults to `false`.
- `macos.category` matches `public.app-category.` followed by lowercase ASCII
  letters and hyphens and is passed unchanged to `Info.plist`.
  `minimum_version` contains two or three unsigned numeric components and may
  not precede the shell's minimum. Architectures are a nonempty subset of
  `arm64` and `x86_64` without duplicates.
- `signing.identity` is `-` or a nonempty Keychain identity name.
  `signing.entitlements` is an optional `.entitlements` file below the game
  root. Schema 1 accepts only boolean
  `com.apple.security.app-sandbox` and
  `com.apple.security.network.client`; the latter requires the former. Unknown
  keys, nonboolean values, and `get-task-allow` fail validation.
- Scenario timeout is from 0.1 through 600 seconds. Golden tolerance is a
  finite number from 0 through 1. Scenario discovery includes only direct
  `.toml` children of the configured directory, sorted by file name.

`doctor` prints the effective defaults and allowed values, so validation never
depends on an unrecorded Unity or local-machine default.

### Basic manifest

```toml
schema = 1
[game]
id = "org.masonry.basic"
name = "Masonry Basic"
version = "1.0.0"
[rules]
manifest = "rules/Cargo.toml"
package = "masonry-basic-rules"
bindings = "rules/src/masonry_assets.rs"
default_features = true
features = []
[display]
width = 1280
height = 720
mode = "windowed"
resizable = true
[rendering]
quality = "standard"
frame_pacing = "vsync"
[diagnostics]
level = "info"
console = true
[scenarios]
directory = "scenarios"
default_timeout_seconds = 20
golden_tolerance = 0.01
[[assets]]
id = "gray"
kind = "material"
[assets.generate]
preset = "masonry-unlit"
color = "#808080ff"
[[assets]]
id = "yellow"
kind = "material"
[assets.generate]
preset = "masonry-unlit"
color = "#ffd633ff"
[[assets]]
id = "blue"
kind = "material"
[assets.generate]
preset = "masonry-unlit"
color = "#3388ffff"
```

The standard empty scene and default font are part of the Rust `standard`
module and do not need game declarations. The generated materials are game
assets because their colors are game configuration and changing them rebuilds
the game content pack.

### Tic-Tac-Toe manifest

Tic-Tac-Toe uses the same display, rendering, diagnostics, and scenario defaults
as Basic; this focused manifest shows its distinct identity, rules, and content:

```toml
schema = 1
[game]
id = "org.masonry.tictactoe"
name = "Masonry Tic Tac Toe"
version = "1.0.0"
[rules]
manifest = "rules/Cargo.toml"
package = "masonry-tictactoe-rules"
[[assets]]
id = "board"
kind = "texture"
source = "assets/Board.png"
[assets.import]
srgb = true
filter = "bilinear"
wrap = "clamp"
compression = "normal"
max_size = 2048
[[assets]]
id = "marker_x"
kind = "texture"
source = "assets/X.png"
[assets.import]
srgb = true
filter = "bilinear"
wrap = "clamp"
[[assets]]
id = "marker_o"
kind = "texture"
source = "assets/O.png"
[assets.import]
srgb = true
filter = "bilinear"
wrap = "clamp"
```

Omitted importer fields receive kind-specific documented defaults. The same
source file may not back two logical IDs because that makes import settings and
Unity asset-identifier ownership ambiguous.

### Raw asset declarations

```toml
[[assets]]
id = "turn_bell"
kind = "audio_clip"
source = "assets/audio/turn-bell.wav"
[assets.import]
load = "decompress_on_load"
compression = "pcm"
preload = true
[[assets]]
id = "knight"
kind = "prefab"
source = "assets/models/knight.glb"
[assets.import]
scale = 1.0
materials = "import"
animations = false
colliders = "none"
```

A raw model declared as `prefab` resolves to the imported model's root Unity
object (`GameObject`). A game that needs to add supported components, authored
colliders, or child configuration uses an authored prefab instead.

### Generated asset declarations

```toml
[[assets]]
id = "selection"
kind = "material"
[assets.generate]
preset = "masonry-lit"
color = "#44ccffff"
metallic = 0.0
smoothness = 0.25
surface = "opaque"
[[assets]]
id = "white_pixel"
kind = "texture"
[assets.generate]
preset = "solid-color"
color = "#ffffffff"
width = 4
height = 4
```

Version 1 generates only URP lit or unlit materials and solid-color textures.
Material `preset` is `masonry-lit` or `masonry-unlit`; `color` is required as
eight hexadecimal red, green, blue, and opacity digits. Lit materials allow
`metallic` and `smoothness` values from 0 through 1; their defaults are 0 and
0.5. `surface` is `opaque` or `transparent` and defaults to `opaque`. Unlit
materials allow only `color` and `surface`. Solid-color
texture `width` and `height` are powers of two from 1 through 1024 and default
to 4. It does not accept arbitrary shader names or generated shader properties.

An authored declaration uses the same typed asset contract but names a Unity
serialized source and carries its checked-in `.meta` file:

```toml
[[assets]]
id = "spark_burst"
kind = "particle_effect"
source = "content/authored/SparkBurst.prefab"
```

The content validator proves that the prefab root contains a supported particle
system and that every directly or indirectly referenced asset is allowed before
assigning the catalog key `org.example.game/particle_effect/spark_burst`.

### Development diagnostics

```toml
[diagnostics]
level = "debug"
console = true
console_toggle = "Backquote"
protocol_events = true
```

`console` and `protocol_events` configure only development builds. A release
build ignores their values and structurally omits those features; their presence
in the shared manifest is never permission to include dormant development code.

### Release signing

**Entitlements** are signed declarations of macOS capabilities, such as access
to protected operating-system services. A release may name a reviewed
entitlements file without placing any signing secret in the manifest.

```toml
[macos]
category = "public.app-category.games"
minimum_version = "14.0"
build = 42
[signing]
identity = "Developer ID Application: Example Studio (TEAMID)"
entitlements = "packaging/macos.entitlements"
```

The manifest names a keychain identity but contains no certificate, password,
Apple submission credential, or other secret. `identity = "-"`, or omission of
the signing section, selects ad hoc signing.

### Validation examples

The following declaration is invalid because source paths cannot escape the
game root after symlink resolution:

```toml
[[assets]]
id = "secret"
kind = "texture"
source = "../private/secret.png"
```

The following declaration is invalid because exactly one of `source` and
`generate` is required:

```toml
[[assets]]
id = "board"
kind = "texture"
source = "assets/Board.png"
[assets.generate]
preset = "solid-color"
color = "#ffffffff"
```

The following settings are incompatible because fixed pacing requires a target
and vsync does not accept one:

```toml
[rendering]
quality = "standard"
frame_pacing = "vsync"
target_fps = 60
```

The validator also rejects duplicate logical IDs, duplicate source files,
unsupported file extensions, case-insensitive path collisions, invalid bundle
identifiers, missing Cargo packages, unsupported features, unknown quality
presets, nonpositive dimensions, invalid colors, and release-only settings in
development-only sections.

### Configuration invalidation classes

Manifest fields are classified rather than conservatively rebuilding
everything.

- Rules fields invalidate the dylib and assembled app.
- Asset declarations and generated values invalidate bindings when identity or
  kind changes, content whenever source or import data changes, and the app.
- Display, rendering, and diagnostics values compile into runtime configuration
  and invalidate only config and app assembly when support for the requested
  setting is already compiled into the shell.
- Game name, version, macOS category, and minimum version invalidate metadata,
  signing, and app assembly.
- Development versus release selects a different shell cache entry and changes
  every downstream build profile.
- Signing changes invalidate only the final assembled application.
- No v1 field permits a custom shell variant. Unsupported shell-level settings
  produce a validation error and direct the game to advanced mode.

## Generated Rust asset bindings

Each game asset has one stable Addressables key:

`<game-id>/<kind>/<logical-id>`

The kind segments are `scene`, `prefab`, `particle_effect`, `material`,
`texture`, `audio_clip`, and `font`. Tic-Tac-Toe's `board` declaration maps to
`org.masonry.tictactoe/texture/board`. Moving `Board.png` does not change that
address. Renaming logical ID `board` does.

The generator normalizes logical IDs as Rust snake-case function names. Because
the manifest already requires lowercase snake case, normalization normally
preserves spelling. Any future spelling that would normalize to an existing
name is rejected rather than receiving a suffix.

The checked-in generated module begins with the CLI version, schema version,
game ID, and a **fingerprint**, a deterministic cryptographic hash of every
binding-relevant manifest field. Entries are sorted first by kind and then by
logical ID. Files generated on different machines from identical input are
byte-identical.

A representative generated surface is:

```rust
// Generated by cargo-masonry 0.2.0; do not edit.
// Manifest schema: 1; game: org.masonry.tictactoe
use masonry::TextureAddress;
pub fn board() -> TextureAddress {
    TextureAddress::new("org.masonry.tictactoe/texture/board")
}
pub fn marker_o() -> TextureAddress {
    TextureAddress::new("org.masonry.tictactoe/texture/marker_o")
}
pub fn marker_x() -> TextureAddress {
    TextureAddress::new("org.masonry.tictactoe/texture/marker_x")
}
```

The real file also carries the full fingerprint header. Accessors return the
existing owned typed address because `AssetAddress` contains a `String`; the
design does not require a new static address representation.

Basic uses generated and standard assets without strings:

```rust
use crate::masonry_assets;
use masonry::{MaterialAssignment, standard};
let scene = standard::empty_scene();
let font = standard::default_font();
let gray = MaterialAssignment::new(0, masonry_assets::gray());
```

Tic-Tac-Toe similarly uses typed texture accessors:

```rust
use crate::masonry_assets;
use masonry::ImageState;
let board = ImageState::new(masonry_assets::board(), 7.2, 7.2);
let marker = ImageState::new(masonry_assets::marker_x(), 2.25, 2.25);
```

Deleting `marker_x` removes its function the next time generation runs, so
rules code fails to compile until it is updated. Changing its kind changes the
return type and produces the same useful compiler boundary.

Build, run, author, and scenario commands compute the expected fingerprint and
compare it with the generated header and body. They do not regenerate in
memory and silently compile alternate bindings. A mismatch stops before Cargo:

```text
generated binding mismatch: marker_x changed from texture to material
run `cargo masonry generate`, review the Rust diff, and update callers
```

Standard asset accessors live in the versioned `masonry::standard` API rather
than being copied into every game module. Their catalog keys use the reserved
`masonry.standard` namespace. All standard manifests reject game IDs beginning
with `masonry.`. Source-tree tests use internal configuration outside the public
schema.

## Standard shell architecture

The root Unity project is the single authoritative shell source. Test,
performance, and capture content remains available to continuous integration
(CI) but is Editor-only or excluded from player compilation. Release validation
rejects its code modules and Unity asset IDs in the build report.

One bootstrap scene owns the production runner and MessagePack encoder and
decoder, standard catalog and empty content scene, runtime configuration loader,
game-catalog validation, native ABI verification, and diagnostics. The standard
scene satisfies the protocol's
nonempty scene requirement while Rust creates the world. Authored scenes remain
optional game Addressables.

One URP renderer supports the existing cameras, lights, primitives, image
quads, world text, particles, animation, and audio. It retains shaders required
by standard and generated assets. Custom pipeline assets and render features
are not standard settings.

Development shells include the control listener, simulated input devices,
recorder, image-comparison support, diagnostics, and console. Release shells
exclude those code modules; a runtime flag is insufficient. Verified shell
metadata binds CLI and source commit/hash, Unity and package-lock versions, ABI
and format versions, architectures, shell profile, catalog hash, and immutable
shell-tree hash. This complete tuple is the **shell identity**; every field must
match for cache reuse.

The authoritative Unity package-lock hash is the SHA-256 hash of the root
project's `Packages/manifest.json` and `Packages/packages-lock.json` bytes.
Changing either file creates a new shell identity; a display version in a
package file never substitutes for that hash.

The **immutable shell-tree hash** covers sorted relative paths, file modes, and
bytes for every file assembly may not change. It excludes `shell.json`, code
signature directories, and signature data referenced by the macOS executable's
Mach-O `LC_CODE_SIGNATURE` command; all other executable bytes remain covered.
`shell.json` contains those fields, a SHA-256 hash for every immutable shell
file, and the explicit list of paths assembly may change. That list contains
only the outer app name, supported `Info.plist` keys, code signatures, the rules
dylib slot, compiled game config, and game catalog/bundle locations. Every
other file remains byte-identical through assembly. The final build report
records the `shell.json` hash and every allowed change, proving which shell was
used after the application is renamed and re-signed.

### Shell resolution

Resolution order is exact verified user cache, optional checksummed published
artifact, then a Unity build from the exact local source. Corrupt cache entries
are
**quarantined**, moved aside with their failure reason instead of being reused.
Downloads are verified before atomic installation; HTTPS encryption alone is
insufficient.
Offline mode permits cache or explicit local source only.

Official shell discovery uses a signed JSON index. Each CLI release embeds one
or more trusted Ed25519 public keys; **Ed25519** is the public-key signature
algorithm used to prove that Masonry published the index. An index entry names
the exact shell identity, artifact URL, byte length, SHA-256 archive checksum,
`shell.json` checksum, signing-key ID, and expiry. Key additions ship in a CLI
release signed by an already trusted release process. Revoked key IDs are
embedded in later CLIs and are never accepted merely because an old index names
them.

An installed official cache entry retains the signed index entry, index
signature, signing-key ID, archive checksum, and `shell.json` checksum. Every
cache lookup rechecks file hashes and rejects a key ID revoked by the installed
CLI before considering the entry a hit. Index expiry controls new downloads;
it does not invalidate an already installed, otherwise valid cache entry. This
permits deterministic offline reuse while allowing a later CLI to revoke a
compromised publisher key. Locally built entries record `origin = "local"` and
do not require proof of publication by Masonry.

Download verification occurs in this order: verify the index signature and
expiry; require an exact identity entry; download to a private temporary path;
verify byte length and archive checksum; extract without absolute paths,
symbolic links, or traversal; verify `shell.json` checksum and identity; verify
every immutable file hash; verify code signature, architecture, and required
symbols; then atomically install. Any failure quarantines only the temporary
download. A downloaded shell is never executed to discover its identity.

Local source builds are trusted as local input rather than Masonry-published
artifacts. Their source version is the Git commit plus a SHA-256 hash over
sorted relative paths, file modes, and bytes for tracked and nonignored
untracked shell inputs. Unity's `Library`, `Temp`, logs, build outputs, and user
settings are excluded. The resulting `shell.json` records that it is local and
is not eligible for publication under an official signing key.

`--rebuild-shell` forces source build without replacing a valid entry until the
new shell passes verification. `--shell-source` includes the source commit and
a hash of uncommitted changes in the shell identity. A filesystem lock for each
shell identity lets concurrent callers reuse a winner and is recoverable only
after its owner is confirmed dead.

## Game app assembly and signing

Assembly copies a verified shell into a game temporary directory and never
edits the cached source. It then:

1. Install the architecture-compatible `libmasonry_rules.dylib`.
2. Install the compiled game configuration.
3. Install or reference the game catalog and bundles.
4. Patch supported `Info.plist` macOS application metadata.
5. Rename the outer app, remove its old signature, and sign embedded code.
6. Sign the app with entitlements and verify signature, ABI, and hashes.
7. Atomically replace the published output.

The executable may retain a stable internal file name. Display metadata never
selects runtime files; fixed shell-relative locations do.

The game-owned portion of a release app is represented by:

```text
Masonry Tic Tac Toe.app/
  Contents/Info.plist                         patched game metadata
  Contents/PlugIns/libmasonry_rules.dylib    game rules engine
  Contents/Resources/Data/StreamingAssets/Masonry/
    game-config.msgpack                      compiled runtime settings
    game-catalog.json                        game Addressables catalog
    game-catalog.hash                        verified catalog checksum
    bundles/...                              embedded game content
```

Development bundles may be sibling files selected by compiled config; release
bundles are embedded. Identity `-` means ad hoc signing. Named identities
resolve through Keychain, and the validated entitlements file joins the
assembly fingerprint. Failure preserves the prior app. The contract ends at
structural and `codesign` verification. **Notarization**, Apple's separate
malware-review and ticket service, remains outside this design.

Every assembled app is signed. Development `build` and `run` use ad hoc signing
without release entitlements, regardless of a configured Developer ID. Release
uses the configured identity and entitlements, or ad hoc signing when identity
is omitted or `-`. The phrase "signing identity optional" therefore means a
private certificate is optional, not that the signature step is skipped.

## Incremental build and cache semantics

An **artifact domain** is a separately cached build result with its own inputs,
validation, and publication boundary. There are six:

- **Standard shell:** material inputs are the authoritative Unity source,
  package lock, Unity version, target architecture, shell profile, native ABI,
  and standard/game catalog format version 1. Only an exact published or local
  match is reusable. A miss requires Unity and invalidates final assembly and,
  when catalog format version 1 changes, game content.
- **Rules dylib:** material inputs are the selected Cargo package, features,
  exact Masonry dependency versions, target, Cargo profile, and Rust sources.
  It never requires Unity. A change invalidates only the assembled app.
- **Generated bindings:** material inputs are manifest asset identities,
  kinds, game ID, schema, and generator version. Commands validate this domain
  but only `generate` publishes it. A mismatch blocks rules compilation and
  content work rather than creating an unreviewed source change.
- **Game content pack:** material inputs are asset declarations, source bytes,
  importer settings, authored `.meta` files and dependencies, and exact Unity
  version and content format. A miss requires Unity. It invalidates the app,
  but never the standard shell or rules dylib.
- **Compiled configuration and app metadata:** material inputs are runtime
  manifest fields, bundle metadata, shell profile, scenario automation
  settings, and catalog/bundle hashes. Compilation does not require Unity. A
  change invalidates
  final assembly without rebuilding rules or content unless the field is also
  assigned to one of those domains.
- **Final assembly:** material inputs are immutable hashes of the selected
  shell, dylib, content, config, metadata, entitlements, signing identity, and
  assembly-tool version. It never requires Unity. It is reusable only as a
  verified finished application with the same signature requirements.

Every domain fingerprints all material inputs and exact compatibility
metadata. An output receives metadata marking it complete only after validation
and atomic publication. Abandoned temporary output is removable only after its
owner is confirmed dead. Failed rebuilds never delete the last valid result.

| Change | Shell | Bindings | Rules | Content | Assemble |
|---|---:|---:|---:|---:|---:|
| Rust source | hit | check | rebuild | hit | rebuild |
| Window size | hit | check | hit | hit | rebuild |
| Generated material color | hit | check | hit | rebuild | rebuild |
| Asset ID/kind | hit | out of date | after generate | rebuild | rebuild |
| PNG bytes | hit | check | hit | rebuild | rebuild |
| Authored prefab | hit | check | hit | rebuild | rebuild |
| Package upgrade | rebuild | out of date | rebuild | rebuild | rebuild |
| Profile change | new key | check | rebuild | hit | rebuild |
| Signing identity | hit | check | hit | hit | re-sign |

## Content and Addressables design

Raw imported and portable authored content produce one game-specific catalog
separate from the shell catalog.

### Raw content

**Raw content** is an ordinary portable file that Unity imports using only the
curated settings in `masonry.toml`.

Version 1 imports supported textures, audio, and models. Staging uses a stable
Unity **GUID**, the identifier stored in Unity asset references, derived from
game ID, kind, and logical ID rather than source path or bytes. Curated texture,
audio, and model settings are exhaustive:

- Texture sources are `.png`, `.jpg`, or `.jpeg` and declare kind `texture`.
  `srgb` defaults to `true`; `filter` is `nearest`, `bilinear`, or `trilinear`
  and defaults to `bilinear`; `wrap` is `clamp`, `repeat`, or `mirror` and
  defaults to `clamp`; `compression` is `none`, `normal`, or `high` and
  defaults to `normal`; `max_size` is a power of two from 32 through 8192 and
  defaults to 2048. **Mipmaps**, smaller precomputed texture levels used when an
  object is distant, default to `false` and may be enabled explicitly.
- Audio sources are `.wav`, `.aiff`, or `.ogg` and declare kind `audio_clip`.
  `load` is `decompress_on_load`, `compressed_in_memory`, or `streaming` and
  defaults to `decompress_on_load`; `compression` is `pcm` or `vorbis` and
  defaults to `vorbis`; `quality` is from 0 through 1 and defaults to 0.7;
  `preload` defaults to `true`; `force_mono` defaults to `false`. Streaming
  audio cannot use PCM or preload.
- Model sources are `.fbx`, `.obj`, or `.glb` and declare kind `prefab`.
  `scale` is greater than 0 and at most 1000 and defaults to 1;
  `materials` is `none` or `import` and defaults to `import`; `animations`
  defaults to `false`; `colliders` is `none`, `mesh`, or `convex` and defaults
  to `none`. An `.obj` source cannot enable animations.

No other raw import fields or source extensions are accepted. Unity imports a
staged copy without touching game source. The builder verifies the imported
Unity type before publishing an entry and records the effective defaults in the
content report.

### Authored content

**Authored content** is a Unity-serialized asset edited in the disposable
authoring workspace and committed together with its `.meta` file.

Authored scenes, prefabs, materials, animation, particles, audio, textures,
models, and TMP fonts retain Unity serialization and `.meta` files. A disposable
workspace mounts only that game-owned content. Dependency validation permits
the authored set, a fixed list of standard assets, and supported Unity
built-ins.
Scripts, unknown MonoBehaviours, shaders, Shader Graphs, extra packages, test or
Editor assets, and external paths fail with their dependency chain. Play Mode
is disabled; the packaged player is the only execution environment.

Only scenes, prefabs, particle-effect prefabs, materials, textures, audio
clips, and TMP font assets may be catalog roots. Animation clips, controllers,
models, meshes, and sprites are allowed only as dependencies of those roots;
they do not receive logical IDs or generated Rust accessors in schema 1.

The public kind and type mapping is exact:

- `scene` produces Rust `SceneAddress` and Unity `SceneInstance`.
- `prefab` produces `PrefabAddress` and Unity `GameObject`.
- `particle_effect` produces `ParticleEffectAddress` and a `GameObject` whose
  root contains a Unity `ParticleSystem`.
- `material` produces `MaterialAddress` and Unity `Material`.
- `texture` produces `TextureAddress` and Unity `Texture2D`.
- `audio_clip` produces `AudioClipAddress` and Unity `AudioClip`.
- `font` produces `FontAddress` and Unity `TMP_FontAsset`.

An authored declaration's extension and inspected root must agree with its
kind. There is no generic object kind and no caller-supplied Unity type name.

### Catalog construction and loading

Catalogs expose only generated logical keys: no path/GUID aliases or labels.

- `board` maps `board()` to
  `org.masonry.tictactoe/texture/board`, a `Texture2D` from
  `assets/Board.png`.
- `marker_x` maps `marker_x()` to
  `org.masonry.tictactoe/texture/marker_x`, a `Texture2D` from
  `assets/X.png`.
- `gray` maps `gray()` to `org.masonry.basic/material/gray`, a generated
  `Material`.
- `turn_bell` maps `turn_bell()` to
  `org.example.game/audio_clip/turn_bell`, an `AudioClip` from the declared
  WAV source.
- `knight` maps `knight()` to `org.example.game/prefab/knight`, a
  `GameObject` prefab imported from the declared GLB source.

Runtime verifies standard then game catalog, rejecting reserved or duplicate
keys, unknown kinds, format-version or checksum mismatch, and incorrect Unity
types before any rules call. An **asset lease** is the existing handle that
keeps an Addressable loaded until every user releases it. Its behavior remains
the one defined in the
[Masonry Technical Design](technical-design.md); there are no implicit
loads, retries, catalog updates, or second runtime cache.

## Runtime bootstrap and failure behavior

Player startup has one observable order:

1. Read and validate compiled game configuration.
2. Initialize and validate the standard catalog.
3. Initialize and validate the game catalog when one is declared.
4. Start diagnostics and automation features in a development shell.
5. Load the native library and verify architecture, symbols, and ABI v2.
6. Create the rules engine.
7. Configure and connect the Masonry runner.
8. Report `connected`, then `ready`, to the terminal and the
   **development control plane**, the authenticated local automation protocol
   defined in [Development control plane](#development-control-plane).
9. Poll input, responses, rules, and diagnostics with independent budgets.

No rules entry point runs before catalog validation. `ready` means the initial
snapshot was applied, the Unity client completed a rendered frame, and requested
development control is usable. It does not mean the game has become idle after
all possible future work.

| Failure | Player behavior | CLI or scenario behavior |
|---|---|---|
| Bad config | fatal screen and nonzero exit | fail build or launch |
| Missing content | fatal screen and asset error | fail launch |
| Wrong asset type | fatal before rules load | report key and types |
| Bad dylib | fatal before engine creation | fail launch |
| ABI mismatch | fatal before engine creation | report identities |
| Engine creation error or panic | contained fatal, nonzero exit | fail launch |
| Initial snapshot failure | runner fatal, nonzero exit | fail readiness wait |
| Requested control listener failure | fatal in development | fail launch |
| No control requested | listener does not start | direct launch continues |
| Diagnostic overflow | warning; game continues | strict assertion may fail |
| Forbidden release surface | app is never published | fail release build |

When enough Unity runtime is available, fatal startup errors show a minimal
in-player diagnostic with a stable error code and the log location. Otherwise
the process writes the structured record to stderr and the player log. Control
requests already accepted receive a failure acknowledgement before shutdown's
fixed time limit when transport is still usable.

## Native diagnostics contract

ABI v2 is incompatible with v1. It retains the v1 engine behavior and status
numbers, replaces the required marker with `masonry_abi_v2`, and has this exact
C surface:

```c
typedef struct MasonryEngine MasonryEngine;
typedef struct { uint8_t *data; uint64_t length; } MasonryBuffer;
int32_t masonry_engine_create(
    MasonryEngine **out_engine, MasonryBuffer *out_error);
void masonry_engine_destroy(MasonryEngine *engine);
int32_t masonry_connect(
    MasonryEngine *engine, const uint8_t *data, uint64_t length,
    MasonryBuffer *out_buffer);
int32_t masonry_submit(
    MasonryEngine *engine, const uint8_t *data, uint64_t length,
    MasonryBuffer *out_buffer);
int32_t masonry_poll(
    MasonryEngine *engine, MasonryBuffer *out_buffer);
int32_t masonry_diagnostics_poll(MasonryBuffer *out_buffer);
void masonry_buffer_free(MasonryBuffer buffer);
void masonry_abi_v2(void);
```

The CLI requires the v2 marker and every v2 export before launch; a library
that exposes only `masonry_abi_v1` is rejected. Diagnostic polling has no engine
parameter so it can return records emitted before creation or after destruction
of the one engine.

A **diagnostic queue** is a fixed-capacity list owned by the native adapter and
kept separate from gameplay responses. Unity **polls** it by repeatedly asking
for the next record. Diagnostics cannot affect commands, batches, actions, or
response ordering. Each returned buffer is one MessagePack map with exactly
these fields:

- Monotonically increasing `sequence` within the loaded rules library.
- `level`: trace, debug, info, warn, or error.
- Short stable `target`, such as `tictactoe.ai`.
- A size-limited UTF-8 `message` intended for a human.
- A sorted map of size-limited UTF-8 string fields.
- `elapsedMicros`: optional unsigned monotonic microseconds since queue
  initialization.

Targets are at most 128 UTF-8 bytes. Messages are at most 4096 bytes. A record
has at most 32 fields; keys are at most 64 bytes, values are at most 1024 bytes,
and the encoded record is at most 16 KiB. Oversized emission is counted as a
drop rather than truncated. The queue holds 1024 records and at most 4 MiB of
encoded data.

Any Rust thread may emit without calling or blocking Unity. Emission locks the
sequence counter and queue briefly. If earlier drops are pending and space is
available, the adapter first assigns the next sequence to one synthetic
overflow record and enqueues it. It then assigns the following sequence to the
current ordinary record and either enqueues or counts that record as dropped.
Synthetic records therefore consume sequence numbers, polling never creates a
sequence, and the example below can report dropped 1053 through 1089 with the
synthetic record numbered 1090.

Status `0` (`OK`) returns one nonempty MessagePack buffer. Status `1`
(`NO_MESSAGE`) returns `{NULL, 0}`. Statuses `2` (`INVALID_ARGUMENT`), `3`
(`ENGINE_ERROR`), and `4` (`PANIC`) return optional UTF-8 error text exactly as
the existing ABI does. The function initializes output to `{NULL, 0}`. Unity
copies every nonempty output into managed memory inside a `try` block and frees
the native buffer exactly once with `masonry_buffer_free` in `finally`. It
decodes only the managed copy after the free; freeing `{NULL, 0}` is a no-op.

The queue is created when the dylib initializes and remains until process
shutdown. Engine create and destroy do not clear it. A new process or newly
loaded library starts sequence at 1. During normal frames Unity polls at most
64 records or one millisecond, whichever comes first. At shutdown it destroys
the engine, drains for at most 100 milliseconds, emits a final dropped-count
summary to the player log when needed, and then unloads the library.

A **correlation ID** is a random 128-bit lowercase hexadecimal value created for
each player launch. The CLI supplies it with control launch data; a directly
opened development app creates its own. It is not part of native MessagePack or
game state. Unity attaches it while adding native and Unity records to the
unified diagnostic store, so terminal output, console rows, assertions, and
capture metadata can identify the same run.

A panic inside diagnostic emission drops that record and is caught inside Rust.
A panic from the poll export returns `PANIC`, disables further diagnostic
polling, and leaves gameplay running. A malformed managed MessagePack copy
writes a Unity error and is
dropped; three malformed records disable diagnostic polling. Either condition
fails a scenario that requested complete diagnostic evidence but cannot change
game commands or authoritative state.

Rust code emits through a Masonry-owned logging API:

```rust
masonry_native::diagnostic!(
    Info,
    "tictactoe.ai",
    "computer move applied",
    "cell" => cell.to_string(),
    "session" => self.session_id.to_string(),
);
```

A readable form of the encoded record is:

```json
{
  "sequence": 42,
  "level": "info",
  "target": "tictactoe.ai",
  "message": "computer move applied",
  "fields": { "cell": "4", "session": "…" },
  "elapsedMicros": 812443
}
```

An overflow warning is distinguishable from an application warning:

```json
{
  "sequence": 1090,
  "level": "warn",
  "target": "masonry.diagnostics",
  "message": "diagnostic records dropped",
  "fields": {
    "count": "37",
    "firstSequence": "1053",
    "lastSequence": "1089"
  }
}
```

Development defaults to `info` and may request `debug`. Release defaults to
`warn`, has no overlay/control stream, and may compile lower levels out.

## Development console

The console dependency is pinned to the revision and license linked in Related
information. It is a development-only view over Masonry's size-limited Unity,
Rust, protocol, player-state, and capture records. Scenarios use that store, not
rendered rows. Console focus suppresses game keyboard actions, and Unity UI hit
testing blocks world input. Automatic discovery of methods and arbitrary
evaluation are disabled. Curated commands are `help`, `status`, `clear`,
`level`, `capture-png`, and `quit`.

A representative session is:

```text
[info] masonry.shell: game config loaded game=org.masonry.tictactoe
[info] masonry.native: ABI v2 verified architecture=arm64
[info] masonry.runner: connected session=2d1c…
[info] tictactoe.ai: computer move applied cell=4
> level warn
console level set to warn
[error] masonry.protocol: response rejected reason="unknown asset key"
> capture-png console-error.png
[info] masonry.capture: PNG written path=console-error.png
```

Release validation proves console package, prefab, commands, and toggle handling
are absent.

## Development control plane

The control plane uses **framed requests**, JSON messages preceded by their
byte length, over **loopback TCP**, a network connection reachable only from
the same computer. The CLI chooses an unused port and passes a random
per-process **capability token**, a secret required to control that player,
outside the manifest. The first message authenticates and selects protocol 1:

Each frame begins with a four-byte unsigned **big-endian** JSON byte length,
most-significant byte first, not including the prefix. Exactly that many UTF-8
bytes follow. The JSON root is an object. The first frame has no request ID and
is exactly the hello shape below apart from token value:

```json
{"type":"hello","protocol":1,"token":"b1e7…"}
```

Success returns this envelope; authentication failure closes the socket without
a JSON response or diagnostics:

```json
{
  "type": "hello_ok",
  "protocol": 1,
  "shellHash": "5e1a…",
  "processId": 48201
}
```

Tokens are redacted. After hello, request frames contain exactly `id`, `method`,
and `params`. IDs are positive unsigned 64-bit integers that strictly increase
on one connection. Responses contain the same `id` and exactly one of `result`
or `error`; errors contain stable `code`, human `message`, and optional `data`.
Server events contain `event` and event-specific fields but no ID. Frames are at
most 64 KiB with 32 outstanding requests, plus the input and media limits below.

`cargo masonry run` and every scenario launch request the listener. The CLI
passes port and output-root arguments and sends the token through a one-way
operating-system channel visible only to the launched child process. The token
does not appear in the manifest, process arguments, or logs. Failure to start
is fatal for those commands. A development app opened directly receives no
control arguments; it continues without a warning or
listener. Release code contains no branch that can consume them.

The CLI creates a private output root for each controlled process. A run uses
`target/masonry/run/<process-id>/`. A scenario first writes below a unique
temporary directory and, while holding a per-scenario lock, atomically publishes
it as `target/masonry/scenarios/<scenario-name>/`. Every control `path` is
relative to that private root. Absolute paths, `..`, symbolic-link traversal,
control characters, and normalized paths longer than 1024 UTF-8 bytes fail
before media work begins.

Protocol 1 accepts at most 240 requests per second and at most 64 immediately
queued requests. It allows five held mouse buttons, 32 held keyboard keys, one
pending PNG, and one
active video. A PNG path is required. Video duration is from 0.1 through 300
seconds when present, frame rate is from 1 through 120, and only one start or
stop transition may be outstanding. One PNG may run during a video; a second
PNG or video is rejected. Graceful player shutdown has two seconds; encoder
finalization has five seconds. The CLI then terminates only the process IDs it
started and reports forced cleanup.

The method set is `status`, pointer move/down/up, key down/up,
`diagnostics.subscribe`, PNG capture, video start/stop, and `shutdown`.
Diagnostic subscription streams the same ordered records used by assertions;
it does not drain the native or Unity diagnostic store. Its optional
`afterSequence` replays records from the Unity client's rolling 4096-record
history before live events. When the requested sequence is older than retained
data, the first event is
`diagnostics.gap` with first available and requested sequences.

Only one authenticated client may control a player. A second valid hello gets a
`hello_error` envelope with code `session_busy` and closes. After disconnect
cleanup finishes, the same token may authenticate a new connection; request IDs
restart at 1 and acknowledgements are not replayed. Unknown methods return
`unknown_method`, duplicate or nonpositive IDs return `invalid_request_id`, and
malformed parameters return `invalid_params`. These errors do not change input,
media, or session state. A frame that exceeds the limit or fails JSON decoding
closes only that authenticated connection after releasing its held inputs.

Pointer movement uses normalized top-left coordinates:

```json
{"id":1,"method":"input.pointer.move","params":{"x":0.50,"y":0.62}}
{"id":1,"result":{"accepted":true}}
```

Button transitions are explicit and balanced:

```json
{"id":2,"method":"input.pointer.down","params":{"button":"left"}}
{"id":2,"result":{"accepted":true}}
{"id":3,"method":"input.pointer.up","params":{"button":"left"}}
{"id":3,"result":{"accepted":true}}
```

PNG capture acknowledges completion, not merely scheduling:

```json
{"id":4,"method":"capture.png","params":{"path":"after.png"}}
{"id":4,"result":{"path":"after.png","width":1280,"height":720}}
```

Video has explicit start and completion operations so failures are observable:

`capture.video.start` requires `path` and `frameRate`. `durationSeconds` is
optional: when present, the player stops automatically and emits completion;
when absent, `capture.video.stop` is required.

```json
{
  "id": 5,
  "method": "capture.video.start",
  "params": {
    "path": "play.mp4",
    "durationSeconds": 5,
    "frameRate": 30
  }
}
{"id":5,"result":{"recording":true,"encoderPid":48211}}
{"event":"capture.video.complete","path":"play.mp4","frames":150}
```

An open-ended recording is completed by an acknowledged stop request:

```json
{"id":6,"method":"capture.video.stop","params":{}}
{"id":6,"result":{"path":"play.mp4","frames":150,"complete":true}}
```

Keyboard transitions use Unity Input System key names and the same balanced
state rules as pointer buttons:

```json
{"id":7,"method":"input.key.down","params":{"key":"Space"}}
{"id":7,"result":{"accepted":true}}
{"id":8,"method":"input.key.up","params":{"key":"Space"}}
{"id":8,"result":{"accepted":true}}
```

The JSON event named `lifecycle` reports player connection and readiness state
without object inspection:

```json
{"event":"lifecycle","state":"connected"}
{"event":"lifecycle","state":"ready"}
{"event":"lifecycle","state":"idle"}
```

An invalid transition returns a structured error and keeps the connection
usable:

```json
{"id":9,"method":"input.pointer.up","params":{"button":"left"}}
{
  "id": 9,
  "error": {
    "code": "invalid_input_transition",
    "message": "left button is not held"
  }
}
```

Disconnect releases synthetic input, stops media, closes encoder input, and
reports incomplete work. Graceful shutdown waits a fixed timeout before
terminating owned subprocesses. The protocol cannot enumerate or mutate scenes,
objects, components, or rules.

## Declarative scenario contract

Scenario files are TOML with schema 1. Steps execute in document order. Every
step inherits the scenario timeout unless it specifies a smaller positive
timeout. Time is real wall time; frame waits observe Unity frames but do not
virtualize Unity or Rust clocks.

Top-level metadata contains `name`, optional `description`,
`timeout_seconds`, `profile`, `artifact_prefix`, and optional window
dimensions. The scenario build profile defaults to `development`; schema 1
rejects `release`
because scenarios require the development control plane. `artifact_prefix` is
the base name for automatic `<prefix>-report.json`, `<prefix>-player.log`, and
`<prefix>-diagnostics.msgpack` files. It defaults to the normalized scenario
name and must be a safe relative path component. It does not rewrite explicit
capture paths.

`name` is a lowercase letter followed by at most 63 lowercase letters, digits,
or hyphens and is unique within the scenario directory. Description is at most
1024 Unicode scalar values. A scenario timeout replaces the manifest default
and may be any value from 0.1 through 600 seconds. Window dimensions inherit the
manifest and use the same numeric limits. The runner rejects unknown top-level
or step fields.

Wait steps cover player `ready`, Masonry `connected`, Unity-client `idle`, a
real-time
duration, a frame count, or a matching structured diagnostic. Input steps are
normalized pointer movement, balanced pointer-button transitions, or balanced
keyboard-key transitions. Capture steps create PNG or video artifacts. Exit
steps declare whether a zero or nonzero process exit is expected. An unexpected
exit fails the active step immediately.

The complete schema-1 step set is:

Every step may declare a unique `id` using the scenario-name character rules.
An omitted ID is addressed only by its one-based step number and cannot be used
by `since`. Step IDs do not change execution order.

- `wait` requires `for = "connected"`, `"ready"`, or `"idle"`.
- `wait-time` requires finite `seconds` greater than 0.
- `wait-frames` requires `frames` from 1 through 1,000,000.
- `wait-log` requires at least one diagnostic matcher and waits for a matching
  record rather than consuming it.
- `pointer-move` requires finite `x` and `y` values from 0 through 1, measured
  from the top-left of the rendered image.
- `pointer-down` and `pointer-up` require `button` equal to `left`, `right`,
  `middle`, `back`, or `forward`. Repeating a down or releasing an unheld
  button fails the step.
- `key-down` and `key-up` require one Unity Input System key name. Repeating a
  down or releasing an unheld key fails the step.
- `capture-png` requires an output `path`. Optional `golden` enables comparison.
  Optional `tolerance` replaces the scenario or manifest value and is from 0
  through 1. A mismatch writes `diff/<capture-file-name>` below the run output
  root.
- `capture-video` requires `path`, `duration_seconds` from 0.1 through 300, and
  `frame_rate` from 1 through 120. `pointer_overlay` defaults to `false`.
- `assert-log` and `assert-log-absent` inspect accumulated records from
  `since = "scenario-start"` or a named earlier step. They require at least one
  matcher.
- `wait-exit` requires an integer `code` from 0 through 255 and may include
  diagnostic matchers. It must be the final step.

Diagnostic matchers use exact `level` and `target`, case-sensitive `contains`
on the human message, and exact string equality for optional fields. Omitted
matcher properties do not constrain the match. An assertion never clears
records. A normal
scenario that has no `wait-exit` requests graceful shutdown after its final
step and expects exit code 0.

Every step may set `timeout_seconds` no greater than the remaining scenario
timeout. It replaces the inherited per-step limit; it does not extend the
scenario deadline. Output `path` values resolve below the private run output
root. `golden` resolves below the game root and must identify a `.png` file.
Neither path may be absolute, traverse a symbolic link, contain `..`, or escape
its root after normalization.

Timeouts are hard bounds. A step timeout reports that step and its last
acknowledgement; a scenario timeout interrupts the active step. Both trigger
balanced release of held inputs, encoder finalization or cancellation, player
shutdown, and retention of the failure report. Cleanup has its own fixed time
allowance and cannot turn the original failure into success.

The runner launches a fresh development player for every scenario. It creates
a unique control token and artifact directory, verifies window dimensions,
records player and encoder process identities, and cleans them before returning.

### Basic pointer scenario

```toml
schema = 1
name = "basic-pointer"
timeout_seconds = 20
profile = "development"
artifact_prefix = "basic-pointer"
[window]
width = 1280
height = 720
[[steps]]
action = "wait"
for = "ready"
[[steps]]
action = "assert-log"
target = "masonry.runner"
contains = "connected"
[[steps]]
action = "pointer-move"
x = 0.33
y = 0.53
[[steps]]
action = "wait-frames"
frames = 2
[[steps]]
action = "pointer-down"
button = "left"
[[steps]]
action = "pointer-up"
button = "left"
[[steps]]
action = "wait"
for = "idle"
[[steps]]
action = "capture-png"
path = "actual/basic-after.png"
golden = "goldens/basic-after.png"
tolerance = 0.01
[[steps]]
action = "assert-log-absent"
level = "error"
since = "scenario-start"
```

The click traverses the simulated mouse, Unity Input System, collider hit test,
Masonry action, Rust engine, response, and visible **tween**, an animation that
interpolates values over time. `idle` means the Unity client
has no active preparation, batch, or command operation; it reveals no game
object data.

### Tic-Tac-Toe scenario

```toml
schema = 1
name = "tictactoe-player-and-computer"
timeout_seconds = 20
profile = "development"
artifact_prefix = "tictactoe-player-and-computer"
[window]
width = 1280
height = 720
[[steps]]
action = "wait"
for = "ready"
[[steps]]
action = "pointer-move"
x = 0.39
y = 0.42
[[steps]]
action = "pointer-down"
button = "left"
[[steps]]
action = "pointer-up"
button = "left"
[[steps]]
action = "wait-log"
target = "tictactoe.ai"
contains = "computer move applied"
timeout_seconds = 3
[[steps]]
action = "capture-png"
path = "actual/tictactoe-after.png"
golden = "goldens/tictactoe-after.png"
tolerance = 0.01
```

The diagnostic wait avoids encoding computer-response timing into the
scenario. The migrated Tic-Tac-Toe rules select computer moves deterministically
for all builds, so identical input produces stable pixels without a
scenario-only state hook.

### Video scenario

```toml
schema = 1
name = "basic-animation-video"
timeout_seconds = 20
profile = "development"
artifact_prefix = "basic-animation-video"
[[steps]]
action = "wait"
for = "ready"
[[steps]]
action = "capture-video"
path = "video/basic-animation.mp4"
duration_seconds = 5
frame_rate = 30
[[steps]]
action = "assert-log-absent"
level = "error"
since = "scenario-start"
```

The capture-video step completes only after encoding and inspection succeed.
It produces a metadata file beside the video with dimensions, frame count,
duration, encoding format, CLI/shell versions, game/scenario IDs, and content
hashes.

### Expected startup failure

```toml
schema = 1
name = "rejects-invalid-abi"
timeout_seconds = 10
profile = "development"
artifact_prefix = "rejects-invalid-abi"
[[steps]]
action = "wait-exit"
code = 1
target = "masonry.native"
contains = "ABI v2 is required"
```

Failure tests may override an assembled app only through a CLI test option
unavailable to ordinary game manifests. A scenario cannot replace its own
dylib through the control protocol.

### Negative log assertion

```toml
schema = 1
name = "clean-startup"
timeout_seconds = 10
profile = "development"
artifact_prefix = "clean-startup"
[[steps]]
action = "wait"
for = "ready"
[[steps]]
action = "key-down"
key = "Space"
[[steps]]
action = "key-up"
key = "Space"
[[steps]]
action = "assert-log-absent"
level = "warn"
target = "masonry"
since = "scenario-start"
```

Log assertions match structured level, target, message substring, and optional
field equality. They do not use regular expressions in schema 1. Positive waits
consume no log records; later assertions may refer to the same record.

A whole-frame golden compares decoded **RGBA pixels**, red, green, blue, and
opacity channel values, at equal dimensions. The tolerance is the fraction of
maximum possible absolute channel error across the image. The runner reports
that score and emits a **heat-map difference image**, whose intensity shows
where pixels disagree. Dimensions must match exactly. Schema 1 has no masks,
crop regions, branching, loops, or object-aware selectors.

The runner records every step start/end, acknowledgement, matched diagnostic,
capture metadata/hash, player exit, and cleanup outcome. On failure it retains
actual capture, diff when available, scenario report, player log, recent
structured diagnostics, and encoder error. Capability tokens are redacted.

## Media capture and visual comparison

At end of frame, the development shell copies the **framebuffer**, Unity's final
rendered pixel image, to a render texture. **GPU readback** then transfers those
pixels from graphics memory to ordinary memory without blocking the rendering
thread. PNG completion means the encoded temporary file was atomically
published. Golden images omit the simulated pointer by default; video may
overlay it when requested.

Video pipes real-time blue, green, red, and opacity samples to system `ffmpeg`.
It uses the macOS hardware H.264 encoder `h264_videotoolbox`, the widely
supported `yuv420p` pixel format, and an MP4 layout playable before a download
finishes. The CLI discovers `ffmpeg` and **`ffprobe`**, FFmpeg's
media-inspection program, or accepts absolute overrides and validates the
encoder before launch.
Masonry does not redistribute FFmpeg; its absence affects video only.

The recorder rejects framebuffer-size mismatch, a second PNG or video of the
same kind, invalid
timing, unwritable or escaping paths, and GPU failure. Early encoder exit
closes input, retains size-limited error output, deletes partial output, and
returns an error. Cleanup targets only the recorded encoder process ID. FFprobe
verifies size, encoding format, rate, frames, and duration. A metadata file
beside the video records those values and artifact hashes.

A player exit before acknowledgement fails the capture and removes its
temporary file. A window-size change during video stops recording and reports
the expected and observed dimensions. GPU readback failure fails only that
capture unless repeated failures make a pending scenario exceed its timeout.
Capture timeout closes encoder input, waits the five-second finalization limit,
then terminates that encoder process and removes partial output.

Golden comparison decodes RGBA pixels, so PNG metadata and compression do not
matter. Actual and heat-map diff are new artifacts; only a successful
`--accept` run changes the checked-in golden.

## Security and release separation

The only game executable is one Rust dylib whose native interface catches a
Rust **panic**, an unrecoverable rules error. Content cannot supply C#,
managed/native Unity plugins, shaders, render features,
Editor scripts, or packages. Manifest, scenario, source, and output paths are
resolved to absolute paths under their allowed roots after following symbolic
links, then checked for unsafe or case-colliding components.

Shell metadata and checksums prove the exact downloaded identity; verified
cache entries are immutable. Control is local-only with an unlogged random
token, fixed message and rate limits, explicit input state, and restricted
outputs. Diagnostics, gameplay, assets, and media have separate size and
per-frame limits and separate parsing. Release shells contain no control code.

Temporary output uses restrictive permissions and atomic publication. The CLI
tracks and cleans only Unity, player, and encoder identities it started. All
mutation precedes nested and outer signing. Credentials remain in Keychain or
the release environment, never manifests, config, reports, or logs.

## Migration and compatibility

Basic drops all Unity-owned files. Its Rust remains; the manifest declares
three generated materials, and rules use standard scene/font plus generated
accessors. Tic-Tac-Toe does the same while retaining its three ordinary PNGs as
declared textures. C# capture scenarios become TOML scenarios supplied by the
development shell.

Repository-specific sample commands are replaced by project discovery and
ordinary `build`/`run`; CI tests a copied sample outside the Masonry tree. ABI
v2 changes verifier, shell, export macro, test libraries, and rules together.
ABI v1 fails without fallback.

Migration lands as one exact-version change. The CLI/schema, ABI v2 adapter,
standard shell, and external-project test are available before the
sample switch; Basic and Tic-Tac-Toe then adopt manifests and generated files in
the same change that removes their Unity directories and the old `sample`
command. Versioned caches permit rollback between standard-setup releases only
when the repository also selects the matching manifest/bindings revision.
Rolling back across the migration requires restoring the old sample layout from
version control; the earlier CLI cannot run a migrated repository. There is no
mixed old-sample/new-shell state that CI accepts and no automatic conversion of
an advanced project.

The reusable Unity package and the `cargo masonry plugin` install, verify,
inspect, and restore commands described in
[Native plugin development](native-plugin-development.md) remain for custom C#,
shaders, packages, rendering, HTTP, Unity settings, Editor execution, or
unsupported content. Standard manifests cannot selectively bypass restrictions.

## Alternatives considered

- **One universal prebuilt player:** rejected because selecting, trusting,
  loading, and safely unloading arbitrary rules libraries would create a game
  launcher and package ecosystem. It also complicates app identity, settings,
  crash isolation, and signing. Immutable game-specific apps are simpler.
- **Always rebuild the complete Unity app:** rejected because Rust-only edits
  would retain the dominant Unity build cost and cache invalidation problem.
- **Always download a shell:** rejected because external artifact availability
  must not be a single point of failure. Exact source fallback is required.
- **Arbitrary Unity project overlays:** rejected because merge order, serialized
  settings, package resolution, and GUID conflicts would reproduce a fragile
  game-owned Unity project.
- **Precompiled custom game extensions:** deferred because managed and native
  extension compatibility, trust, stripping, and signing require a broader
  plugin model. Advanced projects already support executable Unity code.
- **Raw Unity settings patches:** rejected because settings are not a stable
  public schema. Curated typed manifest fields define the supported surface.
- **Multiple renderer configurations:** deferred because each configuration
  multiplies shells, shader variants, content compatibility, and acceptance
  coverage.
- **A large bundled asset library:** rejected because it bloats every shell and
  creates long-term visual compatibility obligations. Standard essentials and
  simple generated assets cover the intended baseline.
- **Path-derived or manual asset strings:** rejected because moving a source
  file would become an API change and manual strings drift. Stable manifest IDs
  produce both catalog addresses and checked-in typed accessors.
- **Automatic binding generation inside Cargo builds:** rejected because an
  ordinary Cargo invocation must not rewrite source or hide an API diff.
- **Live Rust or content replacement without restart:** deferred because dylib
  unload safety,
  native state migration, Addressables lease replacement, and partial failure
  are substantially more complex than a fast process restart.
- **Play Mode in the authoring workspace:** rejected because it creates a
  second bootstrap, native library startup/shutdown sequence, and execution
  environment that can
  disagree with the packaged shell.
- **Native operating-system input automation:** rejected because it depends on
  focus, accessibility permission, display arrangement, and timing outside the
  player. Simulated Unity Input System devices exercise the intended path.
- **Full Unity runtime inspection:** rejected because scenarios would couple to
  implementation details and bypass Rust ownership. Pixels, player-state
  events, and diagnostics are the supported evidence.
- **HTTP transport in standard mode:** rejected because the standard artifact
  is defined around one embedded authoritative engine. Advanced mode retains
  HTTP transport.
- **One shared simulated clock:** rejected because it changes gameplay and Unity
  semantics. Real-time waits and frame waits are sufficient for the initial
  scenarios.
- **Reuse one player across scenarios:** rejected because native, Addressables,
  input, and Unity state could leak between cases. Fresh processes make
  isolation observable.
- **Download and redistribute FFmpeg:** rejected because screenshots do not
  require it and system discovery keeps video encoding, licensing, and security
  updates outside Masonry's distribution.

## Acceptance criteria and automated validation

The completed standard setup must satisfy all of the following observable
criteria.

- `cargo masonry init` creates a buildable external-style project without Unity
  project files.
- Project discovery works after copying that project outside the Masonry source
  tree.
- A cached-shell, standard-assets-only build does not launch Unity.
- A missing shell resolves through an exact download or source build and then
  becomes a verified cache hit.
- Editing only Rust rebuilds the dylib and app, not content or shell.
- Editing a PNG rebuilds content and app, not shell.
- Changing an asset ID makes bindings out of date and blocks build until
  generation.
- Generated output is byte-identical across repeated runs.
- Basic contains only Rust, manifest, generated Rust, scenarios, and ordinary
  documentation after migration.
- Tic-Tac-Toe additionally contains only its ordinary source PNGs.
- Raw content imports to the expected Unity kinds and catalog keys.
- Authored supported assets build, while forbidden scripts, shaders, packages,
  and external dependencies fail with dependency chains.
- Development and release shell metadata and files are distinct.
- Release validation finds no control listener, simulated input, capture code,
  or console dependency.
- The assembled app contains the exact rules, config, and content hashes named
  in its report and passes signature verification.
- ABI v1, wrong architecture, missing symbols, and malformed config fail before
  gameplay starts.
- Rust diagnostics retain order, survive engine construction, report overflow,
  and never enter gameplay response handling.
- The console shows correlated Unity and Rust records and suppresses game keys
  while focused.
- An authenticated scenario controls input; an unauthenticated client receives
  no diagnostics or control.
- Every scenario uses a fresh player process and cleans held input and encoder
  work after success, failure, crash, or timeout.
- PNG capture succeeds without FFmpeg.
- Video capture validates H.264 output and reports missing FFmpeg clearly.
- Golden comparison emits actual and diff artifacts, and `--accept` changes a
  golden only after all other behavior passes.
- Cache corruption is quarantined and recovered without damaging a valid game
  output.

Automated coverage includes Rust unit tests for manifest parsing, path safety,
key generation, fingerprints, invalidation, generated output, and app assembly
planning. Native ABI tests dynamically load an exported test library and
exercise all statuses, ownership, diagnostics, overflow, panic containment, and
architecture verification.

Unity Editor tests validate standard bootstrap configuration, standard assets,
raw import projection, authored dependency policy, catalog construction, and
development/release stripping. Player smoke tests validate packaged startup,
catalog loading, dylib loading, and fatal startup screens.

Shell-distribution tests cover valid, expired, and revoked index signatures;
wrong lengths and checksums; archive traversal, symbolic links, and absolute
paths; official-cache reuse after index expiry; and rejection after a CLI key
revocation. Concurrency tests race the same cache key, simulate a dead lock
owner, interrupt publication at every boundary, and prove the previous output
remains valid.

Assembly tests mutate every allowed path and representative forbidden paths,
then verify the immutable tree hash before and after signing. Control tests
cover byte framing, authentication, increasing request IDs, rate limits,
disconnect/reconnect cleanup, diagnostic replay gaps, and unknown methods.
Scenario parser tests cover every step/default, duplicate IDs, path roots,
publication names, timeouts, and held-input/media cleanup.

**Black-box tests** exercise only public commands and observable outputs. They
create a temporary external repository, record whether Unity was launched,
compare artifact hashes, run Basic and Tic-Tac-Toe scenarios, exercise signing,
and inspect cleanup. The existing fake client continues validating Rust rules
independently of Unity.

## Manual QA

1. Run `cargo masonry init` in a temporary directory, inspect the result, run
   `generate`, and launch the starter game. Confirm there are no Unity project
   settings or package files and the app renders through the native engine.
2. Run the same game again with an exact cached shell. Confirm the terminal
   explicitly reports that Unity was not invoked. Edit one Rust response and
   confirm only rules and app assembly rebuild.
3. Empty or quarantine the exact shell cache, disable downloads, and build with
   a compatible Unity Editor installed. Confirm source fallback creates a
   verified cache entry without adding Unity files to the game.
4. Add a PNG and texture declaration, run `generate`, and run the game. Confirm
   the generated accessor, catalog key, visible texture, and content-only cache
   invalidation agree.
5. Run `cargo masonry author`, create a supported material or prefab, close the
   Editor, and inspect source control. Confirm only authored content and `.meta`
   files persist and Play Mode was unavailable.
6. Add a forbidden MonoBehaviour, shader, or external package dependency to an
   authored root. Confirm validation names both the declared root and forbidden
   dependency and publishes no new content pack.
7. Build and run migrated Basic. Hover and click each cube, observe material
   changes and the tween, and confirm its repository has no Unity project or
   ordinary asset files.
8. Build and run migrated Tic-Tac-Toe. Play through a player and computer move,
   confirm the three PNG-backed textures render, and inspect their generated
   typed addresses.
9. Open the development console, filter Unity and Rust targets, and trigger a
   Rust diagnostic. Confirm terminal and console records share a correlation
   ID. Type
   into the console and confirm game keyboard actions are suppressed.
10. Run the Basic pointer scenario. Confirm virtual input does not move the
    physical pointer or require Accessibility permission, and verify its click
    reaches Rust and produces the visible tween.
11. Capture a PNG with FFmpeg absent. Confirm capture succeeds. Change a visible
    color, rerun the golden, inspect actual and heat-map diff, then accept and
    confirm only the checked-in golden changes.
12. Install or select FFmpeg and run the video scenario. Confirm the MP4 has the
    requested size, H.264 codec, frame count, duration, and reproducibility
    metadata file. Repeat with FFmpeg absent and confirm the focused failure.
13. Force out-of-date generated bindings, an ABI v1 library, a missing catalog,
    a bad bundle hash, an invalid input transition, and a scenario timeout.
    Confirm
    each error identifies recovery and no player or encoder remains running.
14. Build with ad hoc signing and verify the app. Build again with a Developer
    ID identity and test entitlements, then verify nested and outer signatures.
    Confirm the manifest and logs contain no signing secrets.
15. Inspect a release app and attempt the development handshake and console
    toggle. Confirm no listener accepts the connection, no console appears, and
    release build evidence contains none of the forbidden development
    assemblies or symbols.
