# 04. Prove cancellation in the threaded WebGL release path

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Execution contract](../execution.md)
- [Validation contract](../validation.md)

**Prerequisite:** [Task 03: Prove native worker cancellation and Rust
cleanup](03-native-cancellation.md) and all its required follow-ups must be
integrated.

**Source roles:** Plugin builds; Unity release/adapter builders; native
cancellation fixture from task 03. Resolve these through source-map.md; its
links track the current owner after crate moves. Inspect the concrete caller and
host/fake counterpart before editing.

## Result

The real threaded WebGL plugin/player executes and catches the same cancellation
unwind as native.

## Implementation

1. Replace the current panic_abort build-std choice for interactive WebGL with
   compatible unwind support, and align Rust exception/thread flags with the
   Unity Emscripten link settings.

2. Make the Ditto WebGL build path use the actual threaded release configuration
   instead of its current threads-disabled path. Reuse the sample builder's
   threading setup rather than maintaining conflicting settings.

3. Serve the fixture with cross-origin isolation and assert actual
   SharedArrayBuffer/thread operation. Verify the rules action runs off the UI
   thread and the cancellation boundary stays inside Rust.

4. Retain a reproducible release build/run check for nested cleanup, endpoint
   cancellation, and ordinary panic. Treat unsupported toolchain behavior as a
   task blocker to repair, not permission to substitute abort or cooperative
   main-thread execution.

## Acceptance

- The browser-hosted release fixture shows worker-started, silent cancellation,
  destructor cleanup, and worker-stopped in order.

- A real panic is visible as an execution failure; a replacement run rejects
  stale results.

- The tested build proves actual threading and isolation, and its build inputs
  enforce the required panic/runtime compatibility.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Physical mobile certification and performance thresholds are not this task.
Mobile build paths follow in task 05.

## Manual QA

Use the configured Playwright MCP service to open the actual release fixture.
Cancel it while opening a local menu; verify responsiveness and retained
lifecycle evidence.
