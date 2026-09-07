# Validation and release evidence

Validate engine behavior through the public display API, then check actual Unity
rendering and integration with native scenarios. Read this for each task's
acceptance and review evidence. Also read the applicable repository [CI
skill](../../.agents/skills/battlement-ci/SKILL.md), [Ditto
skill](../../.agents/skills/battlement-ditto/SKILL.md), and [web
skill](../../.agents/skills/battlement-web/SKILL.md) when relevant.

## Observable tests

Primary engine/game coverage is standalone Rust scenario files through
reactant-testing's public display driver and Battlement fakes. Test inputs are
public actions, prompt answers, pointer/key/controller input, virtual time, and
frame advancement. Outputs are presented objects/text/poses, effects/audio,
prompt/checkpoint identities, accepted-state copies, and run lifecycle.

Do not assert private tree maps, internal channel occupancy, generated commands,
or hidden Hearts hands as a substitute for visible behavior. Small unit tests
are justified for complex algorithms such as dependency-cycle detection or
projection mathematics, but cannot replace end-to-end scenarios.

Each task adds the acceptance scenarios named in its page. Create complete
fixtures with stable seeds and public barriers. Never wait a guessed number of
milliseconds for a worker. Wall-clock timeouts detect hangs; they do not control
the expected order of execution.

## Display driver contract

The rules/session surface is fixed in [interfaces](interfaces.md). The separate
public display test driver supplies worker barriers and virtual host controls,
including these operations:

```rust
display.dispatch(action);
display.wait_for_render_submission(); // Rust has handed commands to the host.
display.advance_time(Duration::from_millis(125));
display.object(card_id).assert_position(halfway);
display.settle();
let prompt = display.wait_for_prompt();
// Match PresentedPrompt and submit through its typed handle.
```

Provide wait_for_worker_started/stopped and builder-entered observations for
controlled fixture builders, plus wait_for_render_submission and settle.
Submission waits synchronize the Rust consumer without advancing host time.
Observe actual playback separately through host poses, text, and effects.
Document that settle advances only finite work and stops at an unanswered
prompt; it must not spin forever on cosmetic loops. Seeking an inspection copy
cannot finish the active gameplay batch. `advance_frame` remains available for
visual observations, with no mandatory per-snapshot frame boundary.

Virtual time, worker scheduling, and rendered frames are independent. Each
helper specifies which it advances. Deterministic synchronization may pump
queued main-thread work but cannot advance presentation time implicitly. Use
bounded event-driven waits; no fake-only direct call to a rules closure.

## Host conformance

For each new host capability, test it through the Rust fake and a native Ditto
test scene. The two executors consume the same public protocol, but a fake
result alone does not establish Unity behavior.

Required native coverage includes interpolation, text/sprite ordering, shader
overrides, particles/audio timing, input capture/modals, live anchor movement,
asset command dependencies, command-operation completion, and safe input while
gameplay commands are queued. Inspect actual visible behavior; do not require a
whole-display atomic swap or a rendered-frame receipt.

Protocol fixture tests must include serialization, Unity consumption, and
correlated returned events. Exercise duplicate delivery and stale IDs, not only
a happy-path animation.

## Platform evidence

Tasks 03-05 establish actual Rust/Unity worker cancellation and release-build
plumbing early. Desktop native and threaded desktop WebGL are required
functional integration targets. Preserve existing macOS/Windows support. Prepare
iOS and Android build paths and reproducible device scenarios here; physical
iPhone 17/Galaxy S25 execution is a separately tracked certification.

For each supported release build, verify panic=unwind configuration. Native and
threaded WebGL fixtures must demonstrate nested rules unwinding, destructor
cleanup before worker-stopped, silent cancellation, a distinct real panic, and
no unwind crossing the C ABI. A Rust-only catch_unwind test is insufficient.

WebGL must use actual threading, cross-origin isolation, SharedArrayBuffer,
compatible exception handling, and the actual release plugin/player path. Remove
the current discrepancy where the Ditto WebGL builder disables threads. Do not
quietly fall back to main-thread rules, panic_abort, or fake cancellation.

If an early platform proof fails, repair the toolchain/integration in that task
or report the concrete blocker before dependent worker implementation proceeds.
Do not declare the architecture supported from compiler flags alone.

## Checks and evidence

Use the current checkout's commands, with the relevant sample manifest:
- cargo test -p reactant-rules
- cargo test -p reactant-core
- cargo test -p reactant-testing
- cargo test --manifest-path samples/hearts/rules/Cargo.toml
- The selected sample's Ditto scenarios through the documented CLI.

These package names are targets introduced by the task sequence, not commands
available at the planning baseline. Before task 06, existing UI checks use
battlement-reactant. Follow source-map for relocation.

Stage intended inputs and run ./scripts/ci.py successfully before completing
every implementation task, including this plan's documentation updates. Use
retained CI logs and exact replay inputs for failures; no competing reruns.
Inspect and stage intentional metadata produced by CI.

For >500 non-test-line changes, apply the repository's independent-review skill
once in that task's session, verify findings, and fix confirmed issues. A task
page's line count is not a reason to skip a necessary shared-engine fix.

Web-visible tasks require the existing verified demo/tunnel lifecycle. Native
Ditto is the primary gameplay validation; interactive web testing covers the
web-specific host and review walkthrough. Use only the configured Playwright MCP
browser service for automation.

## Performance and certification

The benchmark workloads and reporting targets are in [fixtures](fixtures.md).
Run the fixed release captures and publish measured distributions and misses.
The numeric values are not completion gates. Simulation skips snapshot/event
builders and display waits. Primitive calls require no extra allocation or
vtable; context-mode branching is allowed. Measure owned prompt construction and
policy work separately, and prove the primitive contract independently of the
frame-rate targets.

Keep a physical-device certification checklist with build IDs, commands,
fixture/deal, expected observations, and fields for device/OS/results. Missing
physical evidence is reported as not run, never inferred from a simulator. The
final overhaul can complete with that certification explicitly outstanding.

## Manual QA

Use each task's exact test scene and reset instructions. Inspect rendered output
when appearance changes. For final acceptance, play Hearts, replay every
migrated sample's required scenarios, then exercise the laboratory's failure,
identity, and timing cases through the public inspector.
