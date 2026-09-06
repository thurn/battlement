# Source map and reading guide for relocated code

Tasks name the source areas they change. Use this page to locate their current
implementation without reading whole directories. Related pages:
[architecture](architecture.md), [workflow](workflow.md), and
[validation](validation.md). Task 06 must update these pointers after moving
crates so later assignments still lead to the right files.

## Existing implementation roles

These are starting points, not instructions to load whole directories.

| Role | Current source |
| --- | --- |
| Application and engine integration | [app.rs](../../crates/battlement-reactant/src/app.rs), [app_engine.rs](../../crates/battlement-reactant/src/app_engine.rs) |
| Tree representation and construction | [render.rs](../../crates/battlement-reactant/src/render.rs), [render_tree.rs](../../crates/battlement-reactant/src/render_tree.rs) |
| Runtime and commit receipt | [runtime.rs](../../crates/battlement-reactant/src/runtime.rs), [commit.rs](../../crates/battlement-reactant/src/commit.rs) |
| Native tree mutation planning | [reconcile.rs](../../crates/battlement-reactant/src/reconcile.rs) |
| Hooks and context | [hooks.rs](../../crates/battlement-reactant/src/hooks.rs), [context.rs](../../crates/battlement-reactant/src/context.rs) |
| Keys, refs, portals, presence | [key.rs](../../crates/battlement-reactant/src/key.rs), [element_ref.rs](../../crates/battlement-reactant/src/element_ref.rs), [portal.rs](../../crates/battlement-reactant/src/portal.rs), [presence.rs](../../crates/battlement-reactant/src/presence.rs) |
| Stores and subscriptions | [external_store.rs](../../crates/battlement-reactant/src/external_store.rs) |
| Motion targets/controls/layout | [motion.rs](../../crates/battlement-reactant/src/motion.rs), [animation_controls.rs](../../crates/battlement-reactant/src/animation_controls.rs), [layout.rs](../../crates/battlement-reactant/src/layout.rs) |
| Input and focus | [event_dispatch.rs](../../crates/battlement-reactant/src/event_dispatch.rs), [focus.rs](../../crates/battlement-reactant/src/focus.rs) |
| Existing asynchronous executor | [executor.rs](../../crates/battlement-reactant/src/executor.rs); this is the resource spawner, not the new rules executor |
| Existing exported-engine fixture | [fixture entry](../../crates/battlement-native/tests/fixtures/exported-engine/src/lib.rs), [release scenarios](../../crates/battlement-native/tests/fixtures/exported-engine/src/release_scenarios.rs) |
| Generic persistent-data host support | [Connect messages](../../crates/battlement/src/messages.rs), [Unity connect construction](../../Packages/com.battlement.client/Runtime/Host/BattlementRunner.cs); durable browser flush is a task 43 addition |
| Native C ABI and Engine | [engine.rs](../../crates/battlement-native/src/engine.rs), [lib.rs](../../crates/battlement-native/src/lib.rs) |
| Protocol messages and commands | [messages.rs](../../crates/battlement/src/messages.rs), [body.rs](../../crates/battlement/src/commands/body.rs), [objects.rs](../../crates/battlement/src/objects.rs) |
| Unity runner | [BattlementRunner.cs](../../Packages/com.battlement.client/Runtime/Host/BattlementRunner.cs) |
| Unity batches and snapshots | [BattlementBatchScheduler.cs](../../Packages/com.battlement.client/Runtime/Host/BattlementBatchScheduler.cs), [BattlementSnapshotReplacement.cs](../../Packages/com.battlement.client/Runtime/Host/BattlementSnapshotReplacement.cs) |
| Unity world and tween execution | [BattlementWorld.cs](../../Packages/com.battlement.client/Runtime/Host/BattlementWorld.cs), [BattlementTweenAdapter.cs](../../Packages/com.battlement.client/Runtime/Host/BattlementTweenAdapter.cs) |
| Unity UI Motion | [BattlementMotionTimeline.cs](../../Packages/com.battlement.client/Runtime/UI/BattlementMotionTimeline.cs), [BattlementMotionWorld.cs](../../Packages/com.battlement.client/Runtime/UI/BattlementMotionWorld.cs) |
| World fake | [client.rs](../../crates/battlement-fake/src/client.rs), [executor.rs](../../crates/battlement-fake/src/executor.rs), [tween.rs](../../crates/battlement-fake/src/tween.rs) |
| UI fake | [lib.rs](../../crates/battlement-ui-fake/src/lib.rs) |
| Plugin builds | [plugin_build.rs](../../crates/battlement-cli/src/plugin_build.rs) |
| Sample build/author preparation | [sample.rs](../../crates/battlement-cli/src/sample.rs), [author.rs](../../crates/battlement-cli/src/author.rs) |
| Unity release/adapter builders | [BattlementSampleBuild.cs](../../Packages/com.battlement.client/Editor/BattlementSampleBuild.cs), [BattlementDittoBuild.cs](../../Packages/com.battlement.client/Editor/BattlementDittoBuild.cs) |
| Reactant asset pipeline | [reactant_assets.rs](../../crates/battlement-cli/src/reactant_assets.rs), [asset_generator.rs](../../crates/battlement-reactant/src/asset_generator.rs) |
| Repository validation | [ci.py](../../scripts/ci.py), [CI skill](../../.agents/skills/battlement-ci/SKILL.md) |
| Existing game tests | [tic-tac-toe](../../samples/tictactoe/rules/tests/gameplay.rs), [chess](../../samples/chess/rules/tests/gameplay.rs) |
| Chess AI and saves | [ai.rs](../../samples/chess/rules/src/ai.rs), [persistence.rs](../../samples/chess/rules/src/persistence.rs) |
| Existing sample declarations | [sample guidance](../../samples/AGENTS.md), selected sample's rules/src and ditto.toml |

## Planned additions

The numbered tasks introduce the following code:
- reactant-rules: execution modes, choice specifications, worker communication,
  worker lifecycle.
- reactant-core/reactant-ui/reactant: extracted runtime and public facade.
- reactant-testing: public display driver and observation/barrier APIs.
- samples/hearts: rules, views, authoring inputs, tests, Ditto configuration.
- Generic host transaction and frame acknowledgement protocol.
- Host-neutral Motion property adapters and completion-relative scheduling.

Use the corresponding task to create them; do not mistake a same-named existing
asynchronous resource executor for the rules executor.

## External references

These references explain animation, layout, and Rust unwind behavior used by the
engine. The requirements to implement are fully specified in this plan:
- [Motion animation](https://motion.dev/docs/react-animation)
- [Motion useAnimate](https://motion.dev/docs/react-use-animate)
- [Motion sequences](https://motion.dev/docs/animate#timeline-sequences)
- [Taffy](https://github.com/DioxusLabs/taffy)
- [Rust
  resume_unwind](https://doc.rust-lang.org/std/panic/fn.resume_unwind.html)
- [Rust catch_unwind](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html)
- [Rust FFI
  unwinding](https://doc.rust-lang.org/nomicon/ffi.html#ffi-and-unwinding)
- [KayKit assets](https://kaylousberg.itch.io/board-game-bits)

## Manual QA

Follow a task's source-role links and identify its actual caller and host/fake
counterpart. After any crate move, repeat this check for the remaining tasks and
repair pointers in this document before handing off.
