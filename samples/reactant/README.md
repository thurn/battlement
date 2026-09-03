# Battlement Reactant sample

This standalone sample presents the declarative Reactant API through a native
Rust rules engine and Battlement UI Toolkit host.

Sample components use `#[builder]` from `battlement_reactant::prelude::*`.
Declare required props with `#[builder(required)]`; other props use `Default`
or an explicit `#[builder(default = expression)]`. Construct components with
`ComponentType::new().prop(value)` in any required-prop order, without `.build()`.
Field documentation appears on the generated setters.

See the [builder guide](../../crates/battlement-builder/README.md) for generic
components, string and option conversions, and callback setters.

```sh
cargo battlement sample build reactant
cargo battlement sample run reactant
cargo battlement sample run reactant --web
```
