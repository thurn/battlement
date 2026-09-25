# Make Chess Settings Functional

## Authority and use

This is a standalone implementation specification and task graph, grounded in
Battlement revision `381899fceeb6ad8c1ba7b113742ed350f4d1491f`. The user authorized
writing this document and filing tasks, **not executing the implementation**.
All implementation beads and the approval decision remain natively deferred,
without an expiry, until explicit authorization. Filing or delivering this
document does not release that hold. The bead index below records the native IDs.

The preceding design received one cold document review. This specification
incorporates its findings: adapted piece slides must bypass generic spatial
snapping; gameplay mappings need explicit shortcut precedence; preference writes
need ordering and durable completion distinct from applied state. This expansion
has not received a second cold review. Revalidate source pointers in each task's
own checkout before implementation; code remains the source of truth.

Read the shared contracts and the assigned work package before changing code.
Each package includes implementation, proportionate tests, and delivery; later
qualification packages do not excuse missing focused evidence in feature work.
Follow the repository's worktree, CI, review, and Tollgate instructions. Never
select another task's worktree. Keep executable work out of this planning change.

## Engine improvement is the primary outcome

Chess is the integration specimen for improving Battlement and Reactant. Working
settings alone are insufficient: **if Reactant is unchanged, this project fails**.
Deliver substantive, reusable Reactant runtime/API/utility improvements, use them
in chess, and demonstrate how they reduce application complexity. Documentation,
renames, or test-only changes do not satisfy this criterion. Prefer extensions to
existing idioms over sample-owned substitutes for missing engine capabilities.

The persistence, capability context, audio authoring, and exclusive input capture
packages are initial opportunities, not a complete improvement inventory. Let
implementation and hands-on use expose further gaps. Do not invent abstractions
just to meet a change quota; show a real consumer, a smaller authoring example,
and evidence that the library takes responsibility for the relevant behavior.

## Product contract

Deliver on **macOS, Windows, web, and iOS**, beginning with a macOS vertical slice.
Linux, Android, cloud preference synchronization, a replacement diagnostics vendor,
new chess rules, and public deployment are excluded. Include the minimum iOS
physical-device build support required to verify the selected behavior.

Preserve existing setting names, styling, and composition. New UI is allowed in
the existing visual style, including controller capture, save errors/retry,
reporting limitations, and display confirmation. Existing dialog wording may be
corrected. Capability-based hiding/options, localization, and wrapping/scrolling
for enlarged text are allowed; this is not a menu redesign.

| Setting | Observable contract and initial value |
| --- | --- |
| Language | English initially; English and French choices; immediate application-wide switching without resetting game, focus, route, selection, or dialog. |
| Text Size | 100% initially; 100/150/200% actual scaling of readable interface text in menus and gameplay; artwork unchanged. |
| Reduce Motion | Saved toggle off initially; effective reduction is saved toggle OR system request; short direct piece slides remain. |
| Increase Move Duration | On initially; add two seconds before the computer starts its normal response; retain the label and normal search/animation speeds. |
| Upload Crash Reports | On initially; persisted, best-effort runtime control of the existing Unity vendor integration, with honest limitations. |
| Erase Saved Data | Confirm and erase local chess progress only; preserve all preferences/bindings/reporting choice; return to initial menu with no resume. |
| Resolution | Desktop-only host-supported choices; start from current valid host resolution rather than the menu's fixed 1080p string. |
| Max Framerate | 144 FPS desktop/web preference initially; 60 FPS or lower supported rate on iOS; actual cap where exposed. |
| Display Mode | Borderless initially where supported; Windows also Fullscreen and Windowed; macOS also Windowed. |
| Screenshake | On initially; subtle board-only shake on capture/checkmate; suppressed by effective Reduce Motion. |
| VSync | On initially on desktop; desktop synchronization controls actual pacing; not exposed on web/iOS. |
| Master Volume | 80%; every music/effects source, active and future. |
| Music Volume | 65%; menu music, gameplay music, and both sides of crossfades. |
| Effects Volume | 75%; movement, capture, opening, invalid-action, interface, and sequence sounds. |
| Mute in Background | Off initially; when on, all audio muted whenever unfocused OR paused. |
| Keyboard Input Remapping | Seven listed gameplay actions; authoritative defaults arrows, Space, Escape, R; saved single physical-key bindings. |
| Controller Input Remapping | Same actions; defaults D-pad, South/A, Start/Menu, North/Y; saved buttons/D-pad bindings; fixed left-stick board navigation. |

## Source grounding and responsibility

| Concern | Starting implementation and gap |
| --- | --- |
| Menu preferences | [settings screen](../samples/chess/rules/src/menu/settings_screen.rs) owns transient language, graphics, effects, reporting, and duration values; erase confirmation only closes. |
| Root and save | [application composition](../samples/chess/rules/src/reactant_view.rs), [save schema](../samples/chess/rules/src/persistence.rs), and [Reactant persistence](../crates/reactant/src/persistence.rs); game FEN storage exists, synchronous writes do not provide browser durability results. |
| Localization | [AppHandle localizer replacement](../crates/reactant-core/src/app_context.rs) already exists; chess needs catalogs and conversion of untranslated dynamic strings. |
| Motion and turns | [movement sequences](../samples/chess/rules/src/motion.rs), [board effects](../samples/chess/rules/src/chess_board.rs), and TurnCoordinator in application composition. |
| Input | [chess routing](../samples/chess/rules/src/reactant_input.rs), [binding table](../samples/chess/rules/src/menu/input_settings.rs), and [global input facade](../crates/reactant/src/input.rs); displayed mappings differ from gameplay and facade drops controller metadata. |
| Audio | [menu provider](../samples/chess/rules/src/menu/background_music.rs), [game effects](../samples/chess/rules/src/reactant_effects.rs), and [Unity sources](../Packages/com.battlement.client/Runtime/Host/BattlementAudioSources.cs); separate gain owners, no shared category control. |
| Host observations | [connect context](../crates/reactant-core/src/app_context.rs), [controller polling](../Packages/com.battlement.client/Runtime/Host/BattlementControllerInput.cs), [system motion bridge](../Packages/com.battlement.client/Runtime/UI/BattlementReducedMotion.cs); bridge currently implements macOS/web only. |
| Diagnostics | [Rust commands](../crates/battlement-cloud/src/diagnostics.rs), [Unity module](../Packages/com.battlement.client/Runtime/Cloud/Diagnostics/BattlementDiagnosticsModule.cs), [project configuration](../samples/chess/ProjectSettings/UnityConnectSettings.asset); metadata API exists, project reporting is disabled and no Cloud project ID is configured. |

Reuse the generic native-write/browser-flush direction in the existing
[durable-save work package](engine/tasks/43a-durable-save-service.md); do not add a
parallel native file-I/O command service. That document is context, not evidence
that the work shipped. Likewise, reuse the relevant iOS portion of
[mobile build work](engine/tasks/05-mobile-build-paths.md), without importing its
Android scope. No matching existing native beads were found during filing.

### Settings ownership and durability

Create one typed chess settings model, root context, and setter boundary above
menu/game routing. Store it in `chess-settings.json`, separate from
`chess-game.json`. Language, display modes, and actions use stable IDs; localized
labels are presentation only. Avoid a generic settings registry or versioned
migration framework. Validate each persisted field; invalid fields use their
documented defaults. Revalidate loaded binding maps for duplicates/reserved input.

Track desired/applied preferences, latest successfully durable snapshot, one
in-flight write ID, and the newest pending snapshot. Apply valid preferences
immediately; serialize writes and replace the queued snapshot with the newest
intent. Accept a completion only for its matching operation. Older completions
cannot replace current values or clear a newer failure. A failure leaves current
preferences applied, preserves the last durable snapshot, and presents an unsaved
message with Retry. Retry writes current intent, not the failed historical value.
Show pending versus failed state accessibly; do not silently claim a save succeeded.

Native storage uses temporary write, flush, atomic replacement, and platform-
appropriate durability; errors leave a complete prior file. Reuse Rust file I/O.
Extend the existing backend/hook to expose operation completion while allowing a
native operation to complete immediately. On web, bridge the missing correlated
flush/result at the host boundary. Existing `autoSyncPersistentDataPath` stays
compatible; serialize explicit work with automatic synchronization, rather than
issuing competing flushes. A write or delete is durable only after its flush
acknowledgment; denied storage/quota failures are failures, not success in memory.

Gate initial audio/effects and configurable host settings on preference hydration.
Corrupt or absent values use defaults; unavailable storage permits session-only
settings with a visible persistence limitation. Never overwrite stored preferences
with defaults while an asynchronous load is still pending. Store only confirmed
display mode/resolution, not a preview. Existing hooks and context remain the
authoring mechanism; the sample does not own a second transport.

### Capability and application contracts

Extend connect-time observations and runtime change events with a reusable host
settings snapshot: supported platform class, display mode/resolution choices,
refresh information, applied display state, frame-pacing support, keyboard and
controller availability, and diagnostics support/configuration. Expose an updated
Reactant context. Distinguish supported, unavailable, and an operation that failed.

Use typed display configuration and request IDs for apply, confirm, cancel, and
result/state changes. New protocol data must traverse schema generation, Rust
encoding/decoding, Unity retained/direct paths, validation, and fake execution.
Do not infer support from whether a menu widget can be rendered. Observe monitor,
window, device, focus, and pause changes. Retain preferences for disconnected
devices or temporarily unavailable settings. No platform-specific API belongs in
chess rules or translated labels.

### Language and readable text

Use Trox extraction, English source and French bundles, and existing localizer
replacement. Translate every player-facing route and state, including game result,
promotion, pause, bindings/conflicts, errors, confirmations, help, tooltips, and
accessibility names/announcements. Format dynamic messages through localization
templates, not concatenated English. Exclude developer logs from translation.
Retain English fallback for defensive runtime behavior, but catalog completeness
is a release gate. Verify fonts include French accents and punctuation.

Apply the selected 1.0/1.5/2.0 factor to functional text sizes, replacing the current
role-dependent partial-growth formulas. Do not double-scale inherited text.
Preserve unscaled artwork, board geometry, and 100% appearance. Increase containers,
wrap, or scroll as required; all controls must remain reachable with pointer,
keyboard, controller, and accessibility actions. Switching language or scale must
retain interaction state and keep focused content visible.

### Motion, feedback, and computer response

Read system reduced-motion requests on macOS/web and add Windows/iOS observations
and change detection. Effective reduction is logical OR with the saved toggle.
The toggle stores the player's additional preference; system requests remain
effective when it is off. Provide explanatory UI if that distinction is needed.
Use the same effective policy for menu transitions, infinite decoration, particles,
manual effects, openings, and board sequences.

With reduction active, remove decorative motion and use direct 120 ms piece
slides, straight knight travel, and simultaneous king/rook castling. Explicitly
exempt these already-adapted essential sequences from the generic host spatial
snap policy. Do not bypass reduction for normal cinematic movement. Calculate
sound timing from the selected sequence; on a mid-sequence policy change settle
to the accepted position and never replay effects or block turn completion.

Screenshake offsets a board-world presentation parent, not the HUD or logical
squares. Use deterministic decaying motion, approximately 0.03 square widths for
180 ms on captures and 0.06 for 280 ms on checkmate. A checkmating capture produces
one checkmate shake. A newer event replaces the current shake from its baseline;
offsets do not accumulate. Disabling shake/reducing motion immediately restores
the baseline. Hit testing and logical square selection remain coherent.

The computer delay begins when the accepted player move's presentation is ready,
before the computer action is dispatched. Use a session/position-owned timer on
the injectable application clock; never sleep the rules/UI thread. Preserve
remaining delay across pause/menu/background, dispatch once on expiry, and cancel
on erasure, restart, position/session replacement, or unmount. Turning the toggle
off releases the pending wait; turning it on does not restart dispatched work.
Keep the existing AI budget and move animation durations. Scripted tests control
both opponent permission and time; no elapsed-time guessing in Ditto scenarios.

### Progress erasure

The confirmation explicitly says game progress is removed and preferences remain.
On confirmation, quiesce autosave and invalidate pending session generations, then
serialize deletion after older writes and await durability. Return to the initial
menu with no resumable game only after successful deletion. Prevent old worker,
timer, or persistence completions from repopulating state. On failure, preserve
the recoverable game and present Retry/Cancel; do not announce successful erasure.
An already absent save is success. Do not delete preferences, diagnostic files,
uploaded reports, other games' data, or arbitrary persistent-directory contents.

### Audio

Add reusable `Music` and `Effects` routing to playback and motion-sequence sounds,
plus host-owned master/category gains and a background mute factor. Output gain
is source gain multiplied by fade envelope, category gain, master gain, and mute.
Changing the latter three must not cancel fades, restart playback, or reschedule
sounds. Update active, crossfading, pooled/reused, and future sources coherently.

Route both menu and gameplay tracks to Music and every one-shot/sequence sound to
Effects. Replace competing sample volume owners with the shared settings context.
Existing +/- and pause-menu volume shortcuts change Master; the music indicator's
explicit music mute remains a separate transient music-only action. With Mute in
Background on, mute when `!focused || paused`, including hidden browser pages.
Returning restores gains without replay. When off, the application adds no mute;
mobile/browser suspension can still prevent playback. Browser resume may require
a user gesture; use new feedback if needed instead of promising autoplay.

### Input binding and capture

Use seven action IDs: left, right, up, down, move_piece, pause, restart. Persist
one keyboard map and one normalized controller map, not per-hardware identities.
Make displayed defaults authoritative. Every ordinary invocation of a listed
action uses the map; remove obsolete aliases. Nonessential unbound debug/volume
shortcuts may remain, but a gameplay mapping wins. Modified developer commands
must not also invoke the unmodified gameplay binding.

Keyboard capture accepts one physical non-modifier key, with no chords. Reserve
Escape from assignment except to Pause; when a piece is selected, Escape cancels
selection before Pause. When Pause is remapped away, Escape remains cancellation,
not an alternate pause shortcut. Reserve controller East for cancellation. Accept
controller buttons and D-pad directions; keep left-stick directional navigation
fixed and exclude stick/trigger rebinding. Preserve standard menu navigation and
cancel controls independently of the game map.

Reject duplicate assignments and conflicting per-action Reset operations without
changing either map. Announce conflicts and successful assignments in the current
language. Reuse the keyboard dialog's visual language for controller capture.
Capture must intercept input before normal UI navigation/global gameplay handlers,
retain device/source/repeat metadata, swallow the opener until release, suppress
repeat activation, and clear held input on focus loss. Cancel on device disconnect.
Keep a reachable modal Cancel control while Escape itself is being captured.

Desktop keyboard capability remains available. On iOS/touch-only web, expose a
keyboard column only when physical keyboard input is available, and a controller
column when a supported gamepad is available. Hide the Input tab if both are
absent; reveal on observed device connection/use without dropping maps. Browser
device detection must be based on available evidence, not user-agent guesses.

### Graphics and rollback

| Target | Resolution/modes | Frame pacing |
| --- | --- | --- |
| Windows | Valid host choices; Borderless = FullScreenWindow, Fullscreen = ExclusiveFullScreen, Windowed = Windowed. | VSync on uses refresh synchronization; off exposes 60/120/144/240 FPS caps. |
| macOS | Valid host choices; Borderless/Windowed only. | Same desktop VSync/cap policy. |
| Web | Hide Resolution and Display Mode; retain viewport-sized canvas; browser fullscreen is outside scope. | Hide VSync; set host pacing to honor 30/60/120/144/240 FPS caps, still limited by browser refresh. |
| iOS | Hide Resolution and Display Mode. | Hide VSync; expose supported refresh divisions among 30/60/120 FPS; account for ProMotion configuration. |

Hide Max Framerate while desktop VSync makes its software cap ineffective; retain
the selected cap and reapply when VSync is disabled. Unsupported stored rates map
to the closest supported rate not above the request, or the minimum available
rate. Caps are ceilings, not achieved-FPS promises. Display resolution choices
use host dimensions/IDs, deduplicating refresh variants where the UI shows only
dimensions; keep valid current window size visible. Refresh options after monitor
changes and reconcile manual window resizing with actual host state.

Apply a resolution/mode pair as one preview transaction. Persist a recovery record
containing the prior confirmed configuration before changing the display. Unity
owns the 15-second real-time watchdog independently of paused Rust/game clocks.
Show Keep changes?/Revert in a new existing-style modal; keep only after host
readback and user confirmation. Timeout, cancellation, focus loss, apply failure,
or lost owner reverts. On startup with an unfinished transaction, recover the last
confirmed configuration, or a safe current-monitor window if it is unavailable.
Persist the confirmed configuration durably before clearing recovery state. If
that persistence fails, revert rather than leave an unrecorded confirmed preview.
Handle stale confirm/cancel IDs harmlessly. Do not apply preview settings from
older acknowledgments after a newer transaction.

### Best-effort diagnostics

Investigate `UnityEngine.Analytics.PerformanceReporting.enabled` in the installed
Unity version and relevant supported capture controls. The symbol exists in the
installed 6000.5.8f1 assembly, but that does not prove it controls the newer service.
Do not equate `enableCaptureExceptions` with preventing native crashes or uploads.
Apply saved intent as early as possible; keep requested and observed support state
distinct. Keep the vendor/help link and default on, and hide unsupported-platform
controls. Explain partial control or missing configuration in new feedback.

The checked-in project has reporting disabled and no Cloud project ID. Reuse
available project configuration; do not invent credentials, provision another
vendor, or represent missing account access as successful verification. With a
configured test project, verify managed exception, native crash, disabled capture,
disabled upload, queued reports after relaunch, and earliest startup behavior.
Record exact Unity/build identity and observed limitations. This is the sole
best-effort exception: lack of external service configuration does not block all
other settings. It does not establish GDPR compliance or a legal requirement.

Technical references: Unity documents [runtime performance reporting](https://docs.unity3d.com/kr/2022.3/ScriptReference/Analytics.PerformanceReporting-enabled.html),
[newer diagnostics consent scope](https://docs.unity.com/en-us/cloud/developer-data/user-consent),
[frame pacing](https://docs.unity3d.com/6000.0/Documentation/ScriptReference/Application-targetFrameRate.html),
and [fullscreen support](https://docs.unity3d.com/ja/6000.0/ScriptReference/FullScreenMode.html).
Recheck against the implementation's exact Unity version.

## Work packages

The native graph includes the listed dependencies, the approval gate on every
executable package, and the paired introspection prerequisites described below.
Task IDs are stable planning identifiers; bead IDs are recorded in the final index. Independent packages may be scheduled independently
after approval, but shared-file edits require owner coordination. No estimates
imply a commitment to delivery dates. Each package includes necessary protocol,
fake, and host changes for its boundary; no protocol-only feature is considered done.

### S01 — Expose ordered durable persistence operations

**Depends on:** approval. **Owner:** Reactant persistence/native storage.
Extend the existing backend/hook with correlated read/write/delete outcomes, atomic native replacement, and last-durable versus pending state. Keep native I/O in Rust; provide only the extension point needed for browser flush. Reuse the existing durable-save plan when work has landed, rather than implementing it twice.
**Acceptance:** native restart reads acknowledged bytes; failed replacement preserves the prior complete file; missing delete succeeds; out-of-order injected completions cannot overwrite current intent; tests cover write/delete ordering and corrupt input. Supply a tiny generic storage fixture and the async completion contract for S02/S04/S15.

**Manual QA:** Run the generic storage fixture: save, exit, reopen, and inspect the recovered value; inject one failed replacement and verify the previous value remains readable.
**After delivery:** complete I01 before downstream consumers proceed. Apply the per-change loop below if this package is split into multiple deliveries.

### S02 — Acknowledge durable browser storage

**Depends on:** S01. **Owner:** WebGL persistence bridge.
Implement the correlated flush/result path used by S01, compatible with automatic IDBFS synchronization. Serialize writes and flushes; propagate browser load, quota, denial, and deletion errors. Exercise the actual threaded Unity WebGL player, not an unrelated browser storage demo.
**Acceptance:** acknowledged write survives reload/new browser session; acknowledged deletion stays deleted; failed flush leaves last confirmed state recoverable and reports failure; rapid writes cannot resurrect an older snapshot. Retain browser-specific evidence and bridge/fake tests.

**Manual QA:** In the actual WebGL player, save then reload; deny a flush and inspect the visible error plus the last confirmed value after reload.
**After delivery:** complete I02 before downstream consumers proceed. Apply the per-change loop below if this package is split into multiple deliveries.

### S03 — Publish host settings capabilities and device changes

**Depends on:** approval. **Owner:** shared protocol/Unity host/Reactant context.
Publish initial and changed display/platform/input/reporting capabilities and applied state. Add typed observations and result identities through every transport path and relevant fake. Query supported display modes/resolutions and distinguish native desktop, browser, and iOS; no Android/Linux bringup.
**Acceptance:** native snapshot agrees with host; simulated monitor/device changes refresh context without resetting preferences; unavailable versus failed remains distinguishable; connection serialization round-trips; touch-only and attached-device states are exercisable deterministically.

**Manual QA:** Inspect a native capability snapshot against the current display and input hardware; disconnect/reconnect one available device and observe the updated context.
**After delivery:** complete I03 before downstream consumers proceed. Apply the per-change loop below if this package is split into multiple deliveries.

### S04 — Own and persist chess settings at the application root

**Depends on:** S01, S03. **Owner:** chess composition/settings controls.
Introduce the typed settings model, defaults, validation, root context, ordered/coalesced save coordinator, hydration gate, and error/Retry UI. Replace local settings values with root values without redesigning controls. Store preferences separately from game FEN; supply stable action/display/language IDs for downstream packages.
**Acceptance:** every value survives menu unmount, game restart, and native app restart; newest intent wins rapid writes; save failure leaves active state and reports unsaved status; Retry saves current intent; corrupt fields/maps use valid defaults; no startup effects see transient defaults before load completes.

**Manual QA:** Change a preference, navigate into and out of a game, then relaunch; force one save failure and use Retry after another change.
**After delivery:** complete I04 before downstream consumers proceed. Apply the per-change loop below if this package is split into multiple deliveries.

### S05 — Localize chess completely into English and French

**Depends on:** S04. **Owner:** chess text/catalogs/Trox integration.
Extract all player-facing static and dynamic strings, add complete English/French catalogs, and connect selection to existing localizer replacement. Include semantic names, announcements, binding/error dialogs, gameplay/promotion/results, and glyph coverage. Leave logs untranslated.
**Acceptance:** all routes and game states work in both languages with no unintended English; switch while a game/dialog is active without losing state/focus; catalog completeness validation catches omissions; retain representative native French evidence. Newly added downstream strings must extend these catalogs.

**Manual QA:** Play through a move and open a dialog in French; switch language while it is open and check retained selection, focus, and accented text.
**After delivery:** complete I05 before downstream consumers proceed. Apply the per-change loop below if this package is split into multiple deliveries.

### S06 — Scale all functional chess text without losing controls

**Depends on:** S05. **Owner:** menu/game text layout.
Replace role-dependent partial scaling with actual 1.0/1.5/2.0 readable-text scaling, avoiding inherited double multiplication. Preserve 100% styling and artwork. Add only needed wrapping, minimum dimensions, and scrolling; keep focus visible after a scale/language change.
**Acceptance:** English/French at all three scales, desktop and narrow mobile layouts; settings, promotion, pause/results, and binding dialogs remain reachable by each input mode; native captures prove no clipping/overlap and unchanged unscaled artwork.

**Manual QA:** Use French at 200% in a narrow native layout; reach the final control by scrolling and keyboard/controller, then return to 100% and compare artwork.
**After delivery:** complete I06 before downstream consumers proceed. Apply the per-change loop below if this package is split into multiple deliveries.

### S07 — Observe Windows and iOS reduced-motion preferences

**Depends on:** S03. **Owner:** Unity platform accessibility bridge.
Add native Windows/iOS system preference reads and change observations to the existing macOS/web contract. Use documented platform APIs and represent genuinely unavailable observations explicitly. Keep native bridge packaging in the supported build paths.
**Acceptance:** bridge compilation and injected host changes reach Rust; unavailable fallback preserves the player's in-game toggle; macOS/web behavior remains covered. S21/S23 must prove real system changes on Windows/iOS hardware; record that qualification as pending rather than blocking the first macOS slice or claiming a device pass.

**Manual QA:** Use an injected system preference change in a native host and observe its effective value; retain real Windows/iOS toggle checks for S21/S23 with that limitation explicit.
**After delivery:** complete I07 before downstream consumers proceed. Apply the per-change loop below if this package is split into multiple deliveries.

### S08 — Apply reduced motion and board-only screenshake

**Depends on:** S04, S07. **Owner:** chess motion/menu feedback.
Apply one effective policy everywhere; adapt essential piece slides and exempt only those adapted sequences from generic snapping. Implement bounded capture/checkmate shake with stable hit testing, suppress decorative effects, and align sound scheduling with actual arrival.
**Acceptance:** system OR toggle suppresses effects; a controlled intermediate frame proves a 120 ms slide still animates; castling/knight/capture/promotion and checkmate are coherent; switching policy settles correctly; disabling shake restores baseline; no duplicate sounds or stuck turn completion.

**Manual QA:** Watch a capture and checkmate, then disable shake mid-effect; inspect a retained intermediate reduced-motion frame and listen for arrival-aligned sound.
**After delivery:** complete I08 before downstream consumers proceed. Apply the per-change loop below if this package is split into multiple deliveries.

### S09 — Delay the computer response with cancellable application time

**Depends on:** S04. **Owner:** chess turn coordinator.
Insert the two-second wait after presentation readiness and before dispatch, keyed to session/position. Preserve remaining time during pause/menu/background and cancel on erasure/restart/unmount. Releasing the toggle dispatches at most once; never modify search strength or normal animation timing.
**Acceptance:** controlled-clock tests prove no early response, exactly one dispatch, suspend/resume remaining time, cancellation on position replacement, and no defer-after-dispatch race. Native scenario shows the visible pause without arbitrary sleep-based assertions.

**Manual QA:** Make a move, pause during the added wait, resume, and restart during a second wait; observe exactly one response or cancellation as appropriate.
**After delivery:** complete I09 before downstream consumers proceed. Apply the per-change loop below if this package is split into multiple deliveries.

### S10 — Erase saved chess progress without erasing preferences

**Depends on:** S04, S09. **Owner:** chess session/save integration.
Wire confirmation to quiescence, generation invalidation, ordered durable deletion, and initial-menu transition. Preserve every preference. Correct confirmation copy and expose recoverable failure/Retry/Cancel using existing-style UI.
**Acceptance:** erasure during pending save/computer work cannot recreate the game; cancellation changes nothing; restart after success has no resume and retained bindings/preferences; deletion failure retains recoverable progress and never announces success.

**Manual QA:** Erase during pending computer/save work, relaunch, and confirm no Resume while preferences remain; force deletion failure and verify the game is recoverable.
**After delivery:** complete I10 before downstream consumers proceed. Apply the per-change loop below if this package is split into multiple deliveries.

### S11 — Add shared music/effects buses and composable gains

**Depends on:** approval. **Owner:** audio protocol/Unity playback/motion audio.
Add category routing and master/category/mute gains to active and future playback, including sequence sounds. Separate source gain and fade envelope from mixer gains; cover pooled sources, crossfades, and replacement clips. Provide authoring controls through existing audio APIs.
**Acceptance:** measurable output follows the gain product, silence at zero, and changes preserve fades/playhead; reused sources do not inherit the wrong bus; protocol/fake coverage includes animation-sequence sounds; host tests verify active and future sources.

**Manual QA:** Use a small native audio consumer with music, effects, and a crossfade; adjust live master/category gains and verify silence at zero without a track restart.
**After delivery:** complete I11 before downstream consumers proceed. Apply the per-change loop below if this package is split into multiple deliveries.

### S12 — Route chess audio and background mute through shared settings

**Depends on:** S04, S11. **Owner:** chess menu/game audio integration.
Connect all music and effects to shared buses; remove competing menu/game gains and wire existing volume shortcuts to Master. Apply focus/pause mute to all sound, preserve the separate transient music-indicator mute, and handle browser resume restrictions honestly.
**Acceptance:** menu/game/crossfade/one-shot/sequence sounds obey each slider; active changes are audible and host-observable; blur/pause/hidden page mutes all when selected; return restores without restarting or replay; false adds no extra app mute.

**Manual QA:** Listen to menu/game audio while adjusting all sliders; background and return during playback, checking mute policy and absence of replay.
**After delivery:** complete I12 before downstream consumers proceed. Apply the per-change loop below if this package is split into multiple deliveries.

### S13 — Provide exclusive source-aware input capture

**Depends on:** S03. **Owner:** shared input transport/Reactant dispatch.
Preserve device, D-pad-versus-stick, and repeat metadata through the facade. Add scoped exclusive capture before menu/global routing, dynamic subscriptions for capture/bindings, opener-release suppression, and focus/disconnect cleanup. Keep menu navigation available outside capture.
**Acceptance:** capture emits exactly one eligible input with metadata; opener/repeats do not bind; captured input cannot activate gameplay/menu; focus loss/disconnect clears state; release/remount leaves no stuck capture. Cover native controller and fake contract boundaries.

**Manual QA:** Open capture using a held button, release, then bind another; verify no underlying action fires and disconnect/focus loss leaves no stuck capture.
**After delivery:** complete I13 before downstream consumers proceed. Apply the per-change loop below if this package is split into multiple deliveries.

### S14 — Make chess keyboard and controller bindings authoritative

**Depends on:** S04, S13. **Owner:** chess input table/router.
Use saved maps for seven actions, accurate defaults, shortcut precedence/reservations, duplicate/reset validation, and actual controller capture. Preserve existing cell appearance while making controller cells operable. Update input availability on hardware changes and localize new messages.
**Acceptance:** every action executes under its new binding and old aliases stop; Escape selection/Pause precedence and East cancellation work; left stick remains navigation; duplicate/reset conflicts change nothing; restart persists maps; capture cannot lock out standard menu recovery.

**Manual QA:** Remap and exercise all seven actions in gameplay, verify old bindings stop, then try a conflicting Reset and recover through standard menu cancellation.
**After delivery:** complete I14 before downstream consumers proceed. Apply the per-change loop below if this package is split into multiple deliveries.

### S15 — Apply and recover desktop display previews

**Depends on:** S01, S03. **Owner:** Unity desktop display transactions.
Implement typed resolution/mode preview, readback, confirm/cancel, 15-second host watchdog, durable prior/confirmed recovery records, and safe fallback after crash/monitor changes. Timeout must not depend on paused game clocks. Windows exclusive fullscreen only.
**Acceptance:** native Windows/macOS changes match readback; timeout/focus loss/failure reverts; stale IDs cannot confirm a newer preview; process termination during preview recovers prior confirmed state; confirmation persistence failure reverts; changed monitor has a usable fallback.

**Manual QA:** In a native player preview a valid display change and let it time out; repeat with focus loss and inspect restored actual state. Retain crash recovery evidence under the task acceptance.
**After delivery:** complete I15 before downstream consumers proceed. Apply the per-change loop below if this package is split into multiple deliveries.

### S16 — Apply platform-aware VSync and frame-rate caps

**Depends on:** S03. **Owner:** Unity frame pacing/capability policy.
Implement desktop synchronization/software-cap policy, browser caps, and iOS supported refresh divisions including ProMotion. Report supported/applied choices and reconcile device/monitor changes. Keep frame-rate controls separate from resolution preview rollback.
**Acceptance:** host pacing state matches the requested supported policy; desktop cap is restored when VSync is turned off; browser cap respects browser limits; iOS choices never promise unsupported refresh behavior. Validate actual pacing on hardware separately from configured-value tests.

**Manual QA:** Observe native frame pacing with VSync on and off at a supported cap; compare measured pacing with applied state. Leave web/iOS hardware claims to their qualification runs.
**After delivery:** complete I16 before downstream consumers proceed. Apply the per-change loop below if this package is split into multiple deliveries.

### S17 — Connect graphics controls and display confirmation

**Depends on:** S04, S15, S16. **Owner:** chess graphics settings UI integration.
Replace static choices with typed capability-backed options; hide platform-inapplicable rows and Max Framerate under desktop VSync. Add the 15-second Keep changes?/Revert dialog in the existing style, keep pending and applied state accurate, and persist only confirmed display pairs.
**Acceptance:** shown choices match host; hiding retains preferences; countdown/confirmation/cancellation work by pointer/keyboard/controller; failed/stale results cannot lie about state; device/monitor changes retain focus sensibly; no modification to unrelated menu design.

**Manual QA:** Change mode/resolution through settings, confirm once and cancel once; toggle VSync and check cap visibility and restoration without losing focus.
**After delivery:** complete I17 before downstream consumers proceed. Apply the per-change loop below if this package is split into multiple deliveries.

### S18 — Implement and document best-effort crash-report control

**Depends on:** S04. **Owner:** Unity Diagnostics module/chess reporting preference.
Investigate exact-version APIs and wire supported control with early saved-choice restoration. Keep default on, vendor unchanged, and requested versus observed capability distinct. Provide honest unsupported/partial/configuration feedback; do not provision services or claim legal compliance.
**Acceptance:** supported local calls/readback and restored intent are tested; when configured, retain managed/native/queued-upload checks across restart; otherwise record the missing project/access and exact unverified paths. Close as best-effort only with a concrete implementation and limitation record, not an untested boolean menu.

**Manual QA:** Toggle reporting, relaunch, and inspect restored intent and configuration feedback; explicitly distinguish local API results from any unverified remote uploads.
**After delivery:** complete I18 before downstream consumers proceed. Apply the per-change loop below if this package is split into multiple deliveries.

### S19 — Prepare reproducible physical iOS chess validation

**Depends on:** approval. **Owner:** existing iOS build tooling.
Extend the simulator groundwork only as needed for a reproducible arm64 iOS device chess build, native Rust linking/exports, assets, accessibility bridge packaging, and install/run evidence. Use supplied signing configuration; do not invent signing credentials or include Android.
**Acceptance:** device artifact builds and launches with identified toolchain/source/device; persistence path and input bridge function; simulator versus device evidence is explicit. Missing SDK/signing/hardware is a named blocked prerequisite, not a successful simulator substitution.

**Manual QA:** Install and launch the built artifact on a physical iOS device, change one available preference, terminate, and relaunch; record signing/device/build identity.
**After delivery:** complete I19 before downstream consumers proceed. Apply the per-change loop below if this package is split into multiple deliveries.

### S20 — Qualify the complete macOS settings vertical slice

**Depends on:** S06, S08, S10, S12, S14, S17, S18.
Exercise settings together on a real native macOS player with retained Ditto scenarios and actual display/audio/storage checks. Repair integration defects in scope; ensure new dialogs are translated/scaled and motion-aware. Catalog/style checks cover all later-added strings and controls.
**Acceptance:** native restart durability, failed-save Retry, erase races, real remapped actions, French 200%, motion/audio timing, desktop rollback, and background behavior pass. Reporting has explicit best-effort evidence. Record exact build/scenario artifacts, then hand the proven integration to platform qualification.

**Manual QA:** Play a short native game segment while switching language/text size, audio, and one binding; reopen settings and restart the app to catch integration issues beyond isolated scenarios.
**After delivery:** complete I20 before downstream consumers proceed. Apply the per-change loop below if this package is split into multiple deliveries.

### S21 — Qualify Windows settings and display recovery

**Depends on:** S20. **Owner:** Windows native qualification.
Run equivalent functional checks on Windows, emphasizing exclusive fullscreen, resolution/watchdog/crash recovery, VSync/caps, system reduced motion, filesystem behavior, and keyboard/gamepad hotplug. Add only missing target execution support needed for this scope.
**Acceptance:** actual Windows player evidence verifies each platform-specific contract; macOS/fake output never stands in for Windows. Failures become repaired in-scope code or explicitly blocked evidence; no completion claim while required hardware/run evidence is absent.

**Manual QA:** Use the Windows player interactively: change fullscreen mode, background/return, and reconnect a gamepad; compare the controls with actual host state.
**After delivery:** complete I21 before downstream consumers proceed. Apply the per-change loop below if this package is split into multiple deliveries.

### S22 — Qualify browser persistence, input, and lifecycle

**Depends on:** S02, S20. **Owner:** browser-specific qualification.
Use the actual threaded chess WebGL build. Verify durable reload/delete, storage denial/quota, rapid preference writes, browser hidden/focus audio behavior and resume, rate caps, supported physical input detection, and correct hidden native-only settings. Avoid duplicating native visual testing in a browser.
**Acceptance:** retained browser runs establish the web-specific contracts, cross-origin/thread startup succeeds, and failures do not masquerade as durable writes. Record browser/build identity; repair selected browser compatibility checks and any declared web-contract risks.

**Manual QA:** Use the actual browser player: change a preference, reload, hide/restore the page, and connect available input hardware; inspect failures rather than treating UI values as proof.
**After delivery:** complete I22 before downstream consumers proceed. Apply the per-change loop below if this package is split into multiple deliveries.

### S23 — Qualify iOS settings on physical hardware

**Depends on:** S19, S20. **Owner:** iOS physical-device qualification.
Exercise restart durability, game-only erasure, background/resume audio, system motion changes, supported frame pacing, touch-only/attached input visibility, and English/French large text on an iPhone. Include external keyboard and gamepad evidence for exposed remapping.
**Acceptance:** actual device identity, build, refresh/ProMotion configuration, inputs, actions, and retained outcomes are recorded. No native-only display rows appear. Hardware restrictions are distinguished from bugs; simulator output cannot certify device lifecycle/audio/refresh behavior.

**Manual QA:** On a physical iPhone use large French text, background/return during audio, and attach input hardware; verify reachable controls and persisted values after relaunch.
**After delivery:** complete I23 before downstream consumers proceed. Apply the per-change loop below if this package is split into multiple deliveries.

## Per-change QA and introspection

For every delivered implementation change, run a brief hands-on check of the
changed behavior and one relevant edge or failure case before closing its task.
Usually a few minutes and a short note suffice; do not repeat the full platform
matrix after every edit. The package examples above are starting points. Use a
native player for shared behavior and the browser for browser-specific behavior.
For a library/tool change without exposed UI, manually exercise a small real
consumer or the tool itself and inspect its result. Required deterministic tests,
retained native Ditto evidence, CI, and final platform qualification still apply.

Record the source/build, steps, expected versus observed result, and any remaining
limitation in the bead. Link a retained capture or replay when it demonstrates a
visual/timing claim; a concise observation is enough for a nonvisual check. Fix
confirmed defects and recheck the affected path. Never write "QA passed" from
source inspection alone. Unavailable hardware is pending evidence, not a pass.

After each S package, complete its paired I package using the actual diff, QA
observations, authoring code, and tool/CI runs. These are dedicated engineering
work, not a closing checklist or a final retrospective deferred until the end.
Every I package must answer:

1. How could Reactant/Battlement have made this easier and more idiomatic? Which
   ownership, composition, lifecycle, or type boundary should move into the library?
2. Why would the same authoring task be simpler in React? Compare one concrete
   current example with an equivalent React pattern, including cleanup/state
   behavior. Name missing utilities; if React is not simpler, explain the evidence.
3. How could the tools be faster, more reliable, or easier to inspect? Examine
   build, Ditto, debugging, and CI feedback. Retain observed timings/failure handles
   where relevant; identify wasted repetition, weak diagnostics, and flaky checks.
4. What code complexity, duplication, awkward API, or testing gap should improve?
   Separate chess domain rules from reusable engine responsibilities.
5. What changes follow from these findings, who owns them, and which consumers
   must wait? Record the disposition of every actionable gap and native bead IDs
   for improvements that need separate work.

File follow-up beads for **larger-scope items and improvements** found during
introspection; a retrospective note or vague future-work list is insufficient.
Examples include reusable library APIs, broader refactors, missing engine
utilities, and tooling/CI reliability or performance projects. Give each bead a
bounded scope, creator/source-task links, evidence, acceptance, and real
prerequisites. File these throughout the project as findings emerge. Search for
existing work first and link/update it instead of duplicating it.

Small local improvements may stay in the current change when it is still open
and the work is in scope; record what was fixed and recheck it. Larger work gets
its own bead rather than silently expanding the current implementation. After
delivery, new implementation work is tracked separately.
Classify findings as needed for this project's correctness/engine outcome or
independent improvements. Attach required fixes as prerequisites to the affected
consumer and final engine acceptance; deliver them before declaring that outcome
complete. Keep independent improvements in the backlog with their disposition
recorded. Do not hide a required library improvement in an indefinitely deferred
backlog or count filing a bead as delivering the improvement.

Every substantive follow-up implementation also gets its own paired introspection
bead. If an S package delivers multiple distinct changes, create the extra pairs
as the work is split, so each delivery has a reflection before dependent work.
Introspection-only notes do not recursively require another introspection bead.
No arbitrary finding quota applies, but "no follow-up" needs a concrete explanation
covering the questions above; the project-wide Reactant change requirement remains.
New work inherits the current authorization boundary: filing never authorizes
execution. Material scope expansion stays deferred until explicitly approved.

### Introspection dependency policy

I01–I23 each depend on their matching S package and the approval decision.
Whenever an S package depends on another S package, it also depends on that
producer's I package. Independent branches remain independent. This makes the
findings available before the next consumer design is committed. Coordinate any
new prerequisites with active owners; never rewrite another executor's graph
silently. Inspect native holds and edges after filing.

### E01 — Accept delivered Reactant and engine improvements

**Depends on:** I01–I23 and all required follow-up improvements discovered by them.
Review the actual delivered Reactant changes and their consumers. Retain a compact
before/after authoring example, identify the complexity now handled by reusable
library code, and show behavioral evidence for that abstraction. Summarize the
introspection findings by their disposition and linked follow-up IDs, including
tool/CI improvements delivered or deliberately left as independent work.
**Acceptance:** substantive Reactant code/API/utility improvements are delivered
and exercised by chess; no sample workaround remains for a required engine fix;
all paired reflections and required follow-ups have accepted evidence. A working
chess UI with unchanged Reactant, or only filed future improvements, fails.
**Manual QA:** exercise one representative library consumer using the delivered
API and inspect its behavior and authoring path; confirm the claimed simplification
is real. This acceptance-only task is not another implementation package. Any fix
it discovers must be filed with its own QA/introspection pair before E01 closes.

## Validation and release decisions

Use black-box Rust/fake tests for state, ordering, failures, and deterministic
timers; focused Unity tests for host gains, protocol, capability, and transaction
boundaries; native Ditto for shared UI/motion evidence. Test both individual
settings and intersections: French at 200%, system reduction plus Screenshake,
muting during a crossfade, binding while a controller disconnects, deleting while
autosave is pending, and quitting during display preview. Follow
[web contracts](../web/contracts.toml) for browser-specific risks.

Feature tasks run focused checks and the required staged aggregate CI. Qualification
retains exact source/build/run identities, artifacts, device/browser details, and
the actual pass/failure/unavailable result. Do not replace missing hardware with
fakes, screenshots with source inspection, or upload proof with successful API
calls. Keep evidence in retained run artifacts/bead handoffs rather than maintained
guidance. No task may silently broaden to Linux/Android, another diagnostics
vendor, a menu redesign, or public hosting.

After explicit execution approval, release the approval decision as completed,
then remove deferral from only the authorized scope; normal dependency edges still
govern readiness. Closing the document-authoring work is not approval. The epic
completes only when S01–S23, I01–I23, E01, and required follow-up improvements
have accepted evidence; S18 alone allows the documented best-effort exception.
Missing Windows/iOS hardware leaves qualification open.

## Native bead index

Epic: **hv-ou5**. Approval decision: **hv-ou5.1**.

| Package | Bead | Blocking packages and reflections (approval is also required) |
| --- | --- | --- |
| S01 | `hv-ou5.2` | None |
| S02 | `hv-ou5.3` | S01, I01 |
| S03 | `hv-ou5.4` | None |
| S04 | `hv-ou5.5` | S01, S03, I01, I03 |
| S05 | `hv-ou5.6` | S04, I04 |
| S06 | `hv-ou5.7` | S05, I05 |
| S07 | `hv-ou5.8` | S03, I03 |
| S08 | `hv-ou5.9` | S04, S07, I04, I07 |
| S09 | `hv-ou5.10` | S04, I04 |
| S10 | `hv-ou5.11` | S04, S09, I04, I09 |
| S11 | `hv-ou5.12` | None |
| S12 | `hv-ou5.13` | S04, S11, I04, I11 |
| S13 | `hv-ou5.14` | S03, I03 |
| S14 | `hv-ou5.15` | S04, S13, I04, I13 |
| S15 | `hv-ou5.16` | S01, S03, I01, I03 |
| S16 | `hv-ou5.17` | S03, I03 |
| S17 | `hv-ou5.18` | S04, S15, S16, I04, I15, I16 |
| S18 | `hv-ou5.19` | S04, I04 |
| S19 | `hv-ou5.20` | None |
| S20 | `hv-ou5.21` | S06, S08, S10, S12, S14, S17, S18, I06, I08, I10, I12, I14, I17, I18 |
| S21 | `hv-ou5.22` | S20, I20 |
| S22 | `hv-ou5.23` | S02, S20, I02, I20 |
| S23 | `hv-ou5.24` | S19, S20, I19, I20 |

| Reflection | Bead | Delivered package to inspect (approval also required) |
| --- | --- | --- |
| I01 | `hv-ou5.25` | S01 |
| I02 | `hv-ou5.26` | S02 |
| I03 | `hv-ou5.27` | S03 |
| I04 | `hv-ou5.28` | S04 |
| I05 | `hv-ou5.29` | S05 |
| I06 | `hv-ou5.30` | S06 |
| I07 | `hv-ou5.31` | S07 |
| I08 | `hv-ou5.32` | S08 |
| I09 | `hv-ou5.33` | S09 |
| I10 | `hv-ou5.34` | S10 |
| I11 | `hv-ou5.35` | S11 |
| I12 | `hv-ou5.36` | S12 |
| I13 | `hv-ou5.37` | S13 |
| I14 | `hv-ou5.38` | S14 |
| I15 | `hv-ou5.39` | S15 |
| I16 | `hv-ou5.40` | S16 |
| I17 | `hv-ou5.41` | S17 |
| I18 | `hv-ou5.42` | S18 |
| I19 | `hv-ou5.43` | S19 |
| I20 | `hv-ou5.44` | S20 |
| I21 | `hv-ou5.45` | S21 |
| I22 | `hv-ou5.46` | S22 |
| I23 | `hv-ou5.47` | S23 |

Final engine acceptance: **E01 / hv-ou5.48**, blocked by I01–I23 and any required follow-up improvements.
