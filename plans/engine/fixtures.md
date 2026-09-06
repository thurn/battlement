# Interactive test scenes, inspector, and performance workloads

The Reactant sample is the home for small, repeatable engine demonstrations.
Hearts proves that the engine supports a complete game; these scenes exercise
behaviors that an ordinary Hearts match does not show, such as a card moving
between UI and world space or reappearing during its old exit animation.

Read this when implementing test scenes, the inspector, or performance capture.
Related pages: [validation](validation.md), [requirement
checklist](coverage.md), [animation](motion.md), [world objects](world.md), and
[Hearts](hearts.md).

## Make each scene easy to reproduce

Each scene has a stable selector, reset, seed/deal input, and a visible caption
explaining its controls and expected behavior. Reuse the sample's existing
selection interface. A not-yet-implemented scene must be marked unavailable.

For example, the draw scene starts one action before the interesting behavior:

```text
selector: draw-reflow
reset: card face-down in deck, empty hand
Draw: move to reveal, flip, and move into hand
Reflow hand: change its target while the draw is running
expected: no jump; "ready" occurs only when the latest target is reached
```

Tests drive these scenes through the public display API and observe objects,
text, poses, effects, prompts, and lifecycle status. Do not expose private tree
maps or mutable rules state as shortcuts for assertions.

Controlled builders/services provide public worker-started, builder-entered, and
worker-stopped synchronization. They let tests pause at a known point without
guessed sleeps. For example, hold `Game::view`, exit, then release it and verify
that its late view is never displayed.

## Identity, composition, layout, and input scenes

These scenes require more than plain playing-card textures. Use independent
licensed or synthetic assets, rich text, and multiple renderers composed in
Rust. Task 44 completes this set and tests every configured layout-pair
transfer.

| Selector | Required interaction and result |
| --- | --- |
| `identity-transfer` | Move a card with local state among hand, pile, grid, fan, portal, and active roots; preserve state/refs and use new ancestry |
| `duplicate-identity` | Declare a duplicate live UUID in a different root or domain; reject the update and retain the old display |
| `incarnation-exit` | Remove and recreate a UUID during its old exit; fresh state/input belongs only to the new mount |
| `ui-world-transfer` | Transfer between UI and world using explicit orthographic and perspective mappings; preserve screen-space continuity |
| `mixed-input` | Exercise UI blocking/passthrough, nested modals, captured touch, and visible keyboard/controller focus |
| `composed-card` | Switch normal, compact table, hidden, and UI faces with rich text, badges, outlines, outcome preview, and a conditional action button |
| `contained-layout` | Arrange child cards inside a parent and resize its rest bounds; animated scale must not feed back into layout |
| `stores` | Change independent fields and write during render; equal selectors avoid reevaluation and later writes appear in a later complete update |
| `card-browser` | Open a grid from a deck pile, select/reorder cards, cancel or confirm, and retain rules-zone membership while browsing |
| `card-selection-scenes` | Choose a draft card, buy a displayed shop card, and select a quest deck; move cards among offers, selection, and destination layouts |
| `simulation-preview` | Evaluate a hypothetical choice with the shared rules and render its outcome beside the primary view without changing the live game |

The last three are small generic card interactions, not another full game. For
example, the shop has a fixed set of offered cards and a scripted purchase; the
quest-deck selector chooses among fixed decks and rearranges their cards. Their
purpose is to exercise display-only placement, typed prompts, previews, and
transitions. Use separate UUIDs for simultaneous inspection/preview copies.

## Animation, effects, and failure scenes

These scenes expose intermediate events and failure states through controls and
the inspector. Task 45 completes the set, building on the earlier feature tasks.

| Selector | Required interaction and result |
| --- | --- |
| `motion-equivalence` | Apply the same transition and controls to UI, world, default layout movement, and a sequence |
| `draw-reflow` | Reveal, flip, and arrive at a changing hand destination; keep hover responsive and wait for actual arrival |
| `material-effects` | Drive separate card instances with independent overrides; dissolve sprites, fade text, and reverse the effect |
| `attached-effects` | Compare live/captured anchors, trails, projectiles, light/audio properties, and retention after removal |
| `occurrence-replay` | Deliver a sound/burst twice, seek, resume, and replay; show exactly when each should emit |
| `prompt-cycle` | Select/deselect, submit invalid or stale answers, and use settings while waiting |
| `cancellation` | Cancel at publication waits, inside builders, prompt waits, answer/completion races, and explicit computation checks |
| `preparation` | Delay assets, supersede a prepared update, fail a required asset, and verify complete visible updates |
| `gate-replacement` | Replace required movement before/after a label, include multiple required animations, and reject old completion events |
| `save-failure` | Fail a save after an action is accepted; retain playable in-memory state and retry durability |

Include cancellation on entry to primitives before any new builder, after a
builder finishes, and before returning an answer or final state. Check cleanup
before stopped status and distinguish an ordinary panic from cancellation.

Effects scenes include game-owned Rust fallback selection, simultaneous effects,
optional RON configuration, and captured configuration during an active
playback. Validate shaders, text rasterization, particle rendering, and sound
timing in Unity as well as through the fake.

## The inspector explains current display behavior

Provide a reusable developer UI, hidden by default in ordinary player flows. It
reports the state needed to answer questions such as "Why is this card still
moving?" or "Why has the next prompt not appeared?"

Show:

- Current checkpoint and presented prompt.
- Object UUID and mounted lifetime, including retained exits.
- Layout destination and actual displayed transform.
- The animation controlling each property, active labels, and required work.
- Sound/burst history and retained effect resources.
- Pending preparation, committed update, and rendered-frame acknowledgement.
- Running, cancellation-requested, and worker-stopped status.
- Timing, allocation, and pending-checkpoint counters.

For example, a delayed checkpoint might show:

```text
checkpoint 12 is visible
waiting for: card A / draw / "ready"
card A destination: hand slot 4 (updated after resize)
last rendered frame: 80; completion has not occurred
```

Provide pause, slower playback, one-frame advance, supported seek, and explicit
replay. Show unavailable controls when native effects cannot seek. Inspection
must not accidentally answer prompts, satisfy live animation requirements, or
change rules state. Keep hidden Hearts hands out of normal player/inspector
views; fixture-only private data must be explicitly separate.

Geometry and motion reporting are opt-in. Turning the inspector off must remove
per-frame reporting that ordinary host playback does not need.

## Measure complete card views under sustained load

Use fixed populations of 300 cards and 500 cards across 30 layouts. Each card is
a complete composed view with text and material content, not one quad. Record
node, text, material, and renderer counts in the fixture definition.

Both workloads include:

- Sparse changes and reflow of every layout.
- Thirty simultaneous movement/effect tracks.
- Prompts, menus, hover, and inspection during animation.
- Concurrent AI simulation with fixed work counts, assets, and seeds.

Report these engineering targets without treating numerical misses as task
failure:

| Measurement | Target |
| --- | --- |
| Rust render/layout/reconciliation, p95 | At most 2 ms |
| Complete Reactant/Battlement main-thread CPU, p95 | At most 4 ms |
| Average application frame rate | At least 59 FPS |
| Application frame interval, p99 | At most 18.34 ms |
| Longest workload-attributable frame after warmup | At most 33.34 ms |
| Local hover/menu response | Within two rendered frames |

Keep correctness and simulation primitive requirements mandatory. Do not reduce
cards, simplify their content, or disable effects to improve the numbers. Report
misses with the responsible code and reproducible follow-up cases.

Record GPU time, allocations, pending-checkpoint count, state-to-visible
latency, preparation cost, and final visible-swap cost separately. Use release
builds, warmup, and at least ten minutes of sustained operation. Record
device/OS, resolution, graphics settings, refresh cap, browser version, build
settings, and AI worker count.

Capture desktop native and actual threaded desktop WebGL. The browser build
needs cross-origin isolation, `SharedArrayBuffer`, and working native threading.
Physical iPhone 17/Galaxy S25 sustained captures belong to the separate device
certification. Existing 200-element UI Motion workloads remain useful regression
and instrumentation checks; they do not replace the complete-card workloads.

## Manual QA

Select and reset each scene, perform its captioned interaction, and exercise its
failure or replacement control. Use the inspector to explain pending animation
and cleanup. During sustained captures, repeatedly open menus, inspect cards,
and answer prompts. Inspect slow frames and input latency as well as average
FPS.
