# Rust conventions

These conventions also apply to Rust rules under `samples/`.

- Calls use one qualifier (`module::function()`); never import functions directly.
  Type names have no qualifier (`BattleState`); enum values have one (`Zone::Battlefield`).
- Use `crate::`, not `super::`. Keep all imports at file top.
- `mod.rs` and `lib.rs` contain only module and use declarations.
- Order items: private constants/statics, thread-local declarations, public type
  aliases, public constants, traits, structs/enums, functions, then private items.
- Prefer inline expressions over temporary bindings.
- Use macros only with strong justification after considering traits or other
  reusable abstractions.
- Alphabetize Cargo dependencies: internal first, then external.
- Put short doc comments on public items only.

For ownership across crates and Unity, use `battlement-runtime`; for declarative
UI authoring or runtime changes, use `battlement-reactant`. Follow the actual
module and test entrypoints rather than seeking a feature inventory.
