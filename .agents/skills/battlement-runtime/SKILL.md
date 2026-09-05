---
name: battlement-runtime
description: Locate ownership across Battlement protocol, Rust engines, Unity execution, cloud integration, and test doubles when a change crosses runtime layers.
---

# Runtime boundaries

Start at the layer responsible for the behavior; follow its actual callers and
messages across the boundary only as needed.

| Concern | Starting point |
| --- | --- |
| Shared IDs and value types | `crates/battlement-types/src/lib.rs` |
| Engine/client messages and commands | `crates/battlement/src/lib.rs` |
| Serializable UI documents, updates, events | `crates/battlement-ui/src/lib.rs` |
| Rust engine export and native bridge | `crates/battlement-native/src/lib.rs` |
| Unity command execution and input | `Packages/com.battlement.client/Runtime/` |
| Unity authoring and player packaging | `Packages/com.battlement.client/Editor/` |
| Cloud engine integration | `crates/battlement-cloud/src/lib.rs` |
| Headless protocol execution | `crates/battlement-fake/`, `crates/battlement-ui-fake/`, `crates/battlement-cloud-fake/` |
| Concrete engine composition | The selected `samples/*/rules/src/lib.rs` |

Rust describes game state and commands; Unity renders and reports host input.
Reactant produces UI documents and mutations above the UI protocol; it is not
the Unity renderer. Use `battlement-reactant` when that layer owns the issue.

For protocol changes, trace serialization, Unity consumption, returned events,
and relevant fake execution. Search existing messages and tests rather than
inventing a parallel transport or assuming a Rust-only change is sufficient.
Native player scenarios validate host behavior that fake execution cannot.

Use the root and crate manifests to discover dependency direction. Read nearby
black-box tests for the observable contract; a fake's behavior alone does not
establish what the Unity host supports.
