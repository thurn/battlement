# Native plugin development

Masonry rules engines can be replaced in an existing macOS Unity player without
rebuilding the Unity client. Install the developer CLI from Cargo:

```sh
cargo install masonry-cli --locked
```

## Unity authoring

From the root of any Masonry Unity game with a `rules/Cargo.toml`, open the game and enter Play
mode with the current native rules engine:

```sh
cargo masonry author
```

The command builds the host architecture's `masonry_rules` cdylib, stages it below
`Assets/Plugins/macOS`, selects Addressables Fast Mode, discovers the scene containing
`MasonryBootstrap`, and opens that scene in the project-pinned Unity Editor. It remains attached
until Unity closes so logs stay visible in the terminal. After editing, use **Masonry > Play Game**
inside Unity to enter Play mode again.

Use `--project` when the Unity project is not the current directory, `--manifest-path` when its
rules manifest is not `rules/Cargo.toml`, and `--scene` when the project has more than one scene
containing `MasonryBootstrap`:

```sh
cargo masonry author \
  --project path/to/game \
  --manifest-path path/to/rules/Cargo.toml \
  --scene Assets/Scenes/Main.unity
```

## Packaged players

Build and install a rules-engine `cdylib` in a stopped Unity application:

```sh
cargo masonry plugin install \
  Build/MyGame.app \
  --package my-game-rules \
  --release
```

The CLI builds every architecture present in the packaged plugin and combines
the slices when the Unity player is universal. The Cargo package must expose a
`cdylib` target named `masonry_rules`. Use `--manifest-path` when invoking the
command from outside the rules package's workspace.

The first install saves the originally packaged library beside the application
at `MyGame.app.masonry-backup/`. Later installs preserve that original backup.
The command validates Mach-O architectures and the complete Masonry v1 ABI,
atomically replaces `Contents/PlugIns/libmasonry_rules.dylib`, and ad-hoc signs
the library and application for local development.

Use a named signing identity when the development environment requires one:

```sh
cargo masonry plugin install Build/MyGame.app libmasonry_rules.dylib \
  --sign "Apple Development: Developer Name (TEAMID)"
```

Pass `--no-sign` only when another build step will sign the modified application.
Replacing code invalidates an existing application signature until signing is
complete.

An already-built library can be installed directly instead:

```sh
cargo masonry plugin install \
  Build/MyGame.app \
  target/release/libmasonry_rules.dylib
```

Inspect or validate artifacts without changing them:

```sh
cargo masonry plugin verify target/release/libmasonry_rules.dylib
cargo masonry plugin inspect Build/MyGame.app
```

Restore the originally packaged plugin when an experiment is complete:

```sh
cargo masonry plugin restore Build/MyGame.app
```

The CLI is intentionally separate from the `masonry` and `masonry-native`
runtime crates, so Unity rules engines do not compile or ship developer-tool
dependencies.
