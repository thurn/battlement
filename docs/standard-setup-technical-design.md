# Battlement Standard Setup Technical Design

Status: proposed extension to the Battlement client and authoring model

## Start with a game

Consider an external Chess repository:

```text
chess/
├── Cargo.toml
├── battlement.toml
├── src/
│   └── lib.rs
├── content/
│   ├── Main.unity
│   ├── Main.unity.meta
│   └── KayKit/
└── scenarios/
    ├── opening-move.toml
    └── opening-move.png
```

The developer edits the scene with `cargo battlement author`, runs the game with
`cargo battlement run`, and checks the opening move with
`cargo battlement scenario run opening-move`. The repository owns Rust rules,
Unity content, a manifest, and scenarios. It does not own Unity packages,
project settings, render-pipeline configuration, or bootstrap code.

That concrete repository is the model for this design. Each contract below
starts with the file, command, or observable behavior a game author encounters,
then defines the rules behind it.

## Summary

Battlement is a thin Unity rendering and input client for games whose rules and
authoritative state live in Rust. Today a game can avoid game-specific C# but
must still own a complete Unity project. The Basic, Tic-Tac-Toe, and Chess
samples repeat packages, project settings, render-pipeline configuration,
bootstrap code, and build configuration that belong to Battlement.

The **standard setup** lets a game own Rust rules, a small manifest, ordinary
Unity-authored content, and automated scenarios without owning a complete Unity
project. Developers clone Battlement and build a **standard shell**, a reusable
application containing Unity and the Battlement client but no game rules or game
content. The command-line interface (CLI) installs one game's native rules
library and content into a copy of that shell, applies metadata, and signs the
result.

Game content remains native Unity content. Scenes, prefabs, models, textures,
materials, and their `.meta` files live in the game repository and are edited in
a disposable Unity project supplied by Battlement. The game declares only the
content roots that Rust addresses directly. Unity includes their referenced
dependencies. Battlement does not reproduce Unity importer settings in TOML,
or derive Unity asset identifiers. It does generate typed Rust constants for
the stable public addresses declared by the game.

Standard builds provide practical diagnostics without changing the native
gameplay ABI. Rust logs go to stderr. The CLI captures the process stream in
`native-stderr.log` and tails it beside Unity's player log.
UnityIngameDebugConsole displays Unity logs inside development and release
players, and Battlement adds current frame-rate and connection information beside
it. Development builds additionally provide repeatable scenario evidence. The
CLI preserves both logs for failed or crashed runs.

Automated scenarios use the existing visual-capture model. A compact TOML
scenario can wait for readiness, perform clicks and key presses through
simulated Unity Input System devices, wait for frames or time, and compare a
captured screenshot with a checked-in golden image. The transport remains an
implementation detail of local test execution, not a new public control
protocol.

Games that need custom C#, custom shaders, additional Unity packages, a
different rendering pipeline, or unsupported project settings continue to use
Battlement's advanced bring-your-own-Unity-project path.

## Related information

- [Battlement Technical Design](technical-design.md) defines the Rust and Unity
  gameplay protocol. Standard setup changes project ownership and packaging,
  not the rule-authority boundary.
- [Native plugin development](native-plugin-development.md) defines the current
  native library installation, verification, architecture, and signing behavior
  reused during app assembly.
- [Visual evidence capture](visual-capture.md) defines the existing simulated
  input and framebuffer-capture implementation reused by standard scenarios.
  Its authored-C# scenario contract does not apply to standard setup.
- [Fake client design](fake-client-design.md) defines the Rust-only substitute
  for Unity. Player scenarios complement rather than replace fake-client tests.
- [UnityIngameDebugConsole v1.8.9][console] at commit
  `4467c225eaaf5c0db62a11e2c6851a9fdb64763c` supplies the in-game log
  viewer. Its [MIT license][console-license] remains in distributions that
  contain it.

[console]: https://github.com/yasirkula/UnityIngameDebugConsole/tree/4467c225
[console-license]:
  https://github.com/yasirkula/UnityIngameDebugConsole/blob/4467c225/LICENSE.txt

## Problem and current state

The reusable Unity package already provides `BattlementRunner`,
`BattlementBootstrap`, embedded and HTTP-hosted rule transports, MessagePack
serialization, Addressables storage, object construction, grouped command
execution, pointer and keyboard input, and error reporting. A game with no
custom C# still has to place those components into a scene and maintain the
Unity project around them.

Basic and Tic-Tac-Toe each own a complete `Assets`, `Packages`, and
`ProjectSettings` set. Most files are the same Battlement infrastructure. Basic
needs only its Rust behavior and a few colors. Tic-Tac-Toe additionally needs
three PNG files.

Chess demonstrates the other important case. It has a substantial Unity scene
and a library of imported FBX models. The scene, models, textures, materials,
and `.meta` files are genuine game content that must remain editable in Unity.
Its package manifest, input configuration, Universal Render Pipeline (URP)
settings, and other project settings are still duplicated infrastructure.

The current `cargo battlement sample` command assumes the game lives inside the
Battlement repository. It locates a sample-specific manifest, builds the current
machine's native rules library, installs it into that sample's Unity project,
and asks Unity to build the project. This proves that Rust-authored standalone
games work, but it makes external game repositories and fast Rust-only
iteration awkward.

The root Battlement repository is already the authoritative Unity project. It pins
the supported Unity and package versions and owns the standard renderer,
bootstrap, client package, input configuration, and build support. Standard
setup reuses that project as the source of both the player shell and disposable
content-authoring workspaces.

## Goals and invariants

- A game repository works from any location on disk.
- A developer explicitly clones Battlement and uses that checkout as the standard
  shell source. Standard setup never downloads a prebuilt shell.
- Rust remains the owner of game rules and authoritative game state.
- Every packaged application contains one game's native Rust engine and one
  game catalog. It is not a universal launcher.
- Battlement owns Unity packages, project settings, bootstrap, render pipeline,
  Addressables settings, and standard assets.
- Games own Unity-authored content and its `.meta` files without owning Unity
  infrastructure.
- Chess can migrate without flattening, translating, or reauthoring its scene
  and imported models.
- Unity remains the source of truth for importer settings and asset identifiers.
- The manifest names only assets that Rust must address directly. Unity resolves
  their transitive dependencies.
- `cargo battlement generate` is the only command that rewrites checked-in Rust
  source for asset addresses.
- Rust-only changes rebuild the rules library and assembled app without
  rebuilding the shell or game content.
- Development runs preserve Unity logs, Rust logs, exit status, and available
  crash information.
- A development or release run shows Rust and Unity logs in the terminal, while
  the player can show Unity logs and current frame rate.
- Each automated scenario starts a fresh player and drives Unity's normal Input
  System path.
- Screenshot comparison operates on final rendered pixels and produces useful
  failure artifacts.
- Scenario-only simulated input, file-command handling, and capture code are
  absent from release players; the console and FPS surface remain available.
- Failed app builds do not replace the last valid app, and failed scenarios do
  not replace golden images.
- Existing advanced Unity projects remain supported.

The first supported platform is native macOS. Windows, Linux, mobile, WebGL,
HTTP-hosted rules, multiple render pipelines, and Apple notarization are outside
this contract.

## Developer experience

A typical first session in a game repository is:

```console
$ cargo battlement doctor
Game: Chess 1.0.0
Battlement checkout: /Users/alex/src/battlement
Unity: 6000.5.8f1

$ cargo battlement generate
Generated rules/src/battlement_assets.rs (2 addresses)

$ cargo battlement run
shell: reused
content: rebuilt
rules: rebuilt
app: target/battlement/Chess.app

$ cargo battlement scenario run opening-move
opening-move: passed
```

From that experience, the command contract follows.

The CLI discovers a game by searching from the working directory toward the
filesystem root for `battlement.toml`. An explicit manifest path disables search.
Every relative manifest path resolves from the manifest's directory.

The supported commands are:

- `init [path]` creates a manifest, Rust rules package, generated address module,
  starter content directory, starter scenario, and ignore rules without
  overwriting work.
- `doctor` checks the manifest, generated addresses, Cargo dependencies, Battlement
  checkout, required Unity version, and signing requirements. It also prints
  cache locations so a developer can remove stale entries.
- `generate` atomically rewrites the checked-in generated address module from
  the manifest and reports added, removed, renamed, or retyped constants.
- `build [--release]` resolves or builds the shell, builds changed rules and
  content, assembles the game app, signs it, and prints the output path.
- `run [--release]` performs the incremental build, launches the app, tails its
  logs, and preserves run artifacts after exit.
- `author` opens a disposable Unity project containing Battlement infrastructure
  and the game's directly editable content.
- `scenario run <name>` launches a fresh development player and executes one
  TOML scenario. `--all` executes every scenario independently and `--accept`
  replaces otherwise-successful golden screenshots.

Build, run, author, and scenario commands validate that generated addresses are
current and that every declared root appears once in the built catalog. They do
not regenerate source implicitly; a mismatch stops before compiling rules and
directs the developer to run `cargo battlement generate` and review the Rust diff.

Unity is required to build an absent shell, build changed content, and author
content. A cached shell and content pack let Rust-only `build`, `run`, and
scenario iterations avoid the Unity Editor. Screenshot capture does not require
FFmpeg.

`build --release` always signs. Ad hoc signing is the default and produces a
locally valid application without a trusted developer certificate. A named
Developer ID identity resolves through the macOS Keychain. Notarization and
stapling are not performed.

## Game manifest

This complete manifest shows the schema in context before its fields are
defined:

```toml
schema = 1

[game]
id = "com.example.chess"
name = "Chess"
version = "1.0.0"

[rules]
manifest = "Cargo.toml"
package = "chess-rules"
default_features = false
features = ["standard-setup"]

[display]
width = 1440
height = 900
mode = "windowed"
resizable = true
frame_pacing = "fixed"
target_fps = 60

[diagnostics]
console = true
console_toggle = "Backquote"

[content]
directory = "content"

[[content.addressables]]
id = "main"
kind = "scene"
source = "Main.unity"

[[content.addressables]]
id = "move_marker"
kind = "prefab"
source = "Board/MoveMarker.prefab"

[scenarios]
directory = "scenarios"
default_timeout_seconds = 20
golden_tolerance = 0.002

[macos]
bundle_identifier = "com.example.chess"
category = "public.app-category.board-games"
minimum_version = "14.0"
build = 1
architectures = ["arm64", "x86_64"]

[signing]
identity = "Developer ID Application: Example Studio (ABCDE12345)"
entitlements = "release.entitlements"
```

The manifest is a strict TOML document. Unknown tables, fields, and enum values
are errors so misspellings cannot silently receive defaults. Schema version 1
contains game metadata, rules selection, display settings, diagnostics,
content, scenarios, macOS metadata, and signing.

### Game and rules

`game.id` is a permanent reverse-domain package identity and defaults the macOS
bundle identifier. `game.name` is display metadata and defaults the outer app
name. `game.version` is a three-component numeric version. Changing these
values affects app metadata, not content addresses.

`rules.manifest` and `rules.package` select exactly one Cargo package with a
`cdylib` target named `battlement_rules`. Feature selection is allowed through
declared Cargo features. Arbitrary build commands, environment variables, link
flags, and output paths are not manifest features.

The generator locates that target's crate root through Cargo metadata and owns
`battlement_assets.rs` beside it. `init` declares the module from the crate root.
The generated file is checked in so address API changes are visible in review
and callers receive ordinary compiler errors after a constant is removed or
changes type.

The CLI uses Cargo metadata to locate the game's `battlement` and
`battlement-native` path dependencies. Both must come from the same Battlement
checkout and match the checkout's package versions. This checkout is also the
shell and authoring-project source.

`schema`, `game.id`, `game.name`, `game.version`, `rules.manifest`,
`rules.package`, `display.width`, `display.height`, and `content.directory` are
required. The example deliberately supplies every schema-1 field. A game may
omit any field shown as optional in the sections below and receive the stated
default. It may also omit all `[[content.addressables]]` entries when Rust does
not directly address game content.

Values follow the types demonstrated above: flags are Boolean; dimensions,
frame rate, build number, and timeouts are integers; screenshot tolerance is a
floating-point number; features and architectures are string arrays; and the
remaining scalar values are strings. Each repeated
`[[content.addressables]]` table requires string `id`, `kind`, and `source`
fields.

Enum spellings are `windowed` and `borderless_fullscreen` for display mode;
`vsync`, `unlimited`, and `fixed` for frame pacing; and `scene`, `prefab`,
`particle_effect`, `material`, `texture`, `audio_clip`, and `font` for content
kind. Architectures are `arm64` and `x86_64`. `console_toggle` is one Unity
Input System key name and defaults to `Backquote`. Signing identity `-` means ad
hoc signing; every other nonempty value is a Keychain identity name.

Defaults are development-oriented: windowed, non-resizable, vertical
synchronization, console enabled, a 20-second scenario timeout, exact screenshot
comparison, the current machine's development architecture, both arm64 and
x86_64 for release, and ad hoc signing. Fixed frame pacing requires
`target_fps`; other pacing modes reject it. `doctor` prints all effective
values.

### Display and diagnostics

Display mode is windowed or borderless fullscreen. Windowed builds declare
positive pixel dimensions and whether the window is resizable. Frame pacing is
vertical synchronization, unlimited, or a fixed positive frame rate. Schema 1
has one Battlement-owned URP quality configuration.

Diagnostics declare whether the in-game console is enabled and its toggle key
for both development and release builds. UnityIngameDebugConsole provides its
own interactive filtering. Standard setup does not parse or filter native
stderr. Release retains ordinary logs, the console, its toggle handling, and
the FPS status surface.

### Content

In the example manifest, Rust addresses the `main` scene and `move_marker`
prefab. `Main.unity` can still reference the board models, textures, materials,
animations, and audio below `content/` without listing any of them in TOML.
Those files are dependencies, not public catalog roots.

`content.directory` names the game-owned Unity asset root. Every file below it
is ordinary Unity content. Unity-authored files and imported source files retain
their checked-in `.meta` companions. The directory may contain scenes, prefabs,
materials, animation, particles, audio, textures, models, fonts, and other
non-executable assets supported by the standard Unity project.

Each `content.addressables` entry contains:

- A stable lowercase snake-case `id` used by Rust and in the game catalog.
- A broad `kind`: scene, prefab, particle effect, material, texture, audio clip,
  or font.
- A source path below the content directory.

IDs begin with a lowercase letter and contain only lowercase ASCII letters,
digits, and underscores. They are unique across kinds and do not include the
game ID. A game is packaged
with only one game catalog, so cross-game namespacing adds no safety. Changing a
source file or moving it while keeping the same ID does not change its public
address. Changing the ID is an intentional Rust API change.

Only directly addressed roots are declared. An authored scene may reference
hundreds of models, textures, materials, animation clips, and prefabs without
listing them in the manifest. Unity includes that dependency closure in the
content pack.

The CLI validates that each declared path exists, has a `.meta` file when Unity
requires one, and imports as the declared broad kind. It does not project Unity
importer settings into TOML. Texture filtering, model scale, material import,
audio compression, and similar choices remain in the source asset's `.meta`
file and are edited with Unity.

Game content cannot contain C# source, assembly definitions, managed or native
plugins, Editor scripts, package manifests, or project settings. Missing scripts
and dependencies outside the game-content and Battlement-owned roots fail the
content build with the dependency path. Custom shader assets and Shader Graphs
are not supported in schema 1; game materials may reference shaders supplied by
Unity, URP, or Battlement.

The validator does not maintain an exhaustive allowlist of Unity components or
serialized asset types. A scene may use built-in, URP, Input System, TextMesh
Pro, and Battlement components supplied by the fixed standard project. This is what
allows a real authored Chess scene to migrate without Battlement recreating a
smaller Unity object model.

### Address use from Rust

`cargo battlement generate` maps every declared ID to an uppercase snake-case
constant whose type follows the declared kind. Given the manifest above, the
generated module exposes constants equivalent to:

```rust
pub const MAIN: SceneAddress = SceneAddress::from_static("main");
pub const MOVE_MARKER: PrefabAddress = PrefabAddress::from_static("move_marker");
```

Rules import `crate::battlement_assets` and use `battlement_assets::MAIN` and
`battlement_assets::MOVE_MARKER` without repeating address strings. The public
catalog key remains the declared ID. IDs are already unique across kinds, so
schema 1 uses one flat constant namespace and rejects any normalized-name
collision rather than inventing a suffix.

The generated file records the schema version, generator version, and a
fingerprint of all binding-relevant IDs and kinds. Entries are sorted by ID and
identical inputs produce byte-identical output on every machine. Source paths,
game metadata, and importer data are excluded because changing them does not
change the Rust address API.

`AssetAddress` supports a constant `from_static` constructor for generated
addresses and retains its owned constructor for dynamic callers. Both forms
serialize to the same string key. Runtime still verifies that each catalog
entry resolves to its declared Unity type before rules begin.

Standard shell addresses remain handwritten constants in the versioned
`battlement::standard` API rather than being copied into every generated game
module.

### Scenarios and signing

The scenarios table names a directory, default timeout, and default screenshot
tolerance. Scenario files are direct TOML children of that directory and are
executed in filename order by `--all`.

The optional signing table selects ad hoc signing or names a Developer ID
identity and a reviewed entitlements file. Secrets, certificates, and passwords
never appear in the manifest. The accepted entitlements are limited to the
capabilities explicitly supported by standard mode. Schema 1 accepts only the
boolean app-sandbox and outbound-network-client entitlements; outbound network
access requires the sandbox entitlement. Hardened runtime is enabled for a
Developer ID release.

`game.version` becomes `CFBundleShortVersionString`; the positive integer
`macos.build`, defaulting to 1, becomes `CFBundleVersion`. Architecture values
are `arm64` and `x86_64` without duplicates.

### Path safety

The **game root** is the directory containing `battlement.toml`. Manifest paths are
relative UTF-8 paths below that root. After resolving
symbolic links, content, scenario, entitlement, and golden-image inputs must
remain within their allowed roots. Outputs are written only below CLI-owned
target directories. Absolute paths, parent traversal, control characters, and
case-insensitive collisions fail validation.

## Unity content authoring

For the example Chess repository, `cargo battlement author` opens `Main.unity`
inside a generated project. Moving a KayKit model updates files below
`chess/content/`; changing URP settings updates only the disposable workspace
and is discarded. The ownership boundary is visible in the Project window.

`cargo battlement author` opens a disposable Unity project assembled from the
selected Battlement checkout. The project uses Battlement's packages, project
settings, render pipeline, Addressables settings, bootstrap, and standard
assets. The game's configured content directory appears as an editable asset
root inside that project.

Edits to game content persist directly into the game repository, including
Unity-created `.meta` files. Generated project infrastructure, Unity caches,
logs, and user settings remain in a CLI-owned ignored workspace. The command
must make the ownership boundary visible and must not copy incidental project
settings back into the game.

The authoring workspace directly mounts the game-content directory at one fixed
location below Unity's `Assets` root. It does not maintain a second writable
copy and performs no copy-back synchronization. Startup verifies that Unity
recognizes the mount and that imported asset paths remain below it. If the
supported Unity version cannot satisfy that contract, `author` fails rather
than falling back to a lossy synchronization scheme.

Only one authoring editor may hold a game's workspace lock. Content builds fail
with an actionable message while that editor is open. An ordinary content build
uses a private snapshot of the content root, so Unity import cannot create or
rewrite source `.meta` files during a build. Changes made by the authoring
Editor, including importer-driven `.meta` updates, are intentional game-source
changes and are visible in source control immediately.

Play Mode is supported in the authoring project. Before entering Play Mode, the
workspace installs the current development rules library and game content and
uses the same Battlement bootstrap as the packaged development player. Play Mode
is an iteration convenience, not a second gameplay implementation. If its
inputs are stale or the rules library cannot be built, Unity reports the
actionable failure instead of running an older game silently.

Unity uses Battlement's bootstrap as the Play Mode start scene. The Rust initial
snapshot loads the same declared game scene that a packaged player loads. When
the Editor is not playing, `author` opens the first declared scene root, while
Unity may subsequently remember the developer's last open game scene in the
ignored workspace.

The content builder uses an equivalent disposable project without exposing it
to the user. It marks only the manifest-declared roots as public Addressables,
lets Unity resolve their dependencies, and produces one catalog and bundle set.
It never edits the game source during an ordinary build.

## Standard shell

On the example developer's first `cargo battlement run`, the CLI builds a
development shell from `/Users/alex/src/battlement`. A later Rust-only edit reuses
that shell. A release build uses a separate release shell that retains the log
viewer and FPS display but never contains scenario controls.

The root Unity project in the selected Battlement checkout is the only standard
shell source. Developers obtain it by cloning Battlement. Standard setup does not
discover, download, authenticate, publish, revoke, or update shell archives.

The shell contains the production bootstrap scene, Battlement client, standard
renderer, standard assets, native-library slot, configuration loader, catalog
loader, diagnostics, and the code required for its profile. It contains no game
rules or game content.

Schema 1 exposes five standard assets through handwritten constants in the
`battlement::standard` API: the empty scene, default font, white lit material,
white unlit material, and white texture. Their keys remain under the reserved
`battlement/` prefix. Game IDs cannot contain `/`, so they cannot collide with
these shell keys. Changing this standard set requires a content-format change.

Development and release are separate shell profiles. Development contains the
log viewer, FPS display, simulated input, file-command handler, and screenshot
capture. Release contains the same log viewer and FPS display while omitting the
scenario-control assemblies and assets; a dormant runtime flag is not
sufficient separation for those scenario-only capabilities.

### Shell cache

The CLI fingerprints the Unity version, package lock, shell profile,
architecture, and relevant Battlement Unity source files. It reuses the shell when
that fingerprint matches and invokes Unity otherwise. The fingerprint need not
encode Git history or distinguish committed from uncommitted files; it covers
the source bytes that affect the shell.

The shell cache is disposable. An incomplete or unreadable entry is deleted and
rebuilt, and a developer may remove any entry without losing source data.
Concurrent builds of the same entry are not supported in schema 1. This is a
local iteration optimization, not a publication or trust system.

## Content build and caching

Suppose `Main.unity` references 200 KayKit files. The resulting game catalog has
the public keys `main` and `move_marker`, not 202 public keys. Editing a KayKit
texture invalidates the content pack because its bytes changed; editing Rust
rules does not.

The content builder creates one Addressables catalog for the game. Public keys
are exactly the declared IDs; Unity dependencies have no public Battlement key
unless separately declared. Catalog entries expose no path, GUID, label, or
game-ID aliases.

The content fingerprint covers:

- The complete Battlement-owned Unity build environment used for content,
  including its project and renderer settings, content format, Unity version,
  package lock, and macOS build target.
- All game-content bytes, including `.meta` files.
- The declared public IDs, kinds, and paths.

Changing Rust, game display metadata, diagnostics, or signing does not rebuild
content. Changing a scene, model, texture, `.meta` importer setting, or public
root declaration does. The content build validates every declared root and
reports missing scripts and forbidden executable content before publication.

At player startup, Unity loads the standard catalog and then the game catalog.
It rejects duplicate public IDs, unknown kinds, checksum mismatches, and entries
whose Unity type differs from the manifest. Catalog validation completes before
the native rules engine is created.

The compiled game configuration records its schema, content-format version,
game catalog checksum, public ID and kind table, display settings, and active
profile. The shell reads it from a fixed internal app location. The native rules
slot and game catalog locations are likewise fixed shell contracts defined with
the existing native-plugin installation design. App metadata never changes
those lookup locations.

## App assembly and signing

For the example development build, assembly produces
`target/battlement/Chess.app`: it copies the cached shell, installs
`libbattlement_rules.dylib`, the compiled configuration, and the Chess catalog,
sets the displayed name to Chess, and ad hoc signs the completed app. The
cached shell remains unchanged.

Assembly copies a validated shell into a temporary game application, leaving the
cached shell untouched. It installs the architecture-compatible native rules
library, compiled game configuration, and game catalog and bundles. It patches
supported application metadata, removes the shell signature, signs embedded
code, signs the outer application, verifies the result, and then atomically
publishes the output.

Development builds use ad hoc signing and no release entitlements. Release
builds use the configured identity and entitlements, or ad hoc signing when no
Developer ID is configured. A named identity must resolve through Keychain. The
shell and native library, independently, must contain every requested
architecture. The completed application is verified with `codesign`;
notarization is outside this design.

The application uses fixed internal locations for configuration, native rules,
and game content. Display name and outer application name never select runtime
files. A failed rules build, content build, metadata update, or signature check
preserves the previous valid application.

## Incremental builds

For the example game, four common edits have deliberately different effects:

| Edit | Rules | Content | Shell | Assembly |
| --- | --- | --- | --- | --- |
| Change a legal-move rule | rebuild | reuse | reuse | rerun |
| Change `Board.png` | reuse | rebuild | reuse | rerun |
| Change the app display name | reuse | reuse | reuse | rerun |
| Change Battlement's renderer | reuse | rebuild | rebuild | rerun |

Standard setup owns two disposable caches: the standard shell and the game
content pack. Cargo remains responsible for incremental compilation of the
native rules library. Final app assembly reruns whenever an installed result or
app metadata changes.

The intended behavior is deliberately coarse:

- A Rust-only change rebuilds through Cargo and reassembles the app without
  invoking Unity.
- A game-content, `.meta`, or addressable declaration change rebuilds content
  and reassembles the app.
- A relevant Battlement Unity source, Unity version, package, profile, or
  architecture change rebuilds the shell. Content also rebuilds when its Unity
  environment changes.
- Metadata and signing changes only reassemble and sign the app.

Commands print whether each cache was reused or rebuilt. A cache entry is reused
only after a completed build marked it valid; incomplete or unreadable entries
are discarded. Schema 1 does not promise concurrent cache writers, preservation
of old cache entries after a failed rebuild, or a detailed explanation of
individual invalidation inputs. If another command is already building the same
entry, the later command may fail and ask the developer to retry.

## Runtime startup and failure behavior

For a successful Chess launch, the bootstrap validates both catalogs, loads the
native library, applies Rust's initial snapshot, renders `main`, and only then
reports ready. If `main` was declared as a prefab by mistake, startup exits
before calling Rust and the CLI identifies catalog validation as the failed
stage.

Player startup performs the following observable work in order:

1. Read and validate compiled game configuration.
2. Initialize and validate the standard catalog.
3. Initialize and validate the game catalog when present.
4. Start development diagnostics and file-command handling when requested.
5. Load the native library and verify architecture, required symbols, and ABI.
6. Create the Rust engine and connect the Battlement runner.
7. Apply the initial snapshot and complete one rendered frame.
8. Report readiness to the CLI and any active scenario.

No rules entry point runs before catalog validation. Ready means that the
initial snapshot has been applied, at least one resulting frame rendered, and
development control is usable. It does not imply that all future asynchronous
game work is idle.

A startup failure records a stable error message in the Unity player log and,
when possible, on a minimal fatal screen. The process exits nonzero. The CLI
reports the failed stage, preserves logs, and does not publish a failed app over
a valid one.

## Diagnostics

During the example run, a Rust line such as `illegal move: e2-e5` appears in the
terminal and `native-stderr.log`. A Unity missing-material warning appears in
the terminal, Unity player log, and in-game console. Neither system translates
or duplicates the other's records.

Diagnostics serve three audiences: a developer watching the terminal, a player
opening the in-game viewer, and a developer investigating a failed or crashed
run. They use ordinary logs rather than a separate query or subscription
service.

### Rust logging

Standard setup does not add logging operations to the native ABI. Game rules
and the native adapter write Rust diagnostics to stderr using their existing
logging facilities. Panics and adapter failures also write to stderr before
returning an engine failure when possible.

The CLI captures the player process's stderr verbatim as `native-stderr.log` in
the run directory and tails it to the terminal with a native source label. Rust
logs, panic output, and output from other native code may share this stream.
Standard setup does not assign levels or parse records from it. Nothing from
native stderr is forwarded into Unity or shown in the in-game viewer.

### Preserved run logs

`cargo battlement run` and the scenario runner launch the app's internal player
executable directly rather than delegating to the macOS `open` command. They
direct Unity's player log to a CLI-owned run directory and capture process
stdout and stderr as `native-stdout.log` and `native-stderr.log`. They tail the
Unity and stderr logs while the player runs and retain both native streams, the
Unity log, and exit status. When macOS produces a matching crash report, the CLI
records its path; the absence of such a report does not hide the other failure
evidence.

The CLI does not depend on the in-game viewer to collect evidence. A player that
never reaches readiness, crashes while rendering, or has a broken console still
leaves the available native streams and Unity log behind.

### In-game viewer and FPS

Development and release shells use the pinned UnityIngameDebugConsole dependency
for log display, filtering, scrolling, and clearing. Battlement configures the
dependency as a viewer over Unity's ordinary log stream; it does not receive
native stderr or build a competing log-overlay implementation.

UnityIngameDebugConsole does not supply an FPS counter. Battlement adds a small
status surface showing current and rolling-average frames per second and basic
connection state. It is a compact panel beside the log viewer and updates at a
human-readable cadence rather than every rendered frame. Console focus
suppresses game keyboard actions, and Unity UI hit testing prevents console
interaction from reaching the game world. The pinned console source and license
are carried by the Battlement Unity project, not fetched or supplied by each game.

The viewer, FPS surface, and toggle input are available in development and
release players whenever diagnostics are enabled. Scenario controls remain
development-only.

## Local scenario automation

The Chess opening-move scenario is a complete example of the authored format:

```toml
schema = 1
name = "opening-move"
timeout_seconds = 20

[window]
width = 1440
height = 900

[[steps]]
action = "click"
x = 0.22
y = 0.78

[[steps]]
action = "click"
x = 0.22
y = 0.55

[[steps]]
action = "wait_frames"
frames = 2

[[steps]]
action = "screenshot"
name = "after-e2-e4"
golden = "scenarios/opening-move.png"
tolerance = 0.002
```

The CLI starts a fresh development player, waits for readiness, performs the
two clicks through simulated Unity input, waits for two rendered frames, and
compares the screenshot. That observable flow defines the automation model.

Standard scenarios reuse the existing in-player simulated input and framebuffer
capture implementation defined by Visual evidence capture. The CLI starts a
development player, waits for its first rendered frame, and issues one input,
wait, or capture command at a time. The existing private control directory is an
implementation detail and is not versioned as part of standard setup.

Simulated clicks and key presses traverse Unity's Input System, normal hit
testing, Battlement actions, Rust rules, and rendering. Scenarios cannot inspect or
mutate Unity objects or Rust state. A timeout, command failure, or unexpected
player exit fails the run; the CLI then stops the player and preserves available
logs and captures.

## Scenario contract

The example above contains the entire document shape: `schema = 1`, required
string `name`, optional positive integer `timeout_seconds`, an optional
`window` table with positive integer `width` and `height`, and one or more
`[[steps]]` tables. The name must match the file stem. Every scenario starts a
fresh development player and waits for readiness before executing its first
step. `--all` runs each direct `.toml` child of the scenario directory
independently in filename order and continues after failures.

Schema 1 supports these steps:

- `wait_seconds` requires positive numeric `seconds`.
- `wait_frames` requires positive integer `frames`.
- `click` requires finite numeric `x` and `y` coordinates from zero through one
  and clicks the primary pointer button.
- `key_press` requires string `key` using a Unity Input System key name.
- `screenshot` requires safe relative string `name`, which selects its path in
  the run output. It may include a safe `golden` path below the game root and a
  numeric `tolerance` from zero through one.

Every step declares string `action` and only that action's fields. Unknown
fields and actions are errors. Steps execute in document order under the single
scenario timeout. Schema 1 has no held input, drag gestures, branching, loops,
runtime inspection, or per-step timeouts. More complex behavior belongs in Rust
tests or dedicated fixtures.

### Screenshot comparison

The player captures the completed framebuffer after rendering. PNG publication
is acknowledged only after the file is complete. The CLI decodes actual and
golden images to red, green, blue, and opacity channels and requires equal
dimensions.

The comparison score is normalized mean absolute channel error: the sum of the
absolute differences for all red, green, blue, and opacity samples divided by
the product of the sample count and 255. Tolerance is the largest passing score.
A mismatch preserves the actual image and creates a difference image. PNG
metadata and compression do not affect comparison.

A screenshot without `golden` is capture-only and cannot specify `tolerance`.
With a golden path, tolerance comes from the step, then
`scenarios.golden_tolerance`, and otherwise defaults to exact comparison. A
missing golden fails a normal run. With `--accept`, a missing golden may be
created after successful scenario completion.

Scenario window dimensions are rendered-pixel dimensions and do not change with
Retina display scaling. Standard scenarios use the shell's fixed renderer,
color space, quality settings, antialiasing, and supported Unity version.
Goldens are specific to the standard macOS shell profile; small remaining GPU
or font-rasterization differences are handled through an explicit nonzero
tolerance rather than an implicit platform adjustment. Scenarios wait for
readiness and explicit frame or time steps before capture; Battlement does not
claim that arbitrary real-time animation is pixel deterministic.

With `--accept`, a golden mismatch is eligible for replacement rather than a
scenario failure. All input, wait, capture, and shutdown behavior must still
succeed. Goldens are replaced only after the whole scenario succeeds, so a later
failure leaves every existing golden unchanged. Each run retains the actual and
difference images, comparison score, player exit status, Unity player log, and
native stderr needed to understand a failure.

## Security and release separation

Two examples establish the boundary. A material and its texture are valid game
content because they are data consumed by the fixed Unity player. A C# behavior
beside that material is rejected because it adds executable Unity code. The one
permitted game executable is the Rust rules library installed during assembly.

Standard game executable content is limited to the one Rust rules library.
Game Unity content cannot add C#, assemblies, native plugins, Editor scripts,
packages, or project settings. The content builder reports forbidden files and
missing dependencies before publication.

All input and output paths are confined to their declared roots after symbolic
link resolution. Temporary and scenario control directories use restrictive
permissions. Cleanup targets only process identities started by the CLI.

The local scenario mechanism does not open a socket and has no remotely
reachable surface. It does not need authentication tokens because the CLI owns
the private directory and child process.

Release validation inspects the build report and assemblies to prove the
file-command handler, simulated input, and capture code are absent. It also
proves the log viewer and FPS surface are present when diagnostics are enabled.
Native stderr and ordinary Unity logs remain available for release diagnosis.

Credentials remain in Keychain or the release environment. They never enter
the manifest, generated configuration, logs, or scenario artifacts.

## Migration

Basic removes its Unity project. Its rules use Battlement's standard empty scene
and default font. Its build-safe colored materials become a small set of
Unity-authored game assets. Any material that Rust addresses directly receives a
generated typed constant.

Tic-Tac-Toe removes its Unity project and moves its PNG files, with Unity
`.meta` files, into the game content root. It declares the textures it addresses
directly with stable IDs and replaces address literals or game-owned constants
with the generated typed constants.

Chess keeps its authored main scene, default volume profile when referenced,
KayKit models, textures, materials, and all associated `.meta` files as game
content. It drops its package manifest, project settings, input settings, URP
pipeline assets, global settings, and other Battlement-owned infrastructure. The
main scene is one declared addressable root; its model and texture references
are included transitively. The scene is opened and edited through
`cargo battlement author` using Battlement's standard renderer and packages.

The current Chess scene audit finds no C#, assembly, plugin, custom shader, or
Shader Graph asset. Its serialized model references resolve to checked-in
KayKit `.meta` identifiers. Its only script components are URP's additional
camera and additional light data, both supplied by the standard project; its
remaining special identifiers are Unity built-ins. The existing default volume
profile is not referenced by the current main scene and need not migrate unless
later content begins using it. Migration validation repeats this dependency
audit against the exact scene rather than assuming the inventory remains
unchanged.

The Battlement bootstrap remains the application start scene. Rust's initial
snapshot loads the declared Chess `main` Addressable scene, so Chess does not
replace or embed bootstrap behavior in its authored scene. A checked-in Chess
golden and a simple move scenario provide the visual and behavioral equivalence
gate after migration.

Repository-specific sample discovery is replaced by manifest discovery. Tests
copy a migrated game outside the Battlement tree and use its Cargo path
dependencies to find the selected Battlement checkout.

There is no compatibility layer for the earlier generated-binding layout,
curated importer tables, generated Unity GUIDs, downloaded shells, or the TCP
development control protocol described by earlier drafts of this design. Those
designs were never a released standard-setup contract.

The reusable package and native-plugin commands remain available for advanced
Unity projects. Migration to standard mode is optional for games that require
custom executable Unity content or project configuration.

## Alternatives considered

- A universal prebuilt launcher was rejected because every output should have
  one game identity, one rules library, and ordinary standalone-app behavior.
- Published shell archives were rejected because developers can clone Battlement
  and build the exact checkout selected by their Cargo dependencies. A signed
  shell distribution system adds security and operations work without serving
  the current workflow.
- Rebuilding the complete Unity app after every Rust edit was rejected because
  Unity dominates iteration time and the shell is independent of game rules.
- A TOML importer schema was rejected because Unity already stores importer
  choices in `.meta` files and exposes mature authoring tools for them.
- Generated Unity GUIDs were rejected because checked-in `.meta` files already
  preserve Unity identity for raw and authored content.
- Handwritten address strings were rejected as the primary standard-game API
  because the compiler cannot check spelling or kind against the manifest.
  Explicit generation makes address API changes reviewable and compiler-checked.
- Enumerating every Unity dependency in the manifest was rejected because
  scenes and prefabs naturally own dependency graphs that Unity already knows
  how to build.
- An exhaustive component and asset allowlist was rejected because it would
  recreate a partial Unity object model and block ordinary authored scenes such
  as Chess. The boundary instead excludes executable content and requires all
  components to resolve from the fixed project.
- A bespoke log overlay was rejected because UnityIngameDebugConsole already
  provides the needed Unity-log viewing and filtering interface.
- A native-to-Unity Rust logging bridge was rejected because captured stderr is
  sufficient for development and crash diagnosis. Logging does not justify a
  new native ABI version, queue, polling loop, or duplicate records in two log
  files.
- A TCP automation service was rejected because scenarios control a CLI-owned
  local child. The existing visual-capture mechanism already covers sequential
  input and screenshots.
- Native operating-system input automation was rejected because it depends on
  focus, accessibility permissions, display arrangement, and physical device
  state. Simulated Unity Input System devices exercise the intended game path.
- Full Unity or Rust runtime inspection was rejected because it couples tests
  to implementation details and bypasses player-visible behavior.
- Reusing one player across scenarios was rejected because native, content,
  input, and Unity state can leak between runs. Fresh processes make isolation
  clear.

## Acceptance criteria and automated validation

- `cargo battlement init` creates a buildable external game without Unity packages
  or project settings.
- A game copied outside the Battlement repository finds its selected Battlement
  checkout through Cargo metadata and builds successfully.
- A shell cache miss builds from that checkout; a matching subsequent build
  does not invoke Unity for the shell.
- No standard command searches for or downloads a prebuilt shell.
- Rust-only changes rebuild rules and app assembly without rebuilding shell or
  content.
- A content or `.meta` change rebuilds content without rebuilding the shell or
  rules.
- `cargo battlement author` opens the game's content with Battlement packages,
  settings, and renderer, and only intended game-content changes persist.
- Play Mode uses the current rules and content rather than stale installed
  artifacts.
- The content builder rejects executable game content, missing scripts, missing
  declared roots, and roots of the wrong broad Unity type.
- A declared scene includes its models, textures, materials, animation, and
  other referenced dependencies without separate manifest entries.
- Generated address output is byte-identical for identical manifests; stale
  output blocks rules compilation, and removing or retyping an address produces
  an ordinary Rust compiler error at callers after regeneration.
- Basic, Tic-Tac-Toe, and Chess migrate without retaining game-owned packages or
  project settings.
- Chess's authored scene renders with the standard shell and remains editable in
  the authoring workspace.
- Rust logs written to native stderr appear in the terminal and
  `native-stderr.log` without changing the native ABI or forwarding records
  into Unity.
- A failed or crashed run retains Unity and native logs and reports its exit
  status and available crash information.
- The development and release viewers display current and rolling-average FPS
  and suppress game input while focused.
- A scenario drives simulated pointer and keyboard input through the Unity Input
  System and Battlement action path.
- Each scenario uses a fresh player and stops the owned process after success,
  failure, crash, interruption, and timeout.
- PNG capture succeeds without FFmpeg.
- Golden mismatch produces actual and difference images; `--accept` changes a
  golden only after all scenario behavior succeeds.
- Scenario control opens no network listener and cannot inspect or mutate Unity
  or Rust object state.
- Release validation finds the console and FPS viewer when diagnostics are
  enabled, and finds no file-command handler, simulated input, or capture
  implementation.
- App assembly installs the exact rules and content selected by the manifest,
  verifies signing, and preserves a previous valid output on failure.

Focused tests cover manifest parsing, path confinement, content-root validation,
generated-address determinism and stale detection, cache fingerprints, app
assembly planning, and stderr capture. Unity Editor tests cover the standard
authoring workspace, transitive content builds, executable-content rejection,
and development/release stripping.

Black-box tests build copied external games, record whether Unity was invoked,
edit Rust and content independently, open and build Chess content, launch a
player, preserve logs after forced failure, execute simulated input, capture a
PNG, compare a golden, and inspect release output. Existing fake-client tests
continue to validate Rust rules without Unity.

## Manual QA

1. Clone Battlement, create a game elsewhere on disk, and run it. Confirm that the
   first build creates a local shell and that a second Rust-only run does not
   invoke Unity.
2. Open the console in development and release players. Confirm Unity logs and
   FPS are visible in both, typing into the console does not trigger game input,
   and Rust logs continue to appear in the terminal rather than the viewer.
3. Force a Rust error and then terminate the player unexpectedly. Confirm the
   CLI preserves native stderr, the Unity player log, nonzero exit status, and
   any available macOS crash-report location.
4. Open migrated Chess with `cargo battlement author`. Edit the main scene, move a
   KayKit model, change an importer setting, and save. Confirm the scene and
   `.meta` edits persist while Battlement packages and project settings remain
   outside the game repository.
5. Enter Play Mode from the Chess authoring workspace. Confirm it runs the
   current Rust rules with the edited content and standard bootstrap.
6. Build Chess and inspect its game catalog. Confirm only declared public roots
   have Battlement addresses while their referenced models, textures, and materials
   are present and render correctly.
7. Add, remove, rename, and retype declared addresses. Confirm generation
   produces a deterministic reviewed diff, stale output blocks build, and Rust
   callers fail to compile until they use the new typed constants.
8. Add a C# file and then a missing script reference to game content. Confirm
   each content build fails with the responsible path and does not replace the
   previously built application.
9. Run a pointer-and-keyboard scenario. Confirm virtual input does not move the
   physical pointer or require Accessibility permission and that the interaction
   reaches Rust and changes rendered output.
10. Run a screenshot scenario with FFmpeg absent. Change a visible asset, inspect
   the actual and difference images, then accept the new golden. Confirm only a
   fully successful run updates the checked-in image.
11. Build a release application and inspect its files and assemblies. Confirm it
    is signed, contains the console and FPS surface when diagnostics are enabled,
    and contains no file-command handler, simulated input, or capture code.
