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

Each task proves its acceptance behaviors using existing coverage where it fits.
Add a scenario only for a distinct behavior or failure mode not already covered;
one scenario may satisfy multiple bullets. Use representative combinations plus
explicit boundaries, not Cartesian products of layouts, effects, inputs, and
platforms. Keep fixtures small, with stable seeds and public barriers. Wall-clock
timeouts detect hangs; they never control worker ordering. Rules-only algorithms
may be checked through the public rules API without a display or native fixture.

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
prompt; it must not spin forever on cosmetic loops. `advance_frame` remains available for
visual observations, with no mandatory per-snapshot frame boundary.

Virtual time, worker scheduling, and rendered frames are independent. Each
helper specifies which it advances. Deterministic synchronization may pump
queued main-thread work but cannot advance presentation time implicitly. Use
bounded event-driven waits; no fake-only direct call to a rules closure.

## Host conformance

For each new host capability, cover logical behavior through the public driver
and obtain focused native evidence for what the fake cannot establish. Reuse one
scene across related capabilities. A fake result alone does not establish Unity
rendering; native checks need not repeat every logical permutation.

Required native coverage includes interpolation, text/sprite ordering, shader
overrides, particles/audio timing, input capture/modals, live anchor movement,
asset command dependencies, command-operation completion, and safe input while
gameplay commands are queued. Inspect actual visible behavior; do not require a
whole-display atomic swap or a rendered-frame receipt.

Changed protocol capabilities need serialization and Unity-consumption coverage,
plus returned events where that capability has them. Reuse existing protocol
checks. Duplicate delivery and stale-ID scenarios belong to their engine owners
and are reused by later features unless a new failure mode warrants extension.

## Platform evidence

Tasks 03-05 establish actual Rust/Unity worker cancellation and release-build
plumbing early. Desktop native and threaded desktop WebGL are required
functional integration targets. Preserve existing macOS/Windows support. The mobile gate is minimal: build the cancellation fixture and Hearts for iOS
and Android, run the fixture on one iOS Simulator and one Android emulator, and
smoke-test Hearts launch, a pass/card play, menu pause/resume, and restart on each.
Task 05 establishes the fixture path; task 47 adds the completed Hearts smoke.
Keep existing sample regression coverage, without adding an all-sample mobile
matrix. Missing required SDKs/modules are explicit blockers. Physical
iPhone 17/Galaxy S25 execution is separate certification.

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

Use the affected package checks below once their owning tasks introduce them.
The required aggregate CI remains the final gate; do not separately rerun every
package suite merely because its package now exists:
- cargo test -p reactant-rules
- cargo test -p reactant-core
- cargo test -p reactant-ui
- cargo test -p reactant
- cargo test -p reactant-testing
- cargo test -p rt
- cargo test --manifest-path samples/hearts/rules/Cargo.toml
- A selected Reactant project's Ditto scenarios through `rt ditto` with an
  explicit configuration path. Repository sample shortcuts use the documented
  `just` recipes.

After task 07, CLI contract tests must also prove:

- Cargo metadata exposes `rt` as the sole project-tool binary, removes
  `cargo-battlement`, and leaves `battlement-ditto` library-only.
- Help/parser behavior exposes the general subcommands without sample names
  or Battlement/Reactant namespaces; a full help snapshot is not required.
- An external project whose path contains spaces resolves `reactant.toml`,
  applies explicit-over-file precedence, and reports relative paths from the
  project root.
- `rt run` invokes the same build operation as `rt build`, and build/preparation
  failure prevents player launch.
- Plugin, typed Addressables, Reactant asset, and Ditto contract tests preserve
  their arguments, exit behavior, and interruption behavior through `rt`.
- Reactant Ditto fixtures run shared asset preparation before the generic Ditto
  library; direct Battlement fixtures skip it; the Ditto library has no Reactant
  dependency.
- Inspect repository recipes and parameterized scripts to verify that only
  `justfile` chooses sample names/defaults. Exercise representative recipes; do
  not add source-text assertion tests for this structural ownership rule.

Before a package's creating task, run its predecessor checks from the source
map. In particular, existing UI checks use battlement-reactant before task 06,
and public display-driver checks begin in task 08.

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
