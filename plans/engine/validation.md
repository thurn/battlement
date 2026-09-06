# Validation and release evidence

Read this for every task's acceptance and handoff. Also read the applicable
repository [CI skill](../../.agents/skills/battlement-ci/SKILL.md), [Ditto
skill](../../.agents/skills/battlement-ditto/SKILL.md), and [web
skill](../../.agents/skills/battlement-web/SKILL.md) when relevant.

## Observable tests

Primary engine/game coverage is standalone Rust scenario files through
reactant-testing's public display driver and Battlement fakes. Test inputs are
public actions, prompt answers, pointer/key/controller input, virtual time, and
frame advancement. Outputs are presented objects/text/poses, effects/audio,
prompt/checkpoint identities, accepted-state availability, and run lifecycle.

Do not assert private tree maps, internal channel occupancy, generated commands,
or hidden Hearts hands as a substitute for visible behavior. Small unit tests
are justified for complex algorithms such as dependency-cycle detection or
projection mathematics, but cannot replace end-to-end scenarios.

Each task adds the acceptance scenarios named in its page. Create complete
fixtures with stable seeds and public barriers. Never wait a guessed number of
milliseconds for a worker. Wall-clock timeouts detect hangs; they do not control
the expected order of execution.

## Display driver contract

The intended public surface includes these operations (names may evolve):

~~~rust
display.dispatch(action);
display.wait_for_prompt();
display.answer(request, answer);
display.advance_time(Duration::from_millis(125));
display.advance_frame();
display.advance_to_label("ready");
display.object(card_id).assert_in_layout(hand);
~~~

Provide wait_for_worker_started/stopped and builder-entered observations for
controlled fixture builders, plus advance_to_next_checkpoint and settle.
Document that settle advances only finite work and stops at an unanswered
prompt; it must not spin forever on cosmetic loops or silently satisfy a gate by
seeking.

Virtual time, worker scheduling, and rendered frames are independent. Each
helper specifies which it advances. Deterministic synchronization may pump
queued main-thread work but cannot advance presentation time implicitly. Use
bounded event-driven waits; no fake-only direct call to a rules closure.

## Host conformance

For each new host capability, test it through the Rust fake and a native Ditto
specimen. The two executors consume the same public protocol, but a fake result
alone does not establish Unity behavior.

Required native coverage includes interpolation, text/sprite ordering, shader
overrides, particles/audio timing, input capture/modals, live anchor movement,
inactive preparation, and no mixed commit generation on input.

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
The numeric values are not completion gates. Simulation's no-builder,
no-mandatory-allocation, static-dispatch contract is still a correctness/API
requirement and must be proven separately.

Keep a physical-device certification checklist with build IDs, commands,
fixture/deal, expected observations, and fields for device/OS/results. Missing
physical evidence is reported as not run, never inferred from a simulator. The
final overhaul can complete with that certification explicitly outstanding.

## Manual QA

Use each task's exact specimen and reset instructions. Inspect rendered output
when appearance changes. For final acceptance, play Hearts, replay every
migrated sample's required scenarios, then exercise the laboratory's failure,
identity, and timing cases through the public inspector.
