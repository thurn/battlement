# 47. Validate native, threaded WebGL, and mobile builds

The completed engine and migrated samples have one reproducible final functional
validation set with explicit physical-certification status.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Validation](../validation.md)
- [Sample migration](../migration.md)
- [Test scenes](../fixtures.md)
- [Hearts rules and behavior](../hearts.md)

**Prerequisite:** [Task 46: Measure complete-card workloads and repair
structural hotspots](46-performance-workloads.md) and all its required
follow-ups must be integrated.

**Starting code:** All sample scenarios; release builders; public worker/native
fixtures; certification checklist.

## Example

Keep actual platform evidence separate from a simulator or fake result:

```text
native release: full sample and cancellation scenarios
threaded desktop WebGL release: threading, cleanup, durable saves
iOS/Android: built artifacts and available automated runs
physical iPhone 17/Galaxy S25: actual result or explicitly not run
```

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
   concurrency/identity/command-queue contracts remain mandatory.

## Acceptance

- All final required functional scenarios and aggregate CI pass on their real
  applicable adapters.

- Release evidence distinguishes real threading, actual cleanup, and true
  storage durability from fake/simulator substitutes.

- Existing sample behavior is preserved and Hearts is playable with all input
  modes and resume.

- Physical iPhone 17/Galaxy S25 certification is explicitly not run or linked to
  actual evidence; it does not block the completed overhaul.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Physical certification execution is the named follow-up; architectural
retirement cleanup remains task 48.

## Manual QA

Perform the final Hearts walkthrough, a chess/tic-tac-toe regression pass, and
the inspector's cancellation/replay flows in actual release players.
