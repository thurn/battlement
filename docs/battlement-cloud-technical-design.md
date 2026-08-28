# Battlement Unity Diagnostics technical design

Status: implemented

This document defines Battlement's Unity Diagnostics integration. It covers the
`battlement-cloud` and `battlement-cloud-fake` crates, the
`BattlementDiagnosticsModule` Unity asset, and the related C# protocol support.

Unity Diagnostics is one service supported by Battlement Cloud. The cloud crates
and module system are not Diagnostics-specific. Future Unity Cloud services get
their own Rust modules, Unity assemblies, module assets, and command contracts.

## Related information

- The [Battlement technical design](technical-design.md) defines sessions,
  commands, batches, failures, and the Unity-to-Rust boundary.
- The [Battlement logging design](logging-design.md) defines how Rust `tracing`
  events are forwarded through Unity logging.
- Unity's [Diagnostics overview][unity-diagnostics] describes crash, exception,
  ANR, and telemetry reports in the Unity Dashboard.
- Unity's [Editor configuration guide][unity-diagnostics-editor] describes
  project and build-profile collection settings.
- Unity's [custom-report guide][unity-custom-reports] documents custom metadata
  through `CrashReportHandler.SetUserMetadata`.
- Unity's [test-report guide][unity-test-reports] documents
  `Debug.LogException` as the supported non-crashing test path.
- Unity's [`CrashReportHandler` API][unity-crash-report-handler] defines
  exception capture, log buffering, and metadata limits.
- Unity's [symbol guide][unity-diagnostics-symbols] describes symbol upload and
  symbolication.
- Unity's [Data Subject Request guide][unity-data-subject-requests] describes
  the developer's data-access and deletion responsibilities.

[unity-diagnostics]:
  https://docs.unity.com/en-us/cloud/developer-data/diagnostics
[unity-diagnostics-editor]:
  https://docs.unity.com/en-us/cloud/developer-data/configure-diagnostics-editor
[unity-custom-reports]:
  https://docs.unity.com/en-us/cloud/developer-data/custom-reports
[unity-test-reports]:
  https://docs.unity.com/en-us/cloud/developer-data/test-reports
[unity-crash-report-handler]:
  https://docs.unity3d.com/6000.0/Documentation/ScriptReference/CrashReportHandler.CrashReportHandler.html
[unity-diagnostics-symbols]:
  https://docs.unity.com/en-us/cloud/developer-data/upload-symbol-files
[unity-data-subject-requests]:
  https://docs.unity.com/en-us/cloud/developer-data/data-subject-requests

## Goals

- Let a game attach bounded, game-specific metadata to future Unity Diagnostics
  reports.
- Let a Unity developer configure managed-exception capture and recent-log
  retention on the selected Diagnostics module asset.
- Put existing Rust `tracing` events in Unity's recent-log buffer without adding
  a second game-logging API.
- Turn eligible failures caught by Battlement into Unity exception logs.
- Preserve normal Battlement batching, validation, and failure behavior.
- Provide a deterministic fake for Rust rules tests.
- Keep Diagnostics isolated from future Battlement Cloud services.

## Non-goals

Battlement does not:

- configure a Unity Cloud project ID;
- enable Diagnostics collection for a project or build profile;
- query the Unity Dashboard;
- model upload, retry, grouping, retention, or symbolication;
- expose a Diagnostics log or breadcrumb API;
- cache, report, reconcile, or restore `CrashReportHandler` state;
- claim ownership of metadata keys;
- make several metadata writes atomic;
- provide an Editor-only QA window;
- add a dedicated sample for Battlement Cloud or Diagnostics.

## Unity owns collection and delivery

The Unity developer enables Diagnostics collection in project settings or on a
build profile. Selecting `BattlementDiagnosticsModule` does not enable collection
and is not a privacy or upload switch.

The module only calls runtime APIs already shipped with supported Unity players.
It does not depend on a Unity Cloud package or Unity Services initialization.

A successful Battlement command means the local `CrashReportHandler` call
returned. It does not prove that a report was created, uploaded, accepted,
grouped, or symbolicated. A report can appear later, and useful native stacks
require symbols from the exact build.

## Module selection

`BattlementRunner` has a serialized list of `BattlementModule` assets. A game opts
into the command surface by adding one `BattlementDiagnosticsModule` asset.

On connection, the runner publishes selected module identifiers in
`Connect.modules`. The Diagnostics identifier is:

```text
battlement.diagnostics
```

There is no aggregate `Connect.cloud` state. The Diagnostics adapter is write-only,
and the rules engine does not need a mirrored copy of Unity process globals.

If Rust sends a Diagnostics command without a selected module, the command fails
with `ModuleUnavailable` through the normal batch-failure path.

## Unity asset configuration

`BattlementDiagnosticsModule` exposes two serialized fields:

- `captureExceptions`, default `true`;
- `logBufferSize`, default `10`, constrained to `0..=50`.

The module applies both values when its runtime is prepared:

```csharp
CrashReportHandler.enableCaptureExceptions = captureExceptions;
CrashReportHandler.logBufferSize = logBufferSize;
```

A zero log-buffer size stops Unity from retaining recent logs for future reports.
It does not disable ordinary Unity logging.

These settings belong on the Unity asset because they are deployment policy, not
gameplay state. Rust does not configure them during a session.

The adapter does not read prior values and does not restore them on disposal.
Unity's API is process-global, so a project should select only one Diagnostics
module per runner and avoid unrelated systems repeatedly changing the same
settings.

## Rust metadata command

The public Rust API lives in `battlement_cloud::diagnostics`:

```rust
pub enum DiagnosticsCommand {
  SetMetadata(DiagnosticsMetadata),
}

pub struct DiagnosticsMetadata {
  pub key: String,
  pub value: Option<String>,
}
```

`Some(value)` sets or replaces one key. `None` clears one key. The names and
behavior intentionally follow Unity's `SetUserMetadata(key, value)` API.

Constructors validate values before they reach Unity:

```rust
let set = DiagnosticsCommand::SetMetadata(
  DiagnosticsMetadata::set("chess.game_status", "ongoing")?,
);

let clear = DiagnosticsCommand::SetMetadata(
  DiagnosticsMetadata::clear("chess.game_status")?,
);
```

The serialized wire shapes are:

```json
{"SetMetadata":{"key":"chess.game_status","value":"ongoing"}}
```

```json
{"SetMetadata":{"key":"chess.game_status"}}
```

One command changes one key. Games use ordinary Battlement batches when several
keys should change together. Battlement preserves group ordering, but Unity offers
no transaction for several metadata calls. An earlier successful call remains
applied if a later call fails.

## Metadata guidance

Unity currently permits up to 64 custom metadata entries, with keys up to 255
Unicode scalar values and values up to 1,024 scalar values. The global entry limit
includes metadata written outside Battlement, so only Unity can authoritatively
enforce it.

Battlement rejects:

- empty keys;
- keys with leading or trailing whitespace;
- keys containing control characters;
- keys longer than 255 Unicode scalar values;
- values containing NUL;
- values longer than 1,024 Unicode scalar values;
- malformed UTF-16 received by the C# client.

An empty string is a valid value and is distinct from clearing the key.

Use stable, low-cardinality metadata for context that should still be true when a
future report occurs. Good examples include:

- rules or content version;
- current scene or mode;
- opponent type;
- coarse game status;
- whether a session was resumed.

Do not put player names, email addresses, account identifiers, authentication
values, chat, save data, or arbitrary user text in metadata.

Metadata should not mirror a chronological event stream. A chess move belongs in
`tracing`; the current game status can be metadata. Avoid rewriting unchanged
metadata on every frame or move.

## Logging

There is no Diagnostics-specific Rust logging API. Rust game code uses `tracing`:

```rust
tracing::info!(from = %from, to = %to, "player moved piece");
```

Battlement's logging bridge forwards those events through Unity logging. Unity can
attach the most recent messages to a later report according to the module asset's
`logBufferSize`.

Unity does not provide the Diagnostics Dashboard as a general live log viewer.
Recent logs are visible there in the context of a collected report. During normal
development, use Battlement's existing log viewer, the Unity Console, or the player
log.

## Caught failures

At core safety boundaries, Battlement records eligible caught failures with
`Debug.LogException`. This lets Unity Diagnostics create an exception report when
collection and exception capture are enabled.

The bridge is part of Battlement core, not the optional Diagnostics module. It:

- preserves the normal Battlement failure response;
- logs one exception at the boundary that caught it;
- includes recent Rust and Unity logs through Unity's normal log buffer;
- does not expose a Rust command;
- does not crash the player deliberately.

Expected command-validation errors are not promoted into exception reports merely
because a caller supplied invalid data.

## C# execution

The Diagnostics assembly contains a thin `IDiagnosticsBackend` over three Unity
writes:

```csharp
bool CaptureExceptions { set; }
uint LogBufferSize { set; }
void SetMetadata(string key, string? value);
```

The runtime validates a metadata command, calls `SetUserMetadata`, and returns.
There is no readback, local state object, revision, state action, ownership ledger,
rollback, compensation, or teardown restoration.

Errors use the normal command pipeline:

- absent module: `ModuleUnavailable`;
- invalid key or value: `DiagnosticsMetadataInvalid`;
- Unity API exception: `DiagnosticsOperationFailed`.

No Diagnostics-specific failure is sent outside a normal `BatchFailed` message.

## Deterministic fake

`battlement-cloud-fake` stores only:

- whether the module is available;
- the current key-value map;
- attempted command outcomes for test assertions.

The general `FakeClient::connect` path selects no optional modules. Tests opt into
Diagnostics explicitly with `connect_with_diagnostics`; explicit connection
metadata keeps any unrelated module identifiers supplied by the caller.

The fake applies `Some(value)` as an insertion or replacement and `None` as a
removal. It performs the same Rust validation and returns the same stable errors
for invalid data or module absence.

It deliberately does not simulate:

- the module asset's Unity-only settings;
- metadata ownership or restoration;
- state reports or revisions;
- upload and Dashboard behavior;
- report creation, grouping, or symbolication.

## Chess sample

The chess sample selects `BattlementDiagnosticsModule` but leaves the Unity Cloud
project association disabled. Its asset enables managed-exception capture and
retains 20 recent log messages.

At session start, Rust batches stable metadata for:

- sample name;
- rules version;
- opponent type;
- game origin;
- coarse game status.

It updates game status when a game reaches a terminal state. Existing chess
`tracing` calls describe moves and other chronological behavior. The sample does
not duplicate those events as metadata.

This demonstrates local integration without claiming that reports can upload from
an unassociated sample project.

## Validation

Automated validation covers:

- Rust set and clear wire shapes;
- Rust and C# key/value bounds;
- unknown union variants;
- module-asset configuration writes;
- one-key set and clear execution;
- Unity API failures through the normal batch-failure path;
- missing-module behavior in the fake;
- chess metadata that remains unchanged across ordinary moves;
- caught-failure logging independent of the optional module.

Manual end-to-end Unity Dashboard testing requires a developer-owned temporary
Cloud project and is intentionally outside the repository's automated suite. A
developer performing that test should enable collection, create a development
build with symbols, generate a documented test exception, and verify metadata and
recent logs on the resulting occurrence. No project ID or credentials are checked
into the repository.
