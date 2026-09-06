# Requirement ownership and acceptance coverage

Read this when reviewing task completeness or checking the final implementation.
The [entry point](README.md) defines scope and order. A listed owner must
provide the relevant acceptance evidence; later integration does not excuse an
earlier task from its own deliverable.

## Architecture and execution

| Required behavior | Implementing tasks | Public acceptance |
| --- | --- | --- |
| Core/UI/world ownership and one-way tooling dependency | [06](tasks/06-crate-boundaries.md), [07](tasks/07-asset-tooling-boundary.md), [48](tasks/48-retire-adapters-final-audit.md) | Existing apps compile/behave; dependency and assembly boundaries |
| Shared sync rules, typed choices, no-op simulation primitives | [02](tasks/02-rules-api-simulation.md), [10](tasks/10-typed-prompts.md), [40](tasks/40-hearts-simulation-ai.md) | Same rules outcomes; allocation/codegen evidence; typed answer rejection |
| Real native/threaded-WebGL unwind and mobile paths | [03](tasks/03-native-cancellation.md), [04](tasks/04-threaded-webgl-proof.md), [05](tasks/05-mobile-build-paths.md), [47](tasks/47-release-conformance.md) | Nested cleanup, silent cancellation, real panic, off-main-thread execution |
| Immutable snapshots, single pending capacity, lazy construction | [09](tasks/09-checkpoint-publication.md) | Builders blocked before reservation; old snapshot unchanged |
| Run/request validation and prompt ordering | [10](tasks/10-typed-prompts.md), [25](tasks/25-checkpoint-motion-gates.md), [45](tasks/45-effects-failures-laboratory.md) | Invalid/stale feedback, ordered actionable prompts |
| Accepted final state and game-owned persistence | [11](tasks/11-accepted-action-runtime.md), [12](tasks/12-host-transactions.md), [25](tasks/25-checkpoint-motion-gates.md), [43](tasks/43-hearts-save-resume.md) | Save/next action only after gate and rendered opportunity |
| Cancellation at every stated point and race | [03](tasks/03-native-cancellation.md), [09](tasks/09-checkpoint-publication.md), [10](tasks/10-typed-prompts.md), [45](tasks/45-effects-failures-laboratory.md) | Publication/build/prompt/answer/completion races, cleanup, replacement isolation |
| Active failure versus abandoned output | [11](tasks/11-accepted-action-runtime.md), [12](tasks/12-host-transactions.md), [25](tasks/25-checkpoint-motion-gates.md), [45](tasks/45-effects-failures-laboratory.md) | Failure surface retains accepted state; replacement unaffected |

## Identity, composition, and input

| Required behavior | Implementing tasks | Public acceptance |
| --- | --- | --- |
| One logical UI/world tree and explicit physical roots | [13](tasks/13-mixed-logical-tree.md) | Shared hooks/context and logical portal events |
| Global UUID matching, duplicates, ancestry extraction | [14](tasks/14-global-presentation-identity.md), [44](tasks/44-identity-composition-laboratory.md) | State/ref retention and atomic cross-domain duplicate rejection |
| Type compatibility, removal, fresh incarnation during exit | [15](tasks/15-incarnations-and-removal.md), [27](tasks/27-effect-exit-retention.md) | Old/new visual overlap with independent state/input/effects |
| Props/selectors and stable store revisions | [16](tasks/16-snapshot-selectors-stores.md), [44](tasks/44-identity-composition-laboratory.md) | Equal selectors skip evaluation; no torn generations |
| Sprites/meshes/text, render sorting, independent overrides | [17](tasks/17-world-rendering-primitives.md), [44](tasks/44-identity-composition-laboratory.md) | Native rich-card appearance, material/text isolation |
| Hit-region geometry and Rust-created typed anchors | [18](tasks/18-hit-regions-anchors.md), [26](tasks/26-effect-occurrences.md), [44](tasks/44-identity-composition-laboratory.md) | Correct hit sizes, following/captured attachment and lifetime |
| UI blocking, modal order, capture/bubble, depth/layer ties | [19](tasks/19-unified-pointer-routing.md), [20](tasks/20-world-focus-touch.md), [44](tasks/44-identity-composition-laboratory.md) | Geometric input cases across domains and portals |
| Focus/controller/touch and responsive resize | [20](tasks/20-world-focus-touch.md), [41](tasks/41-hearts-pointer-touch.md), [42](tasks/42-hearts-navigation-menus.md) | Complete Hearts operation and native mixed-input fixture |
| Taffy world Flex/Grid, pure fan/pile/arc, cached rest measurement | [23](tasks/23-world-layout.md), [44](tasks/44-identity-composition-laboratory.md) | Known target poses, localized invalidation, no animated-bounds feedback |
| Display placement independent of rules location | [24](tasks/24-layout-movement-projection.md), [37](tasks/37-hearts-card-layout.md), [44](tasks/44-identity-composition-laboratory.md) | Inspection/browser layout changes without rules mutation |
| Explicit UI/world projection | [24](tasks/24-layout-movement-projection.md), [44](tasks/44-identity-composition-laboratory.md) | Orthographic/perspective screen-space continuity; missing policy rejection |
| Rich neutral composition beyond playing-card textures | [44](tasks/44-identity-composition-laboratory.md) | Faces, badges, rich text, outline, previews, contained cards and controls |

## Motion, commits, and effects

| Required behavior | Implementing tasks | Public acceptance |
| --- | --- | --- |
| Atomic inactive preparation and generation-safe commits | [12](tasks/12-host-transactions.md), [25](tasks/25-checkpoint-motion-gates.md), [45](tasks/45-effects-failures-laboratory.md) | No partial frame/input or precommit side effects; stale preparation discarded |
| Common targets/transitions/variants/values and controls | [21](tasks/21-shared-motion-drivers.md), [22](tasks/22-sequence-dependencies.md) | UI/world/layout/sequence equivalence and native reduced-motion parity |
| Property ownership and responsive local hover | [21](tasks/21-shared-motion-drivers.md), [24](tasks/24-layout-movement-projection.md), [41](tasks/41-hearts-pointer-touch.md) | No accidental overlap or required-drag theft |
| Live layout destinations and continuous retargeting | [24](tasks/24-layout-movement-projection.md), [45](tasks/45-effects-failures-laboratory.md) | Velocity/pose continuity and true arrival after reflow |
| Fixed/completion-relative sequence scheduling | [22](tasks/22-sequence-dependencies.md), [25](tasks/25-checkpoint-motion-gates.md) | Actual arrival labels, same-time declaration order, invalid graph rejection |
| Checkpoint registrations and exact-once preparation/commit | [25](tasks/25-checkpoint-motion-gates.md), [26](tasks/26-effect-occurrences.md) | Rerenders/retries create one playback/occurrence |
| Required/cosmetic gates, early labels, replacement races | [25](tasks/25-checkpoint-motion-gates.md), [45](tasks/45-effects-failures-laboratory.md) | Permanent accepted satisfaction; successor identity; no stale gate event |
| Sound/burst occurrence identity and typed Rust/RON config | [26](tasks/26-effect-occurrences.md), [45](tasks/45-effects-failures-laboratory.md) | Deduplication, simultaneous effects/fallbacks, captured configuration |
| Exits/projectiles retain original resources | [27](tasks/27-effect-exit-retention.md), [45](tasks/45-effects-failures-laboratory.md) | Logical cleanup immediately, visual release after last use |
| Seeking, delivered history, replay namespaces | [28](tasks/28-inspection-replay.md), [29](tasks/29-presentation-inspector.md), [45](tasks/45-effects-failures-laboratory.md) | Unsupported capabilities visible; no replay effects on live gates |

## Samples and validation

| Required behavior | Implementing tasks | Public acceptance |
| --- | --- | --- |
| Preserve black-box coverage, rework coupled tests | [01](tasks/01-behavior-baseline.md), [08](tasks/08-public-display-driver.md), [30](tasks/30-tictactoe-migration.md), [32](tasks/32-chess-cutover.md), [34](tasks/34-chess-ui-migration.md) | Replacement assertions pass before migration, native baselines preserved |
| Tic-tac-toe and chess full migration | [30](tasks/30-tictactoe-migration.md), [31](tasks/31-chess-reactant-fixture.md), [32](tasks/32-chess-cutover.md) | Existing visible timing, rules, audio, input, reset and persistence |
| Reactant/chess-ui migration; basic/ui retained | [33](tasks/33-reactant-sample-migration.md), [34](tasks/34-chess-ui-migration.md), [47](tasks/47-release-conformance.md) | Existing implemented scenarios and direct host examples still work |
| Hearts full rules/gameplay/AI/input/save | [35](tasks/35-hearts-assets-shell.md)–[43](tasks/43-hearts-save-resume.md) | Fixed-rule full match, hidden-information AI, all inputs and durable resume |
| Public driver, native conformance, inspector | [08](tasks/08-public-display-driver.md), [29](tasks/29-presentation-inspector.md), [44](tasks/44-identity-composition-laboratory.md), [45](tasks/45-effects-failures-laboratory.md) | Real worker barriers, rendered observations, native shaders/text/particles |
| Complete-card performance and structural bounds | [46](tasks/46-performance-workloads.md) | Fixed workloads and sustained measurements; no numerical gate |
| Release integration and physical handoff | [47](tasks/47-release-conformance.md), [48](tasks/48-retire-adapters-final-audit.md) | Required functional passes; physical evidence explicitly separate |

## Manual QA

Select a row, open its owner task and contract, and locate repeatable evidence.
For final review, sample across all three engine layers: public Rust behavior,
Unity-rendered behavior, and actual native/threaded-WebGL integration.
