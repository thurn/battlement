# 47. Run final native, threaded-WebGL, and mobile build conformance

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Validation contract](../validation.md)
- [Migration contract](../migration.md)
- [Fixtures contract](../fixtures.md)
- [Hearts contract](../hearts.md)

**Prerequisite:** [Task 46: Measure complete-card workloads and repair
structural hotspots](46-performance-workloads.md) and all its required
follow-ups must be integrated.

**Source roles:** All sample scenarios; release builders; public worker/native
fixtures; certification checklist. Resolve these through source-map.md; its
links track the current owner after crate moves. Inspect the concrete caller and
host/fake counterpart before editing.

## Result

The completed engine and migrated samples have one reproducible final functional
validation set with explicit physical-certification status.

## Implementation

1. Run all applicable native sample selections and public-driver suites from the
   final integrated code, including basic/ui regression coverage and the
   implemented chess-ui pages.

2. Run actual release native and threaded-WebGL cancellation/cleanup/panic
   fixtures, Hearts full-flow/save durability, and representative mixed world/UI
   motion/input/asset cases.

3. Build the completed fixtures/samples for iOS and Android and run available
   automated platform scenarios. Update certification.md with exact final
   build/fixture inputs for physical device execution.

4. Check the proposed requirement-to-task coverage matrix against concrete
   evidence; any uncovered requirement is a defect to repair, not a silently
   accepted deferral.

5. Confirm performance reports remain advisory, while simulation and
   concurrency/identity/commit contracts remain mandatory.

## Acceptance

- All final required functional scenarios and aggregate CI pass on their real
  applicable adapters.

- Release evidence distinguishes real threading, actual cleanup, and true
  storage durability from fake/simulator substitutes.

- Existing sample behavior is preserved and Hearts is playable with all input
  modes and resume.

- Physical iPhone 17/Galaxy S25 certification is explicitly not run or linked to
  actual evidence; it does not block the completed overhaul.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Physical certification execution is the named follow-up; architectural
retirement cleanup remains task 48.

## Manual QA

Perform the final Hearts walkthrough, a chess/tic-tac-toe regression pass, and
the inspector's cancellation/replay flows in actual release players.
