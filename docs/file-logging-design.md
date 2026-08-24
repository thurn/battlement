# Battlement file logging

## Goal

Battlement writes Unity and Rust diagnostics to one ordered JSON Lines file in
development and release players. The file must contain the records leading up to a
Rust panic or managed crash and, whenever the platform still permits code to run, a
record describing the failure itself. An in-game Unity viewer reads the same stream.

This design does not replace operating-system crash reports or remote crash reporting.
A native Unity crash, power loss, browser termination, or storage failure can prevent
the final record or recent writes from becoming durable.

## One native-owned stream

Rust exclusively owns
`Application.persistentDataPath/Battlement/Logs/battlement.jsonl`. Unity never opens
the active file for writing. A small logging ABI, independent of an engine instance,
accepts Unity records and exposes incremental reads to the viewer:

- `battlement_log_initialize` opens or rotates the file and installs Rust logging.
- `battlement_log_write` accepts one UTF-8 Unity record.
- `battlement_log_read` returns complete records after a byte offset.
- `battlement_log_sync` requests durable platform storage.
- `battlement_log_close` completes the active session.

The ABI uses the existing native output-buffer ownership rules. Calls are thread-safe
because Unity's threaded log callback and Rust tracing events may arrive concurrently.
Initialization is idempotent for Unity Editor domain reloads, but a second incompatible
path is a developer error.

Both sources enter one append sink. While holding its lock, the sink assigns a global
sequence number and UTC Unix timestamp, serializes the complete record plus newline,
and writes it directly without a user-space buffered writer. A reader ignores an
incomplete final line, which may remain after an abrupt crash. Sequence is the
authoritative order; timestamp is for display and correlation with external logs.

Every record includes `schema`, `session_id`, `sequence`, `timestamp_unix_us`,
`source`, `severity`, `event_name`, `message`, and structured `fields`. Exception or
panic details are optional fields. Logs must not contain raw protocol payloads or
secrets by default. Rotation happens only during initialization, before the new active
file is opened, and retained files have fixed count and size limits.

## Rust integration

`battlement-native` installs a `tracing` subscriber during
`battlement_log_initialize`, before creating the rules engine. A custom
`tracing_subscriber` layer converts events and their current span context into the
shared record schema and writes through the append sink. Release builds keep the
subscriber; configuration controls the maximum level rather than compile-time removal.

The exported initialization function also installs the Rust panic hook. Ordinary
records are already passed to the operating system by the time `tracing` returns. The
panic hook uses a dedicated append handle and does not invoke `tracing`, acquire the
ordinary writer lock, or depend on engine state. It makes a best-effort minimal panic
record containing the payload, location, thread, and captured backtrace. This avoids a
deadlock when a panic originates while the ordinary logger is locked. After unwinding,
the existing native `catch_unwind` boundary writes a second structured outcome only
when needed and returns the panic diagnostic to Unity.

Direct writes preserve preceding records across a caught panic and ordinary process
crash. `battlement_log_sync` additionally calls the platform durability primitive;
doing that for every trace record would be too expensive. Every record is appended
immediately before its logging call returns; synchronization is a separate durability
checkpoint and never delays writes until a lifecycle event. Battlement periodically
syncs the active file and also syncs after an error or panic, when the application is
paused or loses focus on mobile, and during orderly shutdown. Sudden device power loss
may still lose records since the last successful sync.

## Unity integration and early startup

A runtime bootstrap marked `RuntimeInitializeOnLoadMethod(BeforeSplashScreen)` calls
`battlement_log_initialize` using `Application.persistentDataPath`. It runs before any
`BattlementRunner`, transport, or Rust engine is created. Native library initializers
must remain silent because no writable path is available before this call. Engine
creation fails as a developer error if logging initialization has not been attempted.

The bootstrap then installs an `IBattlementLogger` that forwards structured Battlement
records to `battlement_log_write`. It also subscribes to
`Application.logMessageReceivedThreaded` so Unity errors and exceptions outside
Battlement reach the file. Forwarding never calls `Debug.Log`, preventing callback
recursion. Best-effort `AppDomain.UnhandledException` handling writes a final managed
failure record directly; Unity's ordinary exception callback remains the primary path.

Unity startup messages emitted before `BeforeSplashScreen`, native-plugin load
failures, and failures of logging initialization cannot use the Rust sink. They remain
in Unity's player log, and the bootstrap uses the existing managed error-file sink when
`persistentDataPath` is writable. Once native logging fails, forwarding disables itself
rather than repeatedly throwing or recursively logging.

The in-game viewer calls `battlement_log_read` and advances only past complete lines.
It refreshes immediately after a returned Rust panic or Unity exception and can filter
by source, severity, event, or session without becoming the persistence mechanism.

## Platform behavior

### Desktop

Windows, macOS, and Linux write beneath `persistentDataPath`, never beside the
executable or inside the application bundle. Direct appends make preceding records
available after a process crash; periodic sync narrows the remaining power-loss window.
Desktop launch tooling may copy the file into retained run artifacts after exit.

### iOS

The file lives in the application's sandboxed persistent-data container and requires no
general filesystem permission. Battlement syncs on pause and focus loss because iOS may
suspend or terminate the application shortly afterward. The viewer reads through the
native ABI. Exporting the log uses the game application's share-sheet integration;
uninstalling the application removes its container.

### Android

The file lives in the application-specific persistent-data directory and requires no
broad external-storage permission. Battlement syncs on pause and focus loss because the
process may be killed while backgrounded. Viewing remains in-process; exporting uses an
Android content URI or share sheet rather than exposing the active path directly.

### WebGL

Rust and Unity share Unity's virtual filesystem, and the active file is immediately
readable by the in-game viewer. Durable persistence requires synchronizing that
filesystem to browser IndexedDB. The generated page enables
`autoSyncPersistentDataPath`, and Battlement also requests an explicit asynchronous
`FS.syncfs` on a periodic cadence, after a caught panic, and when the page is hidden.
Records become visible in the virtual file immediately; the periodic filesystem sync
makes those accumulated writes durable in IndexedDB.

The panic hook and caught-panic viewer work before IndexedDB synchronization completes.
A tab, browser, or WebAssembly failure can terminate execution before the asynchronous
sync finishes, so WebGL cannot guarantee durable local logs for a fatal crash. A
download action creates a browser Blob from a native snapshot. Applications requiring
stronger WebGL crash evidence must additionally stream records to a remote service.

## Failure guarantees

- A normal Rust event is appended before its `tracing` call returns.
- Records preceding a caught Rust panic remain readable, and the panic hook attempts a
  final independent record before unwinding.
- A managed exception normally reaches the threaded Unity callback; an unhandled
  exception hook provides a best-effort final record.
- A native Unity crash cannot safely call managed or Rust logging code. Previously
  appended records remain useful, while the crash itself belongs to the platform crash
  report.
- File-open, disk-full, permission, serialization, and sync failures fall back to Unity
  or native stderr and never recursively enter the file logger.
- No local design can guarantee the last record after power loss, forced termination,
  browser failure before IndexedDB sync, or corruption inside the logging code itself.

Validation covers ordered interleaved Unity and `tracing` records, panic during engine
work, panic while the normal writer is locked, managed exceptions, truncated final
lines, rotation, disk failures, mobile pause synchronization, WebGL caught-panic reads,
and IndexedDB synchronization failure.
