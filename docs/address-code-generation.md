# Addressable asset constants

`cargo masonry generate [PROJECT]` asks Unity to enumerate the explicit entries in a
project's active Addressables settings and writes typed Rust constants to
`<project>/rules/src/assets.rs`. When `PROJECT` is omitted, Masonry searches the current
directory and its parents for a Unity project. Use `--output <file>` to choose a
different module file.

The generated file is intended to be checked in. Add `pub mod assets;` to the
rules crate once, then refer to keys through their address hierarchy. For example,
the address `white/king` becomes `assets::white::KING`. The constants use borrowed
static strings, so reading an address does not allocate.

Unity supplies the imported asset type. Scenes, prefabs, materials, textures, audio
clips, and TextMesh Pro fonts receive their corresponding Masonry address type.
Other valid assets receive `UntypedAssetAddress`. Particle-system prefabs use
`PrefabAddress`; Masonry still validates that a prepared particle prefab contains a
particle system at runtime.

Only explicit entries are generated. Labels, GUID aliases, folder children, and
sub-object keys are not expanded. Generation fails for missing assets, empty or
duplicate addresses, catalog-excluded keys, and names that normalize to the same
Rust identifier.

The output is one deterministic generated Rust module, including for large catalogs.
Masonry only replaces files bearing its generated marker and installs replacements
atomically. In CI, run `cargo masonry generate --check [PROJECT]` to
fail when checked-in output differs from Unity's current Addressables configuration.
