# Battlement logging

## Goal

Battlement sends Rust and managed diagnostics through Unity's logging APIs so they
appear in the Editor console, player logs, and Unity diagnostics. The in-game viewer
shows the same current-run records without owning a second persistent log stream.

Caught Rust panics retain their formatted Rust backtrace and the tracing records that
preceded them. Focused error reports remain available beneath
`Application.persistentDataPath/Battlement/Errors`.

## Rust tracing bridge

`battlement-native` installs a process-wide `tracing_subscriber` layer before an
engine is created. The layer converts every event and its active span names into a
structured record containing its timestamp, severity, event name, message, and
fields. Unity assigns the constant Rust source and its unified arrival sequence.

The subscriber serializes records into a thread-safe native queue. The queue retains
the newest 2,048 records. If Unity does not drain it before it fills, the oldest
record is discarded and the next drain begins with a warning containing the dropped
record count.

`battlement_engine_create` installs the tracing subscriber before invoking the game
factory. `battlement_logging_drain` returns all queued records as UTF-8 JSON Lines
and empties the queue.

The queue is an interop boundary, not a persistence mechanism. It permits tracing
from any Rust thread without calling managed code or a Unity API from that thread.

## Unity integration

A runtime bootstrap creates the persistent logging host. Native engine creation owns
tracing initialization, so every caller satisfies the same ordering requirement.

The host drains native tracing every frame. The synchronous native transport also
drains immediately after engine creation, connect, submit, poll, and destruction.
This makes events produced by one native call visible before Unity handles that
call's result.

Each drained record is added to the managed log store and emitted through the Unity
API according to severity:

- trace, debug, and information use `Debug.Log`;
- warnings use `Debug.LogWarning`;
- errors use `Debug.LogError`.

Messages have a reserved `[Battlement/Rust]` prefix followed by their event name.
Managed Battlement records use `[Battlement/Managed]`. The prefixes make the source
clear in Unity logs and let Battlement's Unity log callback recognize records that
were already added directly to the store.

`Application.logMessageReceivedThreaded` adds all other Unity messages to the same
store. Its handler only copies immutable data into a locked queue and is safe when
Unity invokes it concurrently.

## Caught Rust panics

The Rust panic hook captures the panic location and backtrace in thread-local memory.
The native ABI's `catch_unwind` boundary formats that capture and returns it with the
`PANIC` status. It never depends on the tracing queue for the panic diagnostic.

The managed transport preserves the returned diagnostic. `BattlementRunner` reports
it as a native session failure, and the default `BattlementUnityLogger` emits the
plain diagnostic and Rust backtrace through `Debug.LogError`. The error reporter
also takes its recent-record snapshot from the unified store, so its focused JSON
report includes the Rust events leading up to the panic.

This design is for panics that unwind to Battlement's ABI boundary. A process abort,
double panic, deadlock, or native crash can prevent Unity from receiving the final
diagnostic.

## Managed store and viewer

The managed store assigns one arrival sequence to Rust, managed Battlement, and
ordinary Unity records. It retains the newest 2,048 records for the current process.
Timestamps from Rust are preserved; Unity-authored records receive a timestamp when
their callback is handled.

The in-game viewer snapshots this store when opened and while visible. It retains
source and severity filters, text search, timestamps, structured fields, exception
details, and stack traces. It does not perform file IO or parse a growing persistent
file.

## Failure behavior

- Failure to install or drain native tracing disables the bridge and emits one Unity
  error describing the failure.
- Invalid native records disable the bridge because they violate the fixed logging
  ABI.
- Queue overflow loses the oldest undrained tracing records but reports the loss.
- Unity logging remains usable if the in-game viewer is never created or opened.
- Focused error reports remain bounded independently of the current-run viewer store.
