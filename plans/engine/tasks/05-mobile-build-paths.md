# 05. Prepare iOS and Android release validation paths

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Execution contract](../execution.md)
- [Validation contract](../validation.md)

**Prerequisite:** [Task 04: Prove cancellation in the threaded WebGL release
path](04-threaded-webgl-proof.md) and all its required follow-ups must be
integrated.

**Source roles:** Plugin builds; Unity release/adapter builders; repository
validation. Resolve these through source-map.md; its links track the current
owner after crate moves. Inspect the concrete caller and host/fake counterpart
before editing.

## Result

The cancellation fixture and future samples have reproducible iOS/Android build
and validation entrypoints.

## Implementation

1. Extend the generic plugin/player build configuration to cover native iOS and
   Android architectures using the project's configured toolchains and Unity
   modules. Keep threading and unwind settings explicit in release builds.

2. Reuse the existing iOS Simulator integration path and add the Android
   equivalent build/fixture runner. Separate simulator/emulator proof from
   physical-device evidence in results.

3. Verify the native library exports and link/runtime compatibility; run the
   cancellation fixture on available automated simulator/emulator targets.
   Required SDK/module setup is an explicit prerequisite, not a silently skipped
   check.

4. Add a physical-certification document under plans/engine/certification.md
   with exact build/fixture invocation, device evidence fields, and pending
   iPhone 17/Galaxy S25 functional/performance checks. Do not assert that
   physical certification has run.

## Acceptance

- Both mobile release artifacts build through reproducible commands with
  panic=unwind and compatible native threading.

- Automated mobile runners report target identity and actual pass/failure, never
  substitute desktop output.

- The physical certification handoff explains cleanup-before-stopped, real-panic
  distinction, all key flows, and sustained captures without blocking this
  overhaul on physical access.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Physical device execution remains a separate certification. Mobile sample
content is added by its owning later task.

## Manual QA

Inspect build metadata and run the fixture through the available
simulator/emulator path. Confirm the physical checklist clearly distinguishes
not-run evidence.
