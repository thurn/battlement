# Dreamtides runtime parity audit: Rust and Battlement

This audit compares the runtime C# under `~/dreamtides/client/Assets/Dreamtides` with Battlement's [technical design](technical-design.md) and [implementation plan](implementation-plan.md). It excludes `Editor`, `EditorHelpers`, `Tests`, and `TestFakes`. Those folder exclusions leave 156 files; `Services/TestConfiguration.cs` and `Services/TestHelperService.cs` were also treated as test support, leaving 154 reviewed behavior files (about 33,000 lines). Ten independent high-reasoning reviews covered inventory, layout, interaction, animation, rendering, services, gameplay flows, the bridge protocol, Battlement implementation conformance, and alternative architectures.

The answer depends on what "implemented in Rust using Battlement" means:

| Target | Result parity? | Assessment |
|---|---:|---|
| Current repository HEAD | No | The Unity client stops after a subset of snapshot application; batches, commands, input, and custom handlers are not implemented. |
| Completed Battlement v1 using only built-in commands and actions | No | Runtime UI, continuous gestures, complex prefab internals, responsive viewport feedback, Cinemachine, and dynamic material effects are outside the core model. |
| Completed Battlement v1 plus compiled Dreamtides-specific C# handlers | Yes, credibly | Rust can own rules, authoritative state, semantic layout intent, presentation sequencing, and semantic actions. C# must still own substantial Unity-local presentation and interaction behavior. |
| Battlement extended as recommended below | Yes, with a thin generic Unity host | The missing capabilities can be made reusable without moving game rules or authoritative state back to C#. |

One platform caveat is separate from feature parity: Dreamtides' runtime plugin includes a WebGL path, while Battlement explicitly excludes WebGL. If WebGL remains a required deployment target, the answer for that target is No regardless of the extension strategy below. The remainder of the report evaluates supported native/HTTP hosts.

The important architectural distinction is therefore not `Update()` versus a Rust message. Most frame-by-frame Dreamtides layout and animation work can be reformulated as Rust-computed target states and finite Battlement batches. The hard gaps are places where Unity is the source of new information, where the desired result depends on Unity-only subsystems, or where Battlement cannot address the presentation surface at all.

Findings 2–8 are limitations of core Battlement commands/actions or of a predominantly Rust implementation. Because trusted custom handlers may call arbitrary Unity APIs, most become migration burden rather than absolute impossibility in the broader Battlement ecosystem. Findings 9–10 are deeper lifecycle/protocol omissions that the custom-handler escape hatch does not resolve by itself.

## 1. Blocker today — the implemented client cannot run dynamic Dreamtides behavior

This is an implementation-status finding, not a limitation of the approved design. It is nevertheless the first practical blocker to any port against the repository as it exists today.

`BattlementRunner` applies snapshots and explicitly ignores every response batch (`Packages/com.battlement.client/Runtime/Host/BattlementRunner.cs`, lines 561–574). During replacement snapshots it creates objects only for an initial snapshot (lines 650–662). The implementation plan still leaves snapshot completion and incremental work (Tasks 18–20), scheduling and tweens (Tasks 21–24), all command families (Tasks 25–31), and input/custom handlers (Tasks 32–34) unfinished (`implementation-plan.md`, lines 623–978).

Dreamtides depends on ordered and parallel command groups throughout `Services/ActionServiceImpl.cs` (especially lines 521–835). Consequently, current HEAD could display only part of an initial static world. It could not process a battle update, animate a card, accept an interaction, apply a reconnect replacement, or invoke a Dreamtides escape hatch.

Required action: complete Tasks 18–34 before treating any of the design-level workarounds below as executable. The current Rust workspace tests pass, but those tests do not imply that the unfinished Unity runtime exists.

## 2. Critical — runtime UI has no Battlement representation

Dreamtides uses runtime UI as a primary presentation surface, not a peripheral tool:

- `Services/DocumentService.cs` owns a `UIDocument`, reconciles four UI trees, and renders overlays, card information, effect previews, and world-anchored UI (lines 21–67 and 171–219).
- `Battlement/MasonRenderer.cs` renders scrollable, draggable, text-field, slider, and other UI node kinds with callbacks and style state (lines 19–100 and subsequent style handling).
- `Battlement/ScrollView.cs`, `Slider.cs`, `TextField.cs`, and `TypewriterText.cs` implement runtime interaction and presentation behavior.
- `Services/ActionServiceImpl.cs` refreshes these surfaces on ordinary battle updates (lines 606–668).

Battlement v1 explicitly excludes runtime UI, Canvas, and UI Toolkit (`technical-design.md`, lines 101–123). World-space text and image objects do not supply panels, clipping, scroll inertia, text input, sliders, focus, flexible layout, or the normal UI hit-testing result.

A result-equivalent port is still possible if Rust emits a semantic `render_view` custom command and a compiled C# presenter reconciles the existing UI locally. That is not a small adapter: it preserves a material portion of Dreamtides' current Unity client. A click-only or world-space replacement could preserve game legality but would not preserve the user-visible result.

Recommended Battlement extension: add a retained runtime-UI tree with stable node IDs, layout/style state, screen/world anchoring, clipping and scrolling, plus typed UI events. Until that exists, explicitly classify Dreamtides UI as custom-handler-owned state.

## 3. Critical — continuous and captured interaction cannot be expressed by core input

Dreamtides gameplay relies on local, low-latency pointer streams:

- `Services/InputService.cs` gathers all ray hits, resolves them by `GameContext` and `SortingKey`, and invokes continuous drag and hover callbacks (lines 63–92 and 131–183).
- `Layout/Displayable.cs` exposes mouse-down, drag, mouse-up, and hover semantics (lines 196–227).
- `Components/Card.cs` performs immediate lift, pointer-plane movement, thresholds, effect preview, order selection, and release behavior (lines 632–827).
- `Services/UserHandHoverService.cs` continuously selects among overlapping hand cards and animates the local result (lines 57–139 and 254–367).
- `Layout/ScrollableCardGridLayout.cs` connects moving UI rectangles to world cards and relies on `ScrollRect` inertia (lines 139–173 and 217–243).

Battlement's action set reports enter, exit, down, up, and click. Dragging, scrolling, gestures, and text entry are explicit v1 exclusions (`technical-design.md`, lines 106–118 and 957–1008). Its closest-collider rule also cannot reproduce Dreamtides' all-hit, context-aware arbitration.

Sending every pointer move through Rust would add avoidable latency and still would not solve scroll inertia, capture, or UI focus. Precomputing animation is not applicable because the trajectory is new information originating at the client.

The credible result-parity design is a supported local interaction handler: C# owns hover, drag preview, capture, scrolling, and transient lift; it emits one typed semantic action on commit, while Rust owns legality and the authoritative final state. Snapshot replacement must cancel or reconstruct any local gesture.

Recommended Battlement extension: pointer move/delta/wheel actions, pointer capture, configurable hit arbitration/layers, text/focus input, and a documented local-gesture lifecycle. A Dreamtides-specific handler is viable before those general primitives exist.

## 4. High — complex prefab and authored-scene presentation is opaque below object roots

Dreamtides cards are compound presenters. `Components/Card.cs` references many child transforms, renderers, TMP fields, colliders, icons, face roots, status objects, and attachment points (lines 23–126); it updates child text, sprites, materials, active state, outline state, studio art, and effects (lines 310–614). Card flips rotate two internal face transforms and swap their visibility mid-sequence (lines 426–442). `Services/CardService.cs` reconciles multiple prefab variants and routes them into more than twenty layouts (lines 73–155 and 256–306).

The same issue appears outside cards: Dreamtides changes `SortingGroup.sortingOrder` and sorting layers through `Layout/Displayable.cs` (lines 130–150 and 232–251), targets serialized child `Animator` components, attaches trails to named children, and controls authored scene services through `Services/Registry.cs`.

Battlement deliberately targets created object roots. It cannot address authored scene objects or arbitrary child components, and whole-material assignment applies only to a supported root renderer (`technical-design.md`, lines 649–677 and 845–847). The command union also has no sorting-layer/order primitive. Battlement's prepared asset set lacks a `Sprite` kind, while Dreamtides loads Addressable sprites and assigns them to card renderers (`Services/AssetService.cs`, `Components/Card.cs` lines 515–534).

Some result parity can be obtained by content refactoring: split one card into a hierarchy of separately addressable Battlement objects, replace sprites with texture-backed image quads, encode some ordering in depth, and move Animators to roots. That becomes brittle for semantic child anchors, grouped 2D sorting, reusable compound prefabs, and existing art pipelines.

Recommended Battlement extension: stable named slots or generated component bindings on prefabs, root `SortingGroup` state/commands, and prepared Sprite assets. A more constrained alternative is a reusable composite-presenter custom handler whose entire view is reapplied idempotently after snapshots.

## 5. High — Rust receives no live viewport or safe-area feedback

Dreamtides adapts after startup to information owned by the Unity client:

- `Components/CanvasHelper.cs` watches orientation, resolution, and safe-area changes and updates Canvas layout (lines 43–130).
- `Layout/SceneElementScreenPosition.cs` derives world placement from safe-area-relative screen positions (lines 132–160 and 215–246).
- `Components/DreamscapeMapCamera.cs` recomputes framing from aspect ratio and safe area (lines 102–181 and 253–290).
- `Components/IGameViewport.cs` exposes live viewport bounds used by layouts and cameras.

Battlement's `Connect` message provides initial width and height, but not safe-area insets, and there is no display-change action (`crates/battlement/src/messages.rs`, lines 10–26; `technical-design.md`, lines 164–185). The runner builds that environment once when connecting (`BattlementRunner.cs`, lines 505–515).

Rust can compute many responsive layout targets once it has the measurements; the primary missing capability is feedback, not a need to run layout in Unity's `Update()`. Some Dreamtides layouts additionally depend on resolved UI rectangles, authored anchors, live camera transforms, and world/screen projection. Those cases need either richer measured-layout/camera feedback or a local layout handler. Reconnecting on orientation change is possible but disruptive and still omits safe area.

Recommended Battlement extension: a revisioned `DisplayChanged` client message containing pixel size, orientation, safe-area rectangle/insets, DPI or scale where relevant, and active camera viewport. Rust can respond with an ordinary replacement or animation batch.

## 6. High — Cinemachine and live RenderTexture composition require Unity-local systems

`Services/StudioService.cs` creates 1024×1024 RenderTextures, retargets cameras, assigns those textures to renderers, creates animated character instances, and transitions their Animator states (lines 47–180). Cards and player-status displays use these live studios as art.

`Components/DreamscapeMapCamera.cs` uses Cinemachine priorities, follow/look-at targets, custom blend curves, and viewport-dependent framing (lines 97–181 and 300–535). Battlement explicitly places Cinemachine outside the core (`technical-design.md`, lines 101–116), and it has no RenderTexture allocation or camera-target-texture command.

Rust can select the subject, state, camera mode, and transition schedule. It cannot itself render a Unity hierarchy into a Unity texture. Static baked portraits could approximate the content, and ordinary Battlement cameras cover simpler scenes, but neither is general result parity.

Recommended approach: retain small, generic `studio.render` and `camera.rig` custom handlers with typed, idempotent state. If these patterns recur across games, add named camera-rig bindings and render-target objects to Battlement; do not move the decision logic back into C#.

## 7. High risk — unbudgeted main-thread application can break frame-result parity

Battlement submits, parses, and applies responses synchronously on Unity's main thread (`technical-design.md`, lines 1118–1130). It permits responses up to 16 MiB and snapshots containing up to 100,000 objects, yet explicitly provides no cooperative per-frame budget: ordinary parsing, validation, and Unity object construction are not split across frames (lines 1299–1330).

Dreamtides commonly reconciles cards, UI, layouts, effects, and cameras from one semantic battle update. Consolidating that work into one Rust response reduces protocol chatter, but it does not reduce the main-thread cost of decoding and reconciling the resulting state. A custom `render_view` handler can worsen the spike if it performs the whole retained-tree reconciliation in one call. Visible hitches are a result-parity failure even when the eventual state is correct.

This is a scale risk rather than proof that current Dreamtides content exceeds a frame budget. It should be measured with representative battle, reconnect, quest-map, and large-browser states before migration. Recommended Battlement extension: off-thread decoding and validation where Unity APIs are not involved, incremental preparation, a configurable main-thread work budget, and an atomic or hidden commit boundary so partially applied presentation does not become visible.

## 8. Medium — dynamic shader, procedural geometry, and renderer effects are custom-only

`Components/DissolveEffect.cs` clones materials, preserves textures, changes Advanced Dissolve keywords and parameters, and updates the clip value every frame (lines 33–110). Cards apply this recursively across child renderers while separately fading text. `Components/Arrow.cs` rebuilds curved segments, scrolls them, edits per-instance alpha, creates a head, and follows two moving endpoints every frame (lines 84–151). SpriteRenderer and CanvasGroup fades are also used by generic tween helpers.

Battlement intentionally permits whole material replacement but excludes arbitrary shader properties and keywords (`technical-design.md`, lines 674–677). It has no line, trail, renderer-alpha, constraint, or procedural-geometry primitive. Pre-authored material swaps cannot reproduce a continuous dissolve while preserving each renderer's texture.

These effects are narrower than UI and card interaction, so typed C# handlers are proportionate: Rust sends effect identity, endpoints, timing, colors, and lifecycle; the handler performs Unity-local rendering. Simple projectiles are not a gap: their visible result can be decomposed into object creation, a transform tween, particles, audio, waits, and destruction once planned commands are implemented.

Recommended Battlement extensions, in priority order: root renderer color/alpha and `SortingGroup`, named material-property bindings rather than unrestricted reflection, named attachment points, and an optional line/trail primitive. Advanced Dissolve itself should remain content-specific.

## 9. Medium — custom-handler state has no snapshot contract

Custom handlers are the designed escape hatch for precisely the UI, gesture, Cinemachine, and shader cases above (`technical-design.md`, lines 1041–1082). However, their state is explicitly excluded from snapshots. Battlement requires the application to reconstruct it, without defining when a handler is reset, how replacement cancels its work, or how it contributes to incremental-application budgeting and settled state.

That omission is significant for Dreamtides. A replacement can arrive while a card is locally dragged, a Studio coroutine is between exit and enter, a dissolve owns cloned materials, or a custom UI tree contains transient selection. Reapplying only core object state can leave a visibly hybrid client.

Result parity is achievable with an application convention: every custom presenter consumes an idempotent full state command after each snapshot; the snapshot boundary first cancels gestures, coroutines, temporary materials, and local effects. This convention must be guaranteed by batch ordering and should not be left to incidental handler behavior.

Recommended Battlement extension: a custom subsystem interface with `Prepare`, `ApplyFullState`, `CancelTransientWork`, and `ResetForSnapshot` lifecycle hooks; snapshot-associated custom state or an immediately following atomic custom-state group; and a way for custom work to register finite activity.

## 10. Medium — interruption and completion semantics are insufficient for exact orchestration

Battlement defines property conflicts and operation cancellation, but canceling an operation stops it without a completion callback (`technical-design.md`, lines 1266–1278). The design does not state whether later groups in the canceled operation's batch advance, are suppressed, or can become stranded. Dreamtides uses interruptible composite sequences: for example, `StudioService` cancels an old coroutine and suppresses all of its stale exit/enter/main continuation when a newer request arrives (`Services/StudioService.cs`, lines 132–180). Similar stale continuations can cause a move-then-destroy batch to destroy an object reused by newer state.

A plain "issue one phase from a later poll" workaround is insufficient because Rust receives no successful batch/operation completion signal; it would have to guess from duration. Valid workarounds are a cancellation-aware composite custom operation, an explicit completion action emitted by the client, or authoritative replacement. All give up some simplicity of the requested single-message precomputed sequence.

Battlement also sends Rust only actions and failures. There is no successful `SnapshotApplied`, `BatchCompleted`, finite `OperationCompleted`, or aggregate `ClientSettled` message (`crates/battlement/src/messages.rs`, lines 288–302). This does not block ordinary visible gameplay, but it blocks exact migration of runtime systems such as `Abu/DreamtidesSettledProvider.cs`, which waits for engine completion, command processing, all tweens, custom busy tokens, and quiet frames before observing the client (lines 61–132). A screenshot necessarily remains client-side.

Recommended Battlement extension: define explicit operation outcomes and whether cancellation suppresses composite continuation; add a batch/composite cancellation or generation guard; identify presentation revisions; and optionally report applied/settled lifecycle events. Custom activity must participate if `ClientSettled` is exposed.

## 11. Low — a few continuous services need local ownership or feedback

Dreamtides music shuffles an indefinite playlist and crossfades based on live clip time (`Services/MusicService.cs`, lines 51–117). Battlement audio supports play/stop, fades, pitch, volume, loop, and pooling as designed, so known finite sequences are representable. It does not report clip completion, making an unbounded Rust-driven random playlist awkward. Keep a small local music controller, pre-schedule bounded tracks from metadata, or add an audio-completed action.

Dreamtides also pools arbitrary prefab instances. Battlement limits pooling to opted-in temporary particle and audio objects. This is primarily an allocation/performance difference: the same visible result is available through create/destroy or a custom pool. It is not a fundamental parity blocker.

Automatic reconnect and telemetry are application policies rather than missing expressive power. The runner exposes reconnect, and a custom logger can route telemetry. Small host policies may remain in C# without retaining game authority.

## 12. No gap — layout, ordinary animation, and game behavior can move to Rust

The following broad areas do not require Dreamtides' current C# execution model:

- **Object layouts.** `Layout/ObjectLayout.cs` and its subclasses calculate deterministic transforms from view state, membership, sorting, and viewport inputs. Where all inputs are authoritative or reported, Rust can calculate those target transforms and send one snapshot or grouped transform batch. Layouts tied to resolved UI rectangles, authored anchors, or live projection remain in the measured-feedback/local-handler category described above. Per-frame interpolation belongs in the Battlement host; most per-frame layout policy does not.
- **Ordinary choreography.** Local/world position, quaternion rotation, scale, parallel property tweens, ordered groups, staggered delays, waits, and the easing curves used by Dreamtides are covered by the designed batch model. The inventory found that 56 of 72 detected tween calls are transform movement, scale, or rotation. Card-face flips can become ordered groups with a mid-sequence active-state swap if faces are addressable.
- **Animator decisions.** Rust can choose Animator state and supply explicit waits. Clip durations should be generated into content metadata rather than queried dynamically. A custom composite is needed only for interruptible or child-Animator cases described above.
- **Rules and semantic view state.** Battle, quest, draft, shop, history, effect selection, capability decisions, action legality, environment selection, layout selection, randomization, and persistence naturally belong in Rust. History and effect logs can be recorded from authoritative mutations or emitted commands.
- **Most effects.** Straight projectiles, hits, timed particle effects, trails modeled as separate objects, sounds, messages, and simple overlays are compositions of planned core primitives. Rust should own their durations and lifetimes.
- **Transport.** Battlement's session, submit/poll, prepared-asset, reconnect, and failure model can replace the current `ActionServiceImpl` transport and command queue once the unfinished tasks land.

This is why the apparent incompatibility of `ObjectLayout.Update()` is not itself a Battlement gap. The result depends on the target transforms and timing, not on which language executed the old update loop.

## 13. Recommended result-parity architecture

Use one authoritative presentation revision in Rust. Each accepted game action produces:

1. a semantic view model containing cards, panels, legal interactions, camera/studio intent, responsive layout inputs, effects, and stable presentation IDs;
2. core Battlement snapshot/batch data for independently addressable world objects, normal transforms, ordinary tweens, particles, audio, cameras, lights, text, and images; and
3. a small number of typed full-state custom commands for UI, compound card presentation, local gestures, studios/camera rigs, and advanced effects.

The Unity side should be reactive and non-authoritative. Its custom handlers may derive frame-local visuals, capture input, and call Unity APIs, but they should not decide game legality, select long-term state, or own sequences that Rust could express. Every handler should be idempotent by presentation revision and capable of a complete reset at snapshot boundaries.

The highest-value Battlement additions are, in order:

1. complete the planned replacement, batch, command, input, and custom-handler implementation;
2. add runtime UI or formally support a retained custom UI subsystem;
3. add continuous input, capture, wheel/text events, and local-gesture lifecycle;
4. report live display and safe-area changes;
5. support composite prefab bindings, sorting state, and prepared sprites;
6. define custom-state snapshot/reset hooks and batch interruption outcomes; and
7. add optional presentation-applied/settled feedback for automation and exact client observation.

With those changes, almost all behavioral decisions can be Rust code and the remaining C# can be a generic Unity rendering/input adapter. With Battlement v1 exactly as designed, result parity is still plausible only by retaining a substantial Dreamtides-specific C# extension layer. With current HEAD, it is not yet runnable.
