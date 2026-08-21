# Masonry Standard Setup Technical Design

Status: proposed extension to the Masonry client and authoring model

## Summary

Masonry is a thin Unity rendering and input client for games whose rules and
authoritative state live in Rust. Today a game can avoid game-specific C# but
must still own a complete Unity project. The Basic, Tic-Tac-Toe, and Chess
samples repeat packages, project settings, render-pipeline configuration,
bootstrap code, and build configuration that belong to Masonry.

The **standard setup** lets a game own Rust rules, a small manifest, ordinary
Unity-authored content, and automated scenarios without owning a complete Unity
project. Developers clone Masonry and build a **standard shell**, a reusable
application containing Unity and the Masonry client but no game rules or game
content. The command-line interface (CLI) installs one game's native rules
library and content into a copy of that shell, applies metadata, and signs the
result.

Game content remains native Unity content. Scenes, prefabs, models, textures,
materials, and their `.meta` files live in the game repository and are edited in
a disposable Unity project supplied by Masonry. The game declares only the
content roots that Rust addresses directly. Unity includes their referenced
dependencies. Masonry does not reproduce Unity importer settings in TOML,
derive Unity asset identifiers, or generate Rust source code for asset names.

Development builds provide practical diagnostics and repeatable evidence
without a network control protocol. Rust logs go to stderr and a bounded native
queue. Unity forwards queued records to the ordinary Unity log system, where
the pinned UnityIngameDebugConsole dependency displays them beside Unity logs
and current frame-rate information. The CLI always preserves player and native
logs for failed or crashed runs.

Automated scenarios use the existing visual-capture model: the CLI starts one
player with a private directory and exchanges one acknowledged file command at
a time. A compact TOML scenario can wait for readiness, drive simulated Unity
Input System devices, wait for frames or time, capture a screenshot, and compare
it with a checked-in golden image. There is no TCP listener, authentication
handshake, subscription stream, reconnection behavior, or general runtime
inspection API.

Games that need custom C#, custom shaders, additional Unity packages, a
different rendering pipeline, or unsupported project settings continue to use
Masonry's advanced bring-your-own-Unity-project path.

## Related information

- [Masonry Technical Design](technical-design.md) defines the Rust and Unity
  gameplay protocol. Standard setup changes project ownership and packaging,
  not the rule-authority boundary.
- [Native plugin development](native-plugin-development.md) defines the current
  native library installation, verification, architecture, and signing behavior
  reused during app assembly.
- [Visual evidence capture](visual-capture.md) defines the existing simulated
  input, acknowledged file commands, framebuffer capture, and cleanup behavior
  reused by standard scenarios.
- [Fake client design](fake-client-design.md) defines the Rust-only substitute
  for Unity. Player scenarios complement rather than replace fake-client tests.
- [UnityIngameDebugConsole v1.8.9][console] at commit
  `4467c225eaaf5c0db62a11e2c6851a9fdb64763c` supplies the development log
  viewer. Its [MIT license][console-license] remains in distributions that
  contain it.

[console]: https://github.com/yasirkula/UnityIngameDebugConsole/tree/4467c225
[console-license]:
  https://github.com/yasirkula/UnityIngameDebugConsole/blob/4467c225/LICENSE.txt

## Problem and current state

The reusable Unity package already provides `MasonryRunner`,
`MasonryBootstrap`, embedded and HTTP-hosted rule transports, MessagePack
serialization, Addressables storage, object construction, grouped command
execution, pointer and keyboard input, and error reporting. A game with no
custom C# still has to place those components into a scene and maintain the
Unity project around them.

Basic and Tic-Tac-Toe each own a complete `Assets`, `Packages`, and
`ProjectSettings` set. Most files are the same Masonry infrastructure. Basic
needs only its Rust behavior and a few colors. Tic-Tac-Toe additionally needs
three PNG files.

Chess demonstrates the other important case. It has a substantial Unity scene
and a library of imported FBX models. The scene, models, textures, materials,
and `.meta` files are genuine game content that must remain editable in Unity.
Its package manifest, input configuration, Universal Render Pipeline (URP)
settings, and other project settings are still duplicated infrastructure.

The current `cargo masonry sample` command assumes the game lives inside the
Masonry repository. It locates a sample-specific manifest, builds the current
machine's native rules library, installs it into that sample's Unity project,
and asks Unity to build the project. This proves that Rust-authored standalone
games work, but it makes external game repositories and fast Rust-only
iteration awkward.

The root Masonry repository is already the authoritative Unity project. It pins
the supported Unity and package versions and owns the standard renderer,
bootstrap, client package, input configuration, and build support. Standard
setup reuses that project as the source of both the player shell and disposable
content-authoring workspaces.

## Goals and invariants

- A game repository works from any location on disk.
- A developer explicitly clones Masonry and uses that checkout as the standard
  shell source. Standard setup never downloads a prebuilt shell.
- Rust remains the owner of game rules and authoritative game state.
- Every packaged application contains one game's native Rust engine and one
  game catalog. It is not a universal launcher.
- Masonry owns Unity packages, project settings, bootstrap, render pipeline,
  Addressables settings, and standard assets.
- Games own Unity-authored content and its `.meta` files without owning Unity
  infrastructure.
- Chess can migrate without flattening, translating, or reauthoring its scene
  and imported models.
- Unity remains the source of truth for importer settings and asset identifiers.
- The manifest names only assets that Rust must address directly. Unity resolves
  their transitive dependencies.
- No command generates or rewrites Rust source code for asset addresses.
- Rust-only changes rebuild the rules library and assembled app without
  rebuilding the shell or game content.
- Development runs preserve Unity logs, Rust logs, exit status, and available
  crash information.
- A development player can show Unity and Rust logs and current frame rate while
  the game is running.
- Each automated scenario starts a fresh player and drives Unity's normal Input
  System path.
- Screenshot comparison operates on final rendered pixels and produces useful
  failure artifacts.
- Development-only console, simulated input, and capture code are absent from
  release players.
- Failed builds and captures do not replace the last valid output.
- Existing advanced Unity projects remain supported.

The first supported platform is native macOS. Windows, Linux, mobile, WebGL,
HTTP-hosted rules, multiple render pipelines, and Apple notarization are outside
this contract.

## Developer experience

The CLI discovers a game by searching from the working directory toward the
filesystem root for `masonry.toml`. An explicit manifest path disables search.
Every relative manifest path resolves from the manifest's directory.

The supported commands are:

- `init [path]` creates a manifest, Rust rules package, starter content
  directory, starter scenario, and ignore rules without overwriting work.
- `doctor` checks the manifest, Cargo dependencies, Masonry checkout, required
  Unity version, shell and content caches, and signing requirements.
- `build [--release]` resolves or builds the shell, builds changed rules and
  content, assembles the game app, signs it, and prints the output path.
- `run [--release]` performs the incremental build, launches the app, tails its
  logs, and preserves run artifacts after exit.
- `author` opens a disposable Unity project containing Masonry infrastructure
  and the game's directly editable content.
- `scenario run <name>` launches a fresh development player and executes one
  TOML scenario. `--all` executes every scenario independently and `--accept`
  replaces otherwise-successful golden screenshots.

`generate` is not a standard-setup command. Rust code constructs typed
addresses from the stable manifest IDs it uses. Build and doctor validate that
every declared root appears once in the built catalog.

Unity is required to build an absent shell, build changed content, and author
content. An exact cached shell and content pack let Rust-only `build`, `run`,
and scenario iterations avoid the Unity Editor. Screenshot capture does not
require FFmpeg.

`build --release` always signs. Ad hoc signing is the default and produces a
locally valid application without a trusted developer certificate. A named
Developer ID identity resolves through the macOS Keychain. Notarization and
stapling are not performed.

## Game manifest

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
`cdylib` target named `masonry_rules`. Feature selection is allowed through
declared Cargo features. Arbitrary build commands, environment variables, link
flags, and output paths are not manifest features.

The CLI uses Cargo metadata to locate the game's `masonry` and
`masonry-native` path dependencies. Both must come from the same Masonry
checkout and match the checkout's package versions. This checkout is also the
shell and authoring-project source.

The complete schema-1 manifest surface is:

- `schema`, which must be `1`.
- Required `game.id`, `game.name`, and `game.version` fields.
- Required `rules.manifest` and `rules.package`; optional
  `rules.default_features` and `rules.features`.
- Required `display.width` and `display.height`; optional `display.mode`,
  `display.resizable`, `display.frame_pacing`, and `display.target_fps`.
- Optional `diagnostics.level`, `diagnostics.console`, and
  `diagnostics.console_toggle`.
- Required `content.directory` and zero or more `content.addressables` entries.
- Optional `scenarios.directory`, `scenarios.default_timeout_seconds`, and
  `scenarios.golden_tolerance`.
- Optional `macos.bundle_identifier`, `macos.category`,
  `macos.minimum_version`, `macos.build`, and `macos.architectures`.
- Optional `signing.identity` and `signing.entitlements`.

All scalar names above are TOML strings except Boolean feature, console, and
resizable flags; integer dimensions, frame rate, build number, and timeouts; and
floating-point screenshot tolerance. `rules.features` and
`macos.architectures` are string arrays; `rules.default_features` is Boolean.
Content roots use repeated `[[content.addressables]]` tables with required
string `id`, `kind`, and `source` fields.

Enum spellings are `windowed` and `borderless_fullscreen` for display mode;
`vsync`, `unlimited`, and `fixed` for frame pacing; `trace`, `debug`, `info`,
`warn`, and `error` for diagnostics; and `scene`, `prefab`,
`particle_effect`, `material`, `texture`, `audio_clip`, and `font` for content
kind. Architectures are `arm64` and `x86_64`. `console_toggle` is one Unity
Input System key name and defaults to `Backquote`. Signing identity `-` means ad
hoc signing; every other nonempty value is a Keychain identity name.

Defaults are development-oriented: windowed, non-resizable, vertical
synchronization, info-level diagnostics, console enabled, a 20-second scenario
timeout, exact screenshot comparison, the current machine's development
architecture, both arm64 and x86_64 for release, and ad hoc signing. Fixed frame
pacing requires `target_fps`; other pacing modes reject it. `doctor` prints all
effective values.

### Display and diagnostics

Display mode is windowed or borderless fullscreen. Windowed builds declare
positive pixel dimensions and whether the window is resizable. Frame pacing is
vertical synchronization, unlimited, or a fixed positive frame rate. Schema 1
has one Masonry-owned URP quality configuration.

Development diagnostics declare a minimum level, whether the in-game console is
enabled, and the console toggle key. Diagnostic levels are trace, debug, info,
warning, and error. Release retains warnings and errors in ordinary logs but
does not contain the console or its toggle handling.

### Content

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
and dependencies outside the game-content and Masonry-owned roots fail the
content build with the dependency path. Custom shader assets and Shader Graphs
are not supported in schema 1; game materials may reference shaders supplied by
Unity, URP, or Masonry.

The validator does not maintain an exhaustive allowlist of Unity components or
serialized asset types. A scene may use built-in, URP, Input System, TextMesh
Pro, and Masonry components supplied by the fixed standard project. This is what
allows a real authored Chess scene to migrate without Masonry recreating a
smaller Unity object model.

### Address use from Rust

The public catalog key is the declared ID. Rust uses Masonry's existing typed
address constructors with that ID. For example, a scene declared as `main`
uses a `SceneAddress` containing `main`; a prefab declared as `explosion` uses a
`PrefabAddress` containing `explosion`.

The manifest and Unity content build provide the type boundary. Runtime also
checks that a catalog entry resolves to its declared Unity type before rules
begin. Generated constants are unnecessary for this small, explicitly declared
surface and are not checked into the game repository.

The compiler cannot validate a string literal against the manifest. A missing
ID or a request made with the wrong typed address therefore fails at the asset
load boundary with the ID, requested kind, and catalog kind in the diagnostic.
Games may define ordinary handwritten constants when they want compiler-checked
reuse. Masonry does not own or regenerate those constants.

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

The **game root** is the directory containing `masonry.toml`. Manifest paths are
relative UTF-8 paths below that root. After resolving
symbolic links, content, scenario, entitlement, and golden-image inputs must
remain within their allowed roots. Outputs are written only below CLI-owned
target directories. Absolute paths, parent traversal, control characters, and
case-insensitive collisions fail validation.

## Unity content authoring

`cargo masonry author` opens a disposable Unity project assembled from the
selected Masonry checkout. The project uses Masonry's packages, project
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
uses the same Masonry bootstrap as the packaged development player. Play Mode
is an iteration convenience, not a second gameplay implementation. If its
inputs are stale or the rules library cannot be built, Unity reports the
actionable failure instead of running an older game silently.

Unity uses Masonry's bootstrap as the Play Mode start scene. The Rust initial
snapshot loads the same declared game scene that a packaged player loads. When
the Editor is not playing, `author` opens the first declared scene root, while
Unity may subsequently remember the developer's last open game scene in the
ignored workspace.

The content builder uses an equivalent disposable project without exposing it
to the user. It marks only the manifest-declared roots as public Addressables,
lets Unity resolve their dependencies, and produces one catalog and bundle set.
It never edits the game source during an ordinary build.

## Standard shell

The root Unity project in the selected Masonry checkout is the only standard
shell source. Developers obtain it by cloning Masonry. Standard setup does not
discover, download, authenticate, publish, revoke, or update shell archives.

The shell contains the production bootstrap scene, Masonry client, standard
renderer, standard assets, native-library slot, configuration loader, catalog
loader, diagnostics, and the code required for its profile. It contains no game
rules or game content.

Schema 1 exposes five standard assets through handwritten constants in the
`masonry::standard` API: the empty scene, default font, white lit material,
white unlit material, and white texture. Their keys remain under the reserved
`masonry/` prefix. Game IDs cannot contain `/`, so they cannot collide with
these shell keys. Changing this standard set requires a content-format change.

Development and release are separate shell profiles. Development contains the
log viewer, FPS display, simulated input, file-command handler, and screenshot
capture. Release omits those assemblies and assets; a dormant runtime flag is
not sufficient separation.

### Shell cache

The CLI reuses a cached shell only when its metadata matches the selected
Masonry checkout, checkout modifications, Unity version, package lock, profile,
architecture, native ABI, and content format. A miss invokes Unity to build the
shell from the checkout.

The checkout identity includes the Git commit and relevant tracked and
nonignored untracked source changes. Unity caches, logs, build outputs, and user
settings are excluded. A corrupt or incomplete cache entry is discarded and
rebuilt. A per-entry lock prevents duplicate concurrent builds. A replacement
becomes visible only after validation, so a failed build leaves the previous
valid entry intact.

This cache is a local performance optimization, not a distribution or trust
system. It does not require signed indexes, public keys, expiration, revocation,
download quarantine, or proof of publication.

## Content build and caching

The content builder creates one Addressables catalog for the game. Public keys
are exactly the declared IDs; Unity dependencies have no public Masonry key
unless separately declared. Catalog entries expose no path, GUID, label, or
game-ID aliases.

The content cache key covers:

- The selected Masonry checkout and content format.
- The supported Unity version and package lock.
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

Standard setup caches three expensive results independently:

- The standard shell, keyed by Masonry and Unity inputs plus profile and
  architecture.
- The game content pack, keyed by game content and its Unity environment.
- The native rules library, keyed by Cargo inputs, profile, and architecture.

Final app assembly is cheap and reruns whenever any installed result or app
metadata changes. The expected invalidation behavior is:

- Rust source rebuilds rules and reassembles the app.
- A Unity content or `.meta` change rebuilds content and reassembles the app.
- An addressable ID, kind, or path change rebuilds content and reassembles the
  app; Rust compilation detects any callers not updated for an ID change only
  when the game defines its own constants.
- Display, diagnostics, version, or bundle metadata only recompiles
  configuration and reassembles the app.
- A signing change only reassembles and re-signs the app.
- A Masonry, Unity, or package change rebuilds shell and content and recompiles
  rules against the selected checkout.
- Switching between development and release selects a different shell and
  rules profile but may reuse unchanged content when its format and target are
  compatible.

Every cached output carries enough input metadata to explain why it was reused
or rebuilt. Commands print concise hit or rebuild decisions. Publication uses a
temporary sibling and atomic replacement so interrupted work cannot appear as a
valid cache result.

## Runtime startup and failure behavior

Player startup performs the following observable work in order:

1. Read and validate compiled game configuration.
2. Initialize and validate the standard catalog.
3. Initialize and validate the game catalog when present.
4. Start development diagnostics and file-command handling when requested.
5. Load the native library and verify architecture, required symbols, and ABI.
6. Create the Rust engine and connect the Masonry runner.
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

Diagnostics serve three audiences: a developer watching the terminal, a player
opening the in-game viewer, and a developer investigating a failed or crashed
run. They use ordinary logs rather than a separate query or subscription
service.

### Rust logging

The native adapter exposes a Masonry logging API to game rules. Each record has
only a level, a short target, and a UTF-8 message. Structured arbitrary fields,
correlation IDs, replay cursors, and per-session subscription semantics are not
part of the standard setup contract.

Standard setup advances the native interface to ABI v2. ABI v2 retains the
existing engine operations and buffer-ownership rules and replaces the v1
marker with a v2 marker. It adds one configuration operation accepting the
minimum log level and one polling operation returning a batch of pending log
records. Unity configures logging immediately after loading the library and
before creating the engine. The CLI and shell require the marker and both
logging operations before launch; a v1-only rules library fails with an
actionable version error. No other diagnostic transport is added to the ABI.

Every Rust record is written immediately to native stderr. This is the durable
path for a process that crashes before Unity can poll it. The adapter also
offers the record to a bounded, nonblocking queue. When that queue is full, it
increments a dropped-record count rather than blocking gameplay or allocating
without bound.

The configured minimum level applies before both stderr output and queue
insertion. Panic and native-adapter fatal messages always reach stderr
regardless of that filter. Records are line-framed on stderr with escaped
newlines and are flushed promptly. Invalid text is replaced safely. Target,
message, and batch sizes are bounded by the ABI decoder even though their exact
limits are not application compatibility promises.

Unity polls batches through one native logging ABI operation during ordinary
frames and shutdown. It forwards each record at the equivalent Unity log level.
When records were dropped, it emits one warning with the count after space
becomes available. Logging failures never change gameplay messages, response
ordering, or authoritative state.

Trace, debug, and info records map to Unity's ordinary log level while retaining
their original level and target in the displayed prefix. Warnings and errors map
to Unity warning and error levels. The queue preserves insertion order; records
from concurrently logging Rust threads have no stronger causal ordering
guarantee. It exists for the lifetime of the loaded native library and survives
engine creation and destruction.

The ABI operation returns owned bytes using the same buffer ownership rules as
the existing native protocol. It may return no records. Exact batching and
queue capacity are implementation choices covered by load tests, not public
compatibility promises.

Panics caught by the native adapter write their message to stderr and return the
existing engine failure where possible. An uncontained process crash may prevent
final queue draining, which is why stderr capture is mandatory.

### Preserved run logs

`cargo masonry run` and the scenario runner launch the app's internal player
executable directly rather than delegating to the macOS `open` command. They
direct Unity's player log to a CLI-owned run directory and capture native stdout
and stderr separately. They tail useful output while the player runs and retain
the complete files and exit status. When macOS produces a matching crash report,
the CLI records its path; the absence of such a report does not hide the other
failure evidence.

Forwarded Rust records intentionally also appear in Unity's player log. While
tailing live output, the CLI labels sources and suppresses the forwarded copy so
one Rust record is normally printed once. The preserved native and Unity files
remain complete rather than being rewritten for deduplication.

The CLI does not depend on the in-game viewer to collect evidence. A player that
never reaches readiness, crashes while rendering, or has a broken console still
leaves the available process and Unity logs behind.

### In-game viewer and FPS

Development shells use the pinned UnityIngameDebugConsole dependency for log
display, filtering, scrolling, and clearing. Masonry configures the dependency
as a viewer over the ordinary Unity log stream; it does not build a competing
log-overlay implementation.

The viewer includes a small Masonry status surface showing current and
rolling-average frames per second and basic connection state. It is a compact
panel beside the log viewer and updates at a human-readable cadence rather than
every rendered frame. The status command prints the same values on demand.

Reflection-based command discovery and arbitrary evaluation are disabled.
Masonry may intentionally register help, status, clear, log-level, screenshot,
and quit commands. Console focus suppresses game keyboard actions, and Unity UI
hit testing prevents console interaction from reaching the game world.

The log-level command changes only the viewer filter for the current process;
it does not change native emission or persist to the manifest. The pinned
console source and license are carried by the Masonry Unity project, not fetched
or supplied by each game.

The entire viewer, FPS surface, and toggle input are development-only. Release
players retain ordinary warning and error logging without viewer code or assets.

## Local scenario automation

Standard scenarios reuse the existing in-player simulated input and framebuffer
capture implementation. The CLI creates a private directory with restrictive
permissions, starts one development player with that directory, and exchanges
atomically published JSON files. The directory path is not a secret; access is
controlled by normal filesystem permissions and ownership of the launched
process.

The CLI passes the absolute control-directory path as a launch argument. It
creates a new empty directory for every run and fails if that directory already
contains protocol files. The player publishes `ready.json` after the initial
rendered frame. Startup failures that occur earlier are observed through process
exit and preserved logs rather than through the command protocol.

`ready.json` is a JSON object containing exactly `ready = true`, rendered
`width`, and rendered `height`. A request object contains exactly unsigned
integer `id`, string `operation`, and object `params`. A response contains the
same `id` and exactly one of an object `result` or an `error` object containing
string `code` and human-readable `message`. Unknown object members are malformed
input rather than ignored extensions; a future protocol version may add them.

Only one request may be outstanding. A request has a monotonically increasing
integer ID, an operation, and operation parameters. The player writes one
success or failure response with the same ID after the operation completes.
Malformed input fails that scenario run; reconnect, replay, multiplexing, and
concurrent-client behavior do not exist.

IDs are unsigned 64-bit integers starting at 1 for each run. The CLI writes a
`request-<id>.json.tmp` file and renames it to `request-<id>.json` only after
the complete file is durable. The player applies the same rule to
`response-<id>.json.tmp` and `response-<id>.json`. A response contains either a
result or an error with a stable code and human message. Files are limited to
64 KiB. Wrong, duplicate, out-of-order, partial, or unexpected protocol files
fail the run without executing their operation.

The supported operations are:

- `status`, with empty parameters, returns readiness and rendered dimensions.
- `pointer_move` accepts normalized numeric `x` and `y`.
- `pointer_down` and `pointer_up` use empty parameters for the primary button.
- `key_down` and `key_up` accept string `key`.
- `wait_frames` accepts positive integer `frames` and returns the completed
  rendered-frame count.
- `capture_png` accepts safe relative string `path` and returns that path plus
  captured width and height after publication.
- `shutdown`, with empty parameters, acknowledges before graceful exit.

Successful input operations return only `accepted = true`. Protocol errors use
the stable codes `invalid_request`, `invalid_operation`, `invalid_params`,
`invalid_input_transition`, `capture_failed`, and `shutting_down`. Player or
engine failures use ordinary logs and process exit rather than pretending to be
recoverable command errors.

The CLI may implement convenient click, key-press, and real-time-wait scenario
steps by composing these operations and its own timer. Input transitions are
balanced. Repeating a press, releasing an unheld input, or ending successfully
with held input fails the scenario. Simulated devices traverse Unity's Input
System, UI or collider hit testing, Masonry actions, Rust rules, and rendering.

The player never accepts operations that enumerate or mutate scenes, objects,
components, Addressables, or Rust state. The protocol exists only to reproduce
user input and capture observable output.

An outstanding command is not cancellable. On timeout, malformed command,
unexpected exit, or CLI interruption, cleanup best-effort releases held input,
requests shutdown when the player still responds, and otherwise terminates the
owned player process. A crash may prevent input release inside that already
exiting process. The failure report and available logs and captures remain
available.

## Scenario contract

A scenario is a strict TOML document with a stable name, an optional
description, an overall timeout, optional window dimensions, and an ordered
list of steps. Unknown fields and operations are errors. Every scenario starts
a fresh development player and has its own output directory.

The file declares `schema = 1`, `name`, optional `description`, optional
`timeout_seconds`, an optional `window` table containing `width` and `height`,
and one or more `steps` tables. `scenario run <name>` selects the declared name,
which must equal the TOML file stem. Names are unique lowercase kebab case.
Discovery is nonrecursive and includes nonhidden `.toml` files only. `--all`
continues after independent failures so it can collect evidence for every
scenario.

Schema 1 supports these steps:

- Wait for player readiness.
- Wait for a positive real-time duration.
- Wait for a positive rendered-frame count.
- Move the pointer to normalized top-left coordinates.
- Click the primary pointer button at its current or supplied coordinates.
- Press or release the primary pointer button for drag scenarios.
- Press a keyboard key.
- Press or release a keyboard key for held-key scenarios.
- Capture a PNG, optionally compare it with a checked-in golden image, and use
  an optional comparison tolerance.

Every step declares `action` and may declare `timeout_seconds`. Pointer actions
use finite `x` and `y` values from zero through one. Keyboard actions use Unity
Input System key names and are layout-independent physical controls. Wait steps
declare positive `seconds` or `frames`. Screenshot steps declare a unique safe
relative `name`, optional `golden` path below the game root, and optional
`tolerance` from zero through one. A scenario has at most 1,000 steps, lasts at
most ten minutes, and captures at most 100 images.

Steps execute in document order. A step may shorten its timeout but cannot
extend the scenario deadline. An unexpected process exit fails the active step
immediately. A scenario that finishes its steps requests graceful shutdown and
expects exit code zero.

The deliberately small schema has no branching, loops, object selectors,
runtime state mutation, diagnostic subscriptions, log-history cursors, or
arbitrary methods. A scenario that needs complex state setup should express it
through deterministic Rust rules, a dedicated game fixture, or ordinary user
input rather than expanding the player-control surface.

### Screenshot comparison

The player captures the completed framebuffer after rendering. PNG publication
is acknowledged only after the file is complete. The CLI decodes actual and
golden images to red, green, blue, and opacity channels and requires equal
dimensions.

The comparison score is normalized mean absolute channel error: the sum of the
absolute differences for all red, green, blue, and opacity samples divided by
the number of samples times 255. Tolerance is the largest passing score. A
mismatch preserves the actual image and creates a highlighted difference image.
PNG metadata and compression do not affect comparison.

Scenario window dimensions are rendered-pixel dimensions and do not change with
Retina display scaling. Standard scenarios use the shell's fixed renderer,
color space, quality settings, antialiasing, and supported Unity version.
Goldens are specific to the standard macOS shell profile; small remaining GPU
or font-rasterization differences are handled through an explicit nonzero
tolerance rather than an implicit platform adjustment. Scenarios wait for
readiness and explicit frame or time steps before capture; Masonry does not
claim that arbitrary real-time animation is pixel deterministic.

`--accept` replaces a golden only when every input, wait, capture, shutdown, and
other scenario behavior succeeds. It never turns an otherwise failed scenario
into success. A run stages all accepted images and publishes them as one group
after successful shutdown; failure leaves every existing golden unchanged.

Each run retains a concise machine-readable result containing step outcomes,
timings, player exit, screenshot hashes and comparison scores, plus the Unity
player log and native output. These artifacts are the fixed evidence supplied
to humans or AI reviewers.

## Security and release separation

Standard game executable content is limited to the one Rust rules library.
Game Unity content cannot add C#, assemblies, native plugins, Editor scripts,
packages, or project settings. The content builder reports forbidden files and
missing dependencies before publication.

All input and output paths are confined to their declared roots after symbolic
link resolution. Temporary and scenario control directories use restrictive
permissions. Cleanup targets only process identities started by the CLI.

The local scenario mechanism does not open a socket and has no remotely
reachable surface. It does not need authentication tokens because the CLI owns
the private directory and child process. File sizes, screenshot dimensions,
step counts, and scenario duration are bounded to prevent accidental resource
exhaustion.

Release validation inspects the build report and assemblies to prove the log
viewer, FPS surface, file-command handler, simulated input, and capture code are
absent. Native stderr and ordinary Unity warning and error logs remain available
for release diagnosis.

Credentials remain in Keychain or the release environment. They never enter
the manifest, generated configuration, logs, or scenario artifacts.

## Migration

Basic removes its Unity project. Its rules use Masonry's standard empty scene
and default font. Its build-safe colored materials become a small set of
Unity-authored game assets. No generated material declaration or Rust binding
file is needed.

Tic-Tac-Toe removes its Unity project and moves its PNG files, with Unity
`.meta` files, into the game content root. It declares the textures it addresses
directly with stable IDs. Existing Rust code changes its address literals or
game-owned constants to those IDs.

Chess keeps its authored main scene, default volume profile when referenced,
KayKit models, textures, materials, and all associated `.meta` files as game
content. It drops its package manifest, project settings, input settings, URP
pipeline assets, global settings, and other Masonry-owned infrastructure. The
main scene is one declared addressable root; its model and texture references
are included transitively. The scene is opened and edited through
`cargo masonry author` using Masonry's standard renderer and packages.

The current Chess scene audit finds no C#, assembly, plugin, custom shader, or
Shader Graph asset. Its serialized model references resolve to checked-in
KayKit `.meta` identifiers. Its only script components are URP's additional
camera and additional light data, both supplied by the standard project; its
remaining special identifiers are Unity built-ins. The existing default volume
profile is not referenced by the current main scene and need not migrate unless
later content begins using it. Migration validation repeats this dependency
audit against the exact scene rather than assuming the inventory remains
unchanged.

The Masonry bootstrap remains the application start scene. Rust's initial
snapshot loads the declared Chess `main` Addressable scene, so Chess does not
replace or embed bootstrap behavior in its authored scene. A checked-in Chess
golden and a simple move scenario provide the visual and behavioral equivalence
gate after migration.

Repository-specific sample discovery is replaced by manifest discovery. Tests
copy a migrated game outside the Masonry tree and use its Cargo path
dependencies to find the selected Masonry checkout.

There is no compatibility layer for generated asset bindings, curated importer
tables, generated Unity GUIDs, downloaded shells, or the TCP development
control protocol described by earlier drafts of this design. Those designs were
never a released standard-setup contract.

The reusable package and native-plugin commands remain available for advanced
Unity projects. Migration to standard mode is optional for games that require
custom executable Unity content or project configuration.

## Alternatives considered

- A universal prebuilt launcher was rejected because every output should have
  one game identity, one rules library, and ordinary standalone-app behavior.
- Published shell archives were rejected because developers can clone Masonry
  and build the exact checkout selected by their Cargo dependencies. A signed
  shell distribution system adds security and operations work without serving
  the current workflow.
- Rebuilding the complete Unity app after every Rust edit was rejected because
  Unity dominates iteration time and the shell is independent of game rules.
- A TOML importer schema was rejected because Unity already stores importer
  choices in `.meta` files and exposes mature authoring tools for them.
- Generated Unity GUIDs were rejected because checked-in `.meta` files already
  preserve Unity identity for raw and authored content.
- Generated Rust asset bindings were rejected because standard games declare
  few public roots, existing typed address constructors preserve type intent,
  and generation adds source ownership, fingerprinting, and invalidation rules.
- Enumerating every Unity dependency in the manifest was rejected because
  scenes and prefabs naturally own dependency graphs that Unity already knows
  how to build.
- An exhaustive component and asset allowlist was rejected because it would
  recreate a partial Unity object model and block ordinary authored scenes such
  as Chess. The boundary instead excludes executable content and requires all
  components to resolve from the fixed project.
- A bespoke log overlay was rejected because UnityIngameDebugConsole already
  provides the needed viewing and filtering interface.
- A TCP automation service was rejected because scenarios control a CLI-owned
  local child. A private file-command directory covers fixed input and capture
  without authentication, rate limiting, subscriptions, or reconnect behavior.
- Native operating-system input automation was rejected because it depends on
  focus, accessibility permissions, display arrangement, and physical device
  state. Simulated Unity Input System devices exercise the intended game path.
- Full Unity or Rust runtime inspection was rejected because it couples tests
  to implementation details and bypasses player-visible behavior.
- Reusing one player across scenarios was rejected because native, content,
  input, and Unity state can leak between runs. Fresh processes make isolation
  clear.

## Acceptance criteria and automated validation

- `cargo masonry init` creates a buildable external game without Unity packages
  or project settings.
- A game copied outside the Masonry repository finds its selected Masonry
  checkout through Cargo metadata and builds successfully.
- A shell cache miss builds from that checkout; a matching subsequent build
  does not invoke Unity for the shell.
- No standard command searches for or downloads a prebuilt shell.
- Rust-only changes rebuild rules and app assembly without rebuilding shell or
  content.
- A content or `.meta` change rebuilds content without rebuilding the shell or
  rules.
- `cargo masonry author` opens the game's content with Masonry packages,
  settings, and renderer, and only intended game-content changes persist.
- Play Mode uses the current rules and content rather than stale installed
  artifacts.
- The content builder rejects executable game content, missing scripts, missing
  declared roots, and roots of the wrong broad Unity type.
- A declared scene includes its models, textures, materials, animation, and
  other referenced dependencies without separate manifest entries.
- Basic, Tic-Tac-Toe, and Chess migrate without retaining game-owned packages or
  project settings.
- Chess's authored scene renders with the standard shell and remains editable in
  the authoring workspace.
- Native log records appear on stderr and in the development in-game viewer.
- Queue overflow reports a dropped count without blocking gameplay.
- A failed or crashed run retains Unity and native logs and reports its exit
  status and available crash information.
- The development viewer displays current and rolling-average FPS and suppresses
  game input while focused.
- A scenario drives simulated pointer and keyboard input through the Unity Input
  System and Masonry action path.
- Each scenario uses a fresh player and cleans held input and the owned process
  after success, failure, crash, interruption, and timeout.
- PNG capture succeeds without FFmpeg.
- Golden mismatch produces actual and difference images; `--accept` changes a
  golden only after all scenario behavior succeeds.
- Scenario control opens no network listener and cannot inspect or mutate Unity
  or Rust object state.
- Release validation finds no console, FPS viewer, file-command handler,
  simulated input, or capture implementation.
- App assembly installs the exact rules and content selected by the manifest,
  verifies signing, and preserves a previous valid output on failure.

Automated tests cover strict manifest parsing, path confinement, content-root
validation, cache invalidation, app assembly planning, native logging buffer
ownership, queue overflow, and panic reporting. Unity Editor tests cover the
standard authoring workspace, declared-root typing, transitive content builds,
missing scripts, executable-content rejection, and development/release
stripping.

Black-box tests build copied external games, record whether Unity was invoked,
edit Rust and content independently, open and build Chess content, launch a
player, preserve logs after forced failure, execute simulated input, capture a
PNG, compare a golden, and inspect release output. Existing fake-client tests
continue to validate Rust rules without Unity.

## Manual QA

1. Clone Masonry, create a game elsewhere on disk, and run it. Confirm that the
   first build creates a local shell and that a second Rust-only run does not
   invoke Unity.
2. Open the development console while playing. Confirm Unity and Rust logs are
   both visible, filtering and clearing work, FPS updates, and typing into the
   console does not trigger game input.
3. Force a Rust error and then terminate the player unexpectedly. Confirm the
   CLI preserves native stderr, the Unity player log, nonzero exit status, and
   any available macOS crash-report location.
4. Open migrated Chess with `cargo masonry author`. Edit the main scene, move a
   KayKit model, change an importer setting, and save. Confirm the scene and
   `.meta` edits persist while Masonry packages and project settings remain
   outside the game repository.
5. Enter Play Mode from the Chess authoring workspace. Confirm it runs the
   current Rust rules with the edited content and standard bootstrap.
6. Build Chess and inspect its game catalog. Confirm only declared public roots
   have Masonry addresses while their referenced models, textures, and materials
   are present and render correctly.
7. Add a C# file and then a missing script reference to game content. Confirm
   each content build fails with the responsible path and leaves the prior valid
   content pack unchanged.
8. Run a pointer-and-keyboard scenario. Confirm virtual input does not move the
   physical pointer or require Accessibility permission and that the interaction
   reaches Rust and changes rendered output.
9. Run a screenshot scenario with FFmpeg absent. Change a visible asset, inspect
   the actual and difference images, then accept the new golden. Confirm only a
   fully successful run updates the checked-in image.
10. Build a release application and inspect its files and assemblies. Confirm it
    is signed and contains no console, FPS surface, file-command handler,
    simulated input, or capture code.
