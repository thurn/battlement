---
name: battlement-reactant
description: Find Reactant component authoring, reconciliation, host behavior, and UI examples when building declarative UI or changing its runtime.
---

# Reactant grounding

Begin with `crates/battlement-reactant/src/lib.rs`, `prelude.rs`, and `app.rs`
for the public authoring surface and engine integration. Component code renders
through `Component` and `Render`; inspect a nearby working component before
choosing an abstraction.

| Concern | Starting point under `crates/battlement-reactant/src/` |
| --- | --- |
| Components and host composition | `component.rs`, `components/`, `host.rs` |
| State and lifecycle | `hooks.rs`, `hook_storage.rs`, `effect.rs` |
| Committed tree and updates | `runtime.rs`, `reconcile.rs`, `commit.rs` |
| Input and focus | `event_dispatch.rs`, `control_behavior.rs`, `focus.rs` |
| Layout and geometry | `layout.rs`, `geometry.rs`, `element_ref.rs` |
| Motion and generated paint | `motion.rs`, `asset_generator.rs` |

The UI protocol lives in `crates/battlement-ui`; Unity behavior lives in
`Packages/com.battlement.client/Runtime/`. Trace those layers when changing
host interaction or rendering. Do not infer browser behavior from familiar
React names or CSS-like properties.

Use `samples/reactant/rules/src/` for focused examples and
`samples/chess-ui/rules/src/` for composed application UI. Select the relevant
component or test; do not read every screen or reconstruct a feature ledger.

`crates/battlement-reactant/tests/` exercises authoring and runtime behavior.
The selected sample's `ditto.toml` supplies native host scenarios. Use
`battlement-ditto` to probe visible behavior and `battlement-build` when asset
or player generation is needed.
