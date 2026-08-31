# Reactant V1 Feature Ledger

This ledger maps every public `battlement-reactant` module to the focused lab
screen that teaches its purpose and to black-box tests that prove its public
behavior. The release test reads the crate module declarations and this table,
so an added public module cannot ship without a screen and test mapping.

The screen is a focused specimen, not an exhaustive API catalog. The listed
tests remain the authoritative behavior proof when a capability is broader
than the specimen.

| Public module | Lab screen | Black-box proof |
|---|---|---|
| `component` | Composition | `composition.rs::composition_action_reorders_and_restores_the_badges`, `primitives.rs` |
| `context` | Context & Memo | `composition.rs::context_screen_overrides_only_the_nested_descendant_and_restores`, `refs_context.rs` |
| `element_ref` | Refs & Geometry | `composition.rs::refs_screen_samples_world_geometry_and_restores_an_unavailable_target`, `element_refs.rs` |
| `error_boundary` | Resources & Boundaries | `composition.rs::resources_screen_catches_resets_and_restores`, `error_boundaries.rs` |
| `event` | Events & Portals | `composition.rs::events_screen_runs_and_restores_one_logical_event_path`, `event_catalog.rs`, `propagation.rs` |
| `executor` | Resources & Boundaries | `composition.rs::resources_screen_catches_resets_and_restores`, `resources.rs` |
| `external_store` | Effects & Stores | `composition.rs::effects_store_swaps_updates_and_restores_its_external_snapshot`, `external_stores.rs` |
| `geometry` | Refs & Geometry | `composition.rs::refs_screen_samples_world_geometry_and_restores_an_unavailable_target`, `geometry.rs`, `geometry_effects.rs` |
| `hooks` | State & Identity | `composition.rs::state_screen_batches_updates_preserves_keyed_state_and_restores`, `hook_scheduling.rs`, `state.rs` |
| `key` | State & Identity | `composition.rs::state_screen_batches_updates_preserves_keyed_state_and_restores`, `identity.rs`, `moves.rs` |
| `portal` | Events & Portals | `composition.rs::events_screen_runs_and_restores_one_logical_event_path`, `portals.rs`, `external_portals.rs` |
| `prelude` | Composition | `composition.rs::sample_opens_on_an_accessible_composition_screen`, `primitives.rs` |
| `host` | Composition | `composition.rs::sample_opens_on_an_accessible_composition_screen`, `primitives.rs` |
| `props` | Composition | `composition.rs::composition_action_reorders_and_restores_the_badges`, `required_props.rs` |
| `render` | Composition | `composition.rs::composition_action_reorders_and_restores_the_badges`, `runtime.rs`, `identity.rs` |
| `resource` | Resources & Boundaries | `composition.rs::resources_screen_catches_resets_and_restores`, `resources.rs` |
| `runtime` | Effects & Stores | `composition.rs::effects_screen_defers_connection_until_poll_and_restores`, `runtime.rs`, `lifecycle.rs` |
| `suspense` | Resources & Boundaries | `composition.rs::resources_screen_catches_resets_and_restores`, `resources.rs` |

The remaining focused screens exercise the same public surface from another
angle: Context & Memo covers memoized values and callbacks, Effects & Stores
covers passive effects, and Refs & Geometry covers queued host actions and
coherent geometry effects. The complete initial, changed, and restored flows
are checked in `samples/reactant/rules/tests/composition.rs`.

## Reserved React APIs

These names are deliberately unavailable in V1. They are not future sample
features and code must not infer their React contracts from nearby Reactant
APIs.

| Reserved API | V1 status | Reason |
|---|---|---|
| `StrictMode` | Unsupported | Reactant does not perform development-only duplicate rendering or effects. |
| `use_id` | Unsupported | Battlement UI cannot yet preserve accessible cross-element relationships. |
| `use_layout_effect` | Unsupported | Unity cannot expose React's synchronous post-mutation, pre-paint timing boundary. |
| `use_sync_external_store` | Unsupported | Reactant has no server snapshot or hydration contract; `use_external_store` names the supported client-only behavior. |
