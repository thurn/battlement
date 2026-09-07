# 04. Prove cancellation in the threaded WebGL release path

The real threaded WebGL plugin/player executes and catches the same cancellation
unwind as native.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Rules and choices](../execution.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 03: Prove native worker cancellation and Rust
cleanup](03-native-cancellation.md) is integrated.

**Starting code:** Plugin builds; Unity release/adapter builders; native
cancellation fixture from task 03.

## Example

The proof must run through the browser-hosted Unity player, not only a Rust test
executable:

```text
release WebGL player: actual thread starts the rules action
Cancel: Rust unwinds and runs its nested destructors
replacement menu: still responds
ordinary panic: reports execution failure instead of cancellation
```

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

4. Retain a reproducible release build/run check for nested cleanup, worker
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

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Physical mobile certification and performance thresholds are not this task.
Mobile build paths follow in task 05.

## Manual QA

Use the configured Playwright MCP service to open the actual release fixture.
Cancel it while opening a local menu; verify responsiveness and retained
lifecycle evidence.
