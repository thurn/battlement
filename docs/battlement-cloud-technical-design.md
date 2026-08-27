# Battlement Cloud Diagnostics technical design

Status: proposed implementation contract

This document is normative for `battlement-cloud`,
`battlement-cloud-fake`, `BattlementDiagnosticsModule`, and its C# support in
`com.battlement.client`. It defines the first Battlement Cloud capability:
Unity Diagnostics and Crash Reporting in Unity 6000.5.

**Unity Diagnostics** is Unity's engine-integrated collection and Dashboard
experience for crash reports, exception logs, telemetry, and supported Android
Application Not Responding reports. **Crash Reporting** is the runtime portion
that captures crashes, exceptions, recent logs, and custom metadata.
**Diagnostics collection** is the project- or build-profile setting that
allows Unity to collect diagnostic data. **Cloud state** is Battlement's
complete observable snapshot of the selected Diagnostics module. These terms
have these meanings throughout this document.

## Related information

- The [Battlement technical design](technical-design.md) defines sessions,
  snapshots, commands, errors, input gating, the generic `BattlementModule`
  composition model, and the Unity-to-Rust boundary.
- The [Battlement UI technical design](battlement-ui-technical-design.md)
  defines Rust-authored UI documents and deferred UI event handling.
- Unity's [Diagnostics overview][unity-diagnostics] defines the Dashboard
  experience and the reports available in Unity 6.2 and later.
- Unity's [Developer Data setup][unity-developer-data-setup] defines when
  diagnostics collection begins.
- Unity's [Editor configuration guide][unity-diagnostics-editor] defines the
  project and build-profile collection settings.
- Unity's [collection settings][unity-collection-settings] distinguish
  Essential Data from Additional Recommended Data.
- Unity's [user-consent guide][unity-user-consent] states that diagnostic data
  collection is project-level and does not use the Unity Consent API.
- Unity's [custom-report guide][unity-custom-reports] defines custom metadata
  through `CrashReportHandler.SetUserMetadata`.
- Unity's [test-report guide][unity-test-reports] defines
  `Debug.LogException` as the supported way to create a Diagnostics exception
  report without crashing the player.
- Unity's [`CrashReportHandler` API][unity-crash-report-handler] defines
  exception capture, recent-log buffering, and metadata access.
- Unity's [Diagnostics migration guide][unity-diagnostics-migration] explains
  that the built-in Unity 6.2+ Diagnostics experience replaces the deprecated
  Cloud Diagnostics crash and exception product.
- Unity's [symbol guide][unity-diagnostics-symbols] defines symbol upload and
  symbolication behavior.
- Unity's [Data Subject Request guide][unity-data-subject-requests] defines the
  developer's responsibilities for Diagnostics data access and deletion.

[unity-diagnostics]:
  https://docs.unity.com/en-us/cloud/developer-data/diagnostics
[unity-developer-data-setup]:
  https://docs.unity.com/en-us/cloud/developer-data/get-started
[unity-diagnostics-editor]:
  https://docs.unity.com/en-us/cloud/developer-data/configure-diagnostics-editor
[unity-collection-settings]:
  https://docs.unity.com/en-us/cloud/developer-data/collection-settings
[unity-user-consent]:
  https://docs.unity.com/en-us/cloud/developer-data/user-consent
[unity-custom-reports]:
  https://docs.unity.com/en-us/cloud/developer-data/custom-reports
[unity-test-reports]:
  https://docs.unity.com/en-us/cloud/developer-data/test-reports
[unity-crash-report-handler]:
  https://docs.unity3d.com/6000.0/Documentation/ScriptReference/CrashReportHandler.CrashReportHandler.html
[unity-diagnostics-migration]:
  https://docs.unity.com/en-us/cloud-diagnostics/migration
[unity-diagnostics-symbols]:
  https://docs.unity.com/en-us/cloud/developer-data/upload-symbol-files
[unity-data-subject-requests]:
  https://docs.unity.com/en-us/cloud/developer-data/data-subject-requests

## Summary

Battlement Cloud adds typed Rust commands for Unity Diagnostics while
preserving Battlement's thin-client model. Rust decides which diagnostic
context to attach, whether managed exceptions and recent logs are included,
and when to emit explicit diagnostic breadcrumbs. Independently, the core
Battlement error pipeline turns eligible failures that it catches at safety
boundaries, including Rust panics, into Unity exception logs. This automatic
bridge requires no Diagnostics module because the relevant reporting API is
built into Unity. Unity owns crash and ANR capture, report creation, transport,
retry, aggregation, retention, symbolication, and the Dashboard.

Cloud is optional to Battlement at runtime. The Rust and C# protocols always
recognize Cloud commands and state. A game opts into Battlement's Diagnostics
surface by adding one `BattlementDiagnosticsModule` asset to the serialized
module list on `BattlementRunner`. Only that selected runtime clone may change
Crash Reporting settings, attach Battlement-owned metadata, or write
Battlement diagnostic breadcrumbs. Without the module, `Connect.cloud` is
absent and Diagnostics commands fail with the core `ModuleUnavailable` error.
Caught Battlement failures are still logged as Unity exceptions because that
baseline integration belongs to core error reporting, not the optional Cloud
command surface.

Diagnostics collection itself is not a package SDK and is not activated by the
module. It is built into Unity 6.2 and later and is enabled or disabled in
Unity's project and build-profile settings. A selected module in a build whose
collection setting is disabled still exposes its deterministic local state and
accepts local configuration commands, but Unity uploads no diagnostic data. A
build whose collection setting is enabled may still collect engine-owned
diagnostics when the Battlement module is absent. Battlement never claims that
module selection is a privacy or upload switch.

`com.battlement.client` declares no Unity cloud package dependency.
`Battlement.Cloud.Diagnostics` references only APIs shipped with the supported
Unity Editor and player. Installing or initializing Unity Services Core is not
part of this design. The Diagnostics module is separate from
`BattlementCoreModule` because it controls an externally observable cloud data
surface that games must select deliberately, even though its runtime API ships
with the engine. The automatic caught-failure bridge lives in
`Battlement.Runtime`, not in that assembly, and exposes no Rust command or
Cloud state.

Diagnostics collection does not use `EndUserConsent` or any other Unity
Consent API. Battlement therefore exposes no Diagnostics consent state,
consent dialog, per-user opt-out command, privacy URL, or data-deletion command.
Games remain responsible for notices, legal bases, platform requirements, and
Data Subject Requests. Project- and build-level collection configuration is a
release decision outside a running Battlement session.

## Goals and non-goals

This design provides:

- explicit module selection and normal `ModuleUnavailable` behavior;
- typed configuration for managed-exception capture and recent-log buffering;
- bounded custom metadata set and clear operations;
- bounded diagnostic breadcrumbs with explicit severity;
- automatic core reporting for otherwise-caught Rust panics and C# exceptions,
  independent of module selection;
- complete state snapshots and correlated state actions;
- stable local validation and execution failures;
- a deterministic Rust fake and injectable Unity backend; and
- test and release guidance for real crash-report ingestion.

This design does not provide:

- a command that enables or disables project/build Diagnostics collection;
- a public command that deliberately crashes the player or fabricates an
  exception;
- an upload, flush, delivery, or ingestion acknowledgement;
- enumeration or download of reports from the Unity Dashboard;
- runtime symbol upload;
- user-report screenshots, attachments, or feedback forms;
- Analytics events, identities, sessions, funnels, or consent;
- Cloud Diagnostics' deprecated User Reporting SDK; or
- a promise that every platform captures every report kind.

## Battlement execution model

`BattlementRunner` is the one Unity host that owns a Battlement session. At
connection time Unity sends `Connect`, a platform snapshot, to the Rust rules
engine. Rust returns ordered batches of `Command` values. Each command has a
nonzero UUID `command_id`, one `CommandBody`, and a `blocking` Boolean that
defaults to true. A batch group finishes only when all of its blocking commands
finish; later groups may start after that point.

Unity sends an `Action` to Rust when platform or user state changes. Each
action has a session-unique UUID, the current session ID, and one `ActionBody`.
Rust returns any resulting commands synchronously. `InputSetEnabled(false)`
blocks gameplay and Rust-owned UI input actions, but it does not block system
state actions such as Cloud reports.

Unity APIs and the Rust transport execute on Unity's main thread. If an action
occurs inside a Unity callback where applying the Rust response would be
reentrant, the runner completes the callback first and then applies the
response from a FIFO deferred queue. This safe deferred-response path
preserves action and command order without calling Rust concurrently.

Diagnostics commands are synchronous. A successful command means the local
Unity API call completed and Battlement's observable state was updated. It
does not mean a crash report exists, was uploaded, was accepted by Unity, was
symbolicated, or is visible in the Dashboard.

## Ownership and dependency boundaries

`battlement-cloud` owns the public Rust domain values, Diagnostics command
union, builders, validation, and Serde forms for Cloud state, configuration,
metadata, breadcrumbs, reports, and failures. It has no native library and
does not communicate with Unity directly.

The core `battlement` crate depends on `battlement-cloud`. It embeds the Cloud
payload types directly in `Connect`, `CommandBody`, and `ActionBody`, and adds
the one `Command` convenience constructor described below. This direction
avoids a dependency cycle: `battlement-cloud` never depends on `battlement`.

`battlement-cloud-fake` depends on the Cloud value crate and the core
`battlement` crate, whose `CommandId` and `CoreErrorCode` types appear in its
public test API. This does not create a cycle because the core crate does not
depend on the fake. The existing `battlement-fake` crate composes that state
machine into `FakeClient`; `battlement-cloud-fake` does not depend on
`battlement-fake`.

The base `com.battlement.client` package always contains all Cloud protocol
records in `Battlement.Protocol`. `Battlement.Runtime` contains the generic
`BattlementModule` infrastructure and unavailable-module command path defined
by the main technical design. It also contains the automatic caught-failure
exception bridge because `Debug.LogException` is an engine API and because
faithful reporting must not depend on selecting an enrichment module. Neither
assembly references a Unity cloud SDK.

The package also contains `Battlement.Cloud.Diagnostics`. That assembly
references `Battlement.Protocol`, `Battlement.Runtime`, and
`UnityEngine.CoreModule`. It defines `BattlementDiagnosticsModule`, the
runtime clone, the production Crash Reporting adapter, and Editor validation.
It has no package version define, static startup hook, assembly scan, or global
registration hook. The package's minimum supported Unity version supplies the
required API.

### Diagnostics module

The only user-facing asset in this design extends the generic module base:

```csharp
[CreateAssetMenu]
public sealed class BattlementDiagnosticsModule : BattlementModule
{
    public override string ModuleId => "battlement.diagnostics";
}
```

The module asset is immutable configuration. Its per-session clone owns
Battlement's view of Diagnostics configuration, the keys written by the
session, action publication, failure state, and the injected backend.
`BattlementRunner` disposes it at session teardown. Merely importing the
Battlement package or creating the asset performs no Crash Reporting API call.

The runtime receives this injectable boundary:

```csharp
public interface IDiagnosticsBackend
{
    bool CaptureExceptions { get; set; }
    uint LogBufferSize { get; set; }
    string? GetMetadata(string key);
    void SetMetadata(string key, string? value);
    void WriteLog(DiagnosticsLogSeverity severity, string message);
}
```

The production backend is a thin adapter over
`CrashReportHandler.enableCaptureExceptions`,
`CrashReportHandler.logBufferSize`,
`CrashReportHandler.GetUserMetadata`,
`CrashReportHandler.SetUserMetadata`, and Unity's ordinary logging API. Backend
methods either return normally or throw. They execute synchronously on Unity's
main thread. Core error reporting uses its own injected exception-log boundary
described below; it is not part of `IDiagnosticsBackend`.

The module owns no background task, transport, timer, retry queue, consent
subscription, identifier, or persistent store. Unity's engine owns any
process-global report capture and transport. The clone never initializes UGS
and never probes the Dashboard over the network. Its lifecycle is unrelated to
the core error bridge; teardown only releases command registration and restores
configuration and metadata.

### Process-global API ownership

Crash Reporting configuration and metadata are process-global Unity APIs.
Battlement's existing single-active-runner guard is therefore a hard
requirement for this module. A second runner fails before module preparation
and cannot observe or mutate the first runner's Diagnostics state.

During preparation, the clone reads the current exception-capture flag and log
buffer size. It does not change them. It also registers its command executor,
connection-state provider, and action source. Initialization has no external
side effect and completes synchronously.

If an initial read throws or returns a value outside the supported domain, the
runner rejects connection as a module preparation failure. It does not omit
`Connect.cloud` and pretend the selected module is absent, and it does not
publish guessed configuration values. Later read failures use the recoverable
state-report behavior below because the clone then has a last known complete
snapshot.

For each metadata key first touched in an ownership epoch, the clone records
the backend's prior value. It then tracks the last value it wrote. An epoch
ends when a read observes that another system changed the key. If Battlement
later writes the key again, it begins a new epoch and snapshots that newer
external value as the value to restore. At teardown it restores the epoch's
prior value only when the backend still contains the clone's last written
value. If another system changed the key after Battlement, the clone leaves
that value untouched and logs only the key and collision category. This
compare-before-restore rule prevents Battlement from erasing a newer owner's
metadata.

The clone similarly records the initial exception-capture flag and log buffer
size. It restores each setting at teardown only if the current backend value
still equals the value last written by the clone. Restoration failures are
sanitized developer logs; teardown cannot emit actions into an ended session.

Metadata mutation has transactional Battlement semantics. Before applying a
batch, the clone reads every affected prior value and validates the complete
payload. If a backend call throws, it restores already-mutated keys in reverse
order. When compensation succeeds, the command fails without changing the
published state. If compensation throws or a readback differs, the command
fails with `DiagnosticsStateUncertain`, replaces the state from all readable
backend values, and emits an `OperationFailed` state action. Rust must treat
the resulting complete snapshot as authoritative.

## Activation and collection configuration

There are two independent activation decisions:

1. Unity project/build configuration determines whether the built player
   collects diagnostic data.
2. `BattlementRunner.modules` determines whether Rust may configure and enrich
   that Diagnostics stream through Battlement.

Core Battlement error reporting is not a third activation switch. It always
logs eligible caught failures through `Debug.LogException`. When Unity
collection or managed-exception capture is disabled, the local exception log
still occurs but Unity is not expected to upload it. When collection and
capture are enabled, Unity may ingest it without any Diagnostics module.

For new Unity 6.2+ projects, Essential Data collection is enabled by default
when the project is cloud-connected. Upgraded projects can require explicit
enablement. A build profile may override the project default. The Dashboard's
Developer Data collection and usage settings further control how Unity handles
collected data. Those controls are build/release configuration and are not
readable or mutable through the runtime Crash Reporting API used here.

Unity routes uploaded reports to the Unity Cloud project identified by the
project linkage embedded in the build. This design has no UGS environment or
runtime destination selector. Copying an entire linked Unity project can copy
that destination, while importing a package sample into another Unity project
uses the receiving project's linkage. Public samples must therefore ship
unlinked as specified below.

The module therefore reports `collection_configuration` as `External`, not as
enabled or disabled. `External` means the value is intentionally unknown to
the running Battlement client. Tests and game rules must not branch on an
invented local approximation.

The supported activation matrix is:

| Unity collection | Module selected | Battlement state | Caught failure | Engine collection |
|---|---|---|---|---|
| Disabled | No | Absent | Logged locally as an exception | Disabled for the build |
| Disabled | Yes | Present and locally usable | Logged locally as an exception | Disabled for the build |
| Enabled | No | Absent | Eligible for ingestion | May occur without Battlement enrichment |
| Enabled | Yes | Present and locally usable | Eligible for ingestion | May include Battlement enrichment |

No row implies delivery. Platform support, process termination, network
availability, report limits, Unity service behavior, and Dashboard retention
still apply.

## Protocol contract

All Cloud values use the existing JSON protocol: struct fields retain their
Rust `snake_case` names, unit enum variants are strings, and variants carrying
data use Serde's externally tagged single-property objects. Required fields
are always emitted. Optional fields are omitted when `None` and accepted as
either omitted or JSON `null`; writers always omit them.

Readers reject unknown enum variants and duplicate object properties, but
ignore unknown struct properties so a malformed optional addition cannot
change a known field. Lists preserve wire order. Object property order is not
significant. Rust produces minified UTF-8, and C# must deserialize the same
shapes without numeric coercion.

### Connection state

`Connect` has an optional `cloud: Option<CloudState>` field. Absence means the
runner did not select a valid `BattlementDiagnosticsModule`. Presence means
its runtime clone is active. The generic `Connect.modules` list contains
`battlement.diagnostics` in Inspector order.

Conceptually, the Rust state is:

```rust
pub struct CloudState {
    pub revision: u64,
    pub diagnostics: DiagnosticsState,
    pub failure: Option<CloudFailure>,
}

pub struct DiagnosticsState {
    pub collection_configuration: DiagnosticsCollectionConfiguration,
    pub capture_exceptions: bool,
    pub log_buffer_size: u8,
    pub metadata: Vec<DiagnosticsMetadataEntry>,
}

pub enum DiagnosticsCollectionConfiguration {
    External,
}

pub struct DiagnosticsMetadataEntry {
    pub key: String,
    pub value: String,
}

pub struct CloudFailure {
    pub operation: CloudOperation,
    pub kind: CloudFailureKind,
    pub message: String,
}
```

`metadata` contains only keys currently owned by the module clone. It does not
enumerate metadata written by other code, because Unity provides keyed lookup
but no supported complete enumeration contract. Entries are sorted by Unicode
scalar value of `key`, making state snapshots deterministic independent of
command order.

`log_buffer_size` is in `[0, 50]`. Zero disables recent-log inclusion in crash
reports. The default backend value is normally 10, but Battlement reports the
value it reads rather than hard-coding that assumption.

`revision` starts at zero for each runtime clone and increments after every
published state mutation. A new Battlement session creates a new clone and
resets the revision. Commands whose requested value already matches the
backend do not increment the revision. `ReportState` emits the current revision
when reconciliation finds no drift and increments it only when observed state
or failure state changes.

`CloudFailure.operation` is `Configuration`, `Metadata`, `Breadcrumb`, or
`StateRead`. `CloudFailure.kind` is `InvalidBackendValue`, `BackendRejected`,
`StateUncertain`, or `Unknown`. The sanitized message is limited to 512
Unicode scalar values, replaces control characters with spaces, and contains
no stack trace, metadata value, breadcrumb text, player identifier, token,
path, or serialized Cloud message.

`failure` is the most recent unresolved Diagnostics operation failure. A
newer failure replaces an older one. A successful operation clears `failure`
only when its operation matches. Local validation failures do not change state
and do not create `CloudFailure`; they are ordinary command failures.

### Commands and Rust conveniences

`CommandBody` gains exactly one Diagnostics variant:

```rust
pub enum CommandBody {
    // Existing variants remain here.
    Diagnostics(DiagnosticsCommand),
}

pub enum DiagnosticsCommand {
    Configure(DiagnosticsConfiguration),
    SetMetadata(SetDiagnosticsMetadataPayload),
    ClearMetadata(ClearDiagnosticsMetadataPayload),
    WriteBreadcrumb(WriteDiagnosticsBreadcrumbPayload),
    ReportState,
}

pub struct DiagnosticsConfiguration {
    pub capture_exceptions: bool,
    pub log_buffer_size: u8,
}

pub struct SetDiagnosticsMetadataPayload {
    pub entries: Vec<DiagnosticsMetadataEntry>,
}

pub struct ClearDiagnosticsMetadataPayload {
    pub keys: Vec<String>,
}

pub struct WriteDiagnosticsBreadcrumbPayload {
    pub severity: DiagnosticsLogSeverity,
    pub message: String,
}

pub enum DiagnosticsLogSeverity {
    Info,
    Warning,
    Error,
}
```

`DiagnosticsCommand` is the complete internal union for Diagnostics flows.
Future Diagnostics operations extend it rather than adding another
`CommandBody` variant. A different Battlement Cloud service receives its own
outer variant and its own module. The generic executor routes
`CommandBody::Diagnostics` once to the selected Diagnostics module. If the
module is unavailable, dispatch fails before inspecting the inner operation.

The unit `ReportState` variant serializes as a JSON string. Data variants use
Serde's external tags; for example a configuration command body is
`{"Diagnostics":{"Configure":{"capture_exceptions":true,"log_buffer_size":10}}}`.

The core `Command` type offers one constructor:

```rust
pub fn diagnostics(command: DiagnosticsCommand) -> Self;
```

The constructor generates a `CommandId` in the same way as existing
Battlement helpers and accepts any Diagnostics command. Callers that need a
predetermined ID construct `Command` directly. Every Diagnostics command
created by the helper is blocking. A Diagnostics body with `blocking: false`
fails ordinary command validation before executor dispatch.

Blocking completion is local and specific: configuration completes after
set-and-readback; metadata completes after every mutation and readback;
breadcrumb writing completes after Unity's log call returns; report completes
after its state action is safely enqueued. None waits for report creation or
network delivery.

### State actions

`ActionBody` gains the system-level variant
`CloudState(CloudStateReport)`:

```rust
pub struct CloudStateReport {
    pub state: CloudState,
    pub cause: CloudStateCause,
    pub originating_command_id: Option<CommandId>,
}

pub enum CloudStateCause {
    ReportRequested,
    ConfigurationChanged,
    MetadataChanged,
    ExternalStateObserved,
    OperationFailed,
    OperationRecovered,
}
```

A report caused directly by a command includes that command's ID.
`ReportState` re-reads both configuration values and every currently owned
metadata key. With no drift, it sends an equal-revision `ReportRequested`
snapshot. When a read observes a value not last written by Battlement, the
clone ends that ownership epoch, updates its state, increments revision, and
sends `ExternalStateObserved`. A metadata key whose value was externally
changed or removed is no longer clone-owned and is removed from
`CloudState.metadata`. A configuration or metadata command that makes no
change completes without an unsolicited report. Breadcrumbs do not mutate
state and do not emit a state action unless their backend call fails or
recovers the matching prior failure.

If `ReportState` cannot read a complete snapshot, the command fails with
`DiagnosticsOperationFailed`, retains the last known values, records a
`StateRead` failure, increments revision, and emits `OperationFailed`. A later
successful report clears that failure. It emits `OperationRecovered` when the
reconciled values match the retained snapshot, or `ExternalStateObserved` when
they do not.

Cloud state actions are system actions, not gameplay input. They bypass the
`InputSetEnabled(false)` gameplay gate. They still use the runner's safe
deferred-response path. The action is a complete snapshot, not a patch. Rust
replaces its remembered Cloud state only when the report revision is greater
than the revision it already holds, except that a correlated
`ReportRequested` action with an equal revision is still delivered to the
requesting rule but does not replace state.

The clone does not retain historical reports. `Connect.cloud` contains its
newest state and revision. A report created after the action source registers
but before connection completes waits in the ordinary deferred queue. After
connection, the runner discards queued reports with lower revisions and
submits higher revisions in order. An equal-revision `ReportRequested` report
is retained because its command correlation is observable.

## Validation and stable failures

Rust builders validate before serialization. C# validates again before
calling the backend. Invalid data is a stable command failure and never a
backend exception contract.

Metadata keys must:

- contain 1 to 255 Unicode scalar values;
- have no leading or trailing Unicode whitespace;
- contain no control, surrogate, or null characters; and
- be unique within one payload.

Metadata values may contain 0 to 1,024 Unicode scalar values. They may not
contain null characters or unpaired surrogates. Empty values are valid and are
different from clearing a key. `SetMetadata` contains 1 to 64 entries.
`ClearMetadata` contains 1 to 64 unique keys. The protocol does not reserve a
prefix, but package code uses `battlement.` for its own standard keys.

Unity limits process-global metadata to 64 key-value pairs. Battlement cannot
enumerate keys owned by other systems and therefore cannot prove remaining
capacity before a new-key write. A backend rejection of an otherwise valid
payload fails with `DiagnosticsMetadataLimit` when Unity reports its documented
capacity error, or `DiagnosticsOperationFailed` when the cause is not
distinguishable. Transactional compensation follows the ownership rules above.

Breadcrumb messages contain 1 to 1,024 Unicode scalar values after replacing
line breaks and control characters with spaces and collapsing runs of
whitespace. The normalized message may not be empty. Battlement prepends the
fixed text `Battlement diagnostic: ` and maps severity to Unity's ordinary
info, warning, or error logger. The breadcrumb path never uses
`Debug.LogException`, because a gameplay rule must not fabricate an exception
report. The separate internal error bridge is the only path in this design
that calls `Debug.LogException`.

`log_buffer_size` is an integer in `[0, 50]`. No numeric coercion is accepted.
Configuration is applied capture flag first, then buffer size, and verified by
readback. If the second write fails, the first is restored before the command
returns.

Stable core error codes are:

| Error code | Meaning |
|---|---|
| `ModuleUnavailable` | No selected Diagnostics module owns the command. |
| `DiagnosticsConfigurationInvalid` | Configuration is outside the protocol domain. |
| `DiagnosticsMetadataInvalid` | A key, value, duplicate, or batch is invalid. |
| `DiagnosticsMetadataLimit` | Unity rejected a new metadata key at its global limit. |
| `DiagnosticsBreadcrumbInvalid` | Breadcrumb severity or normalized text is invalid. |
| `DiagnosticsOperationFailed` | A backend call failed without uncertain state. |
| `DiagnosticsStateUncertain` | Mutation compensation or authoritative readback failed. |

Every failed command retains its command ID and follows the normal Battlement
batch-failure contract. No failure implicitly retries, changes collection
configuration, or emits a test crash.

## Configuration behavior

`DiagnosticsCommand::Configure` controls two runtime Crash Reporting settings:

- `capture_exceptions` controls capture of managed exceptions by
  `CrashReportHandler`; and
- `log_buffer_size` controls how many recent Unity log messages, from zero to
  fifty, Unity retains for inclusion with a report.

These settings do not enable native crash capture, Diagnostics collection, the
Dashboard product, or symbol upload. They do not affect logs already attached
to a report. Their exact platform behavior remains Unity-owned.

Configure validates the complete payload, snapshots both prior values, writes
both, and reads both back. A normal return with mismatched readback is
`DiagnosticsStateUncertain`. A successful change increments revision once,
clears a matching configuration failure, and emits one correlated
`ConfigurationChanged` snapshot. A no-op configuration command succeeds
without incrementing revision or emitting a change action.

Battlement does not persist configuration. A new runner clone reads the current
process-global values. Player restarts receive the values built into and
selected by Unity, unless game code changes them before Battlement prepares.

## Metadata behavior

Custom metadata enriches future issues. It is context, not an event stream.
Replacing a key changes the value available to later reports and does not
retroactively update an already captured report.

`SetMetadata` applies entries in payload order after complete validation. State
stores them in sorted order. Setting an owned key to its current value is a
no-op for that key. A successful batch that changes at least one key increments
revision once and emits one correlated `MetadataChanged` report.

`ClearMetadata` passes `null` to Unity for each key. Clearing a key owned by
the current clone removes it from Cloud state. Clearing a valid key that the
clone does not own is rejected with `DiagnosticsMetadataInvalid`; Battlement
must not erase another system's metadata through an untracked command.

The following package-owned keys are recommended and may be set by ordinary
commands rather than automatically:

- `battlement.session_id` for the current Battlement session UUID;
- `battlement.rules_version` for a non-secret rules build identifier;
- `battlement.scene` for a stable content identifier; and
- `battlement.state` for a short, non-user-authored game-state label.

No key or value may contain a player name, email, account identifier, IP
address, authentication value, chat content, save-game contents, arbitrary UI
text, or other unnecessary personal or sensitive data. Validation can enforce
shape and bounds but cannot infer meaning; the game developer owns that
decision.

## Breadcrumb behavior

Breadcrumbs are ordinary Unity log entries intended to appear in the recent
log buffer attached to a later issue. They do not create a Diagnostics event
of their own and can be dropped by Unity. When `log_buffer_size` is zero, a
breadcrumb is still written to the Unity log but is not expected to accompany
a report.

Rust chooses severity explicitly. Error breadcrumbs use `Debug.LogError`, not
an exception object. The command succeeds when the local logging call returns.
There is no receipt, sequence number, remote status, or retry.

Games should record sparse state transitions and invariant failures, not every
frame, pointer movement, card property, or command payload. The module does not
rate-limit valid commands because deterministic command outcomes are more
important than a time-dependent policy. Samples and documentation use no more
than one breadcrumb per meaningful state transition.

## Battlement error integration

Battlement catches failures at boundaries where unwinding would otherwise
cross the Rust ABI, corrupt a session, or bypass the player-safe failure UI.
Catch-and-present remains the authoritative recovery behavior. Core Battlement
adds one Unity exception log after capture; this never changes whether the
session continues, stops, or requires a player restart and does not require a
Diagnostics module.

The generic error pipeline distinguishes three reporting dispositions:

```csharp
internal enum BattlementErrorReportingDisposition
{
    Ignore,
    AlreadyLoggedByUnity,
    ReportCaughtFailure,
}

internal interface IBattlementCaughtFailureReporter
{
    void Report(BattlementError error);
}
```

`BattlementErrorReporter` owns one injected
`IBattlementCaughtFailureReporter`. Production construction always supplies the
Unity implementation; tests supply a recorder or throwing fake. There is no
process-static service locator, module lookup, or Diagnostics-specific branch
in `BattlementRunner`. The reporter first constructs the complete
`BattlementError`, writes the ordinary `BattlementLogRecord`, and invokes the
configured `IBattlementErrorSink`. For `ReportCaughtFailure`, it then calls the
caught-failure reporter before showing terminal failure UI or disposing module
clones. A reporter exception is caught and written as the sanitized internal
warning `battlement.error.exception_report_failed`; it cannot replace the
original error or alter recovery.

The producer of an error supplies its disposition:

- A Rust panic returned as native transport status `PANIC` is
  `ReportCaughtFailure`. Rust already caught the unwind and returned its
  formatted message and native backtrace, so Unity did not observe a managed
  exception.
- A C# exception caught by a Battlement session, command, snapshot, transport,
  or host safety boundary is `ReportCaughtFailure` when it terminates the
  operation or session and has not previously been logged as a Unity
  exception.
- A `LogType.Exception` received through
  `Application.logMessageReceivedThreaded` is `AlreadyLoggedByUnity`. Unity
  Diagnostics has already observed the original entry, so Battlement records
  and presents it without creating a duplicate exception report.
- A `LogType.Assert` received through the same callback is
  `Ignore`. Battlement retains its existing local assertion handling, but the
  bridge does not turn a nonexception assertion log into a synthetic exception
  occurrence.
- `AppDomain.UnhandledException` and
  `TaskScheduler.UnobservedTaskException` are `ReportCaughtFailure` only when
  Battlement receives them without an earlier Unity exception-log callback.
  The shared Unity-error capture path correlates the exception object and
  normalized condition to avoid double reporting when both sources fire.
- Expected command failures, invalid protocol input, transport errors without
  an exception, user-visible validation errors, and warnings are `Ignore`.

The caught-failure reporter acts only for `ReportCaughtFailure`. It does not
inspect module selection, `CloudState`, project linkage, collection settings,
or `CrashReportHandler.enableCaptureExceptions`; those values do not decide
whether an error deserves a faithful Unity exception log. Project/build
collection and the engine's current capture setting remain external: the log
can appear in the Unity console without being uploaded.

For each eligible error, the caught-failure reporter constructs one internal
`BattlementCaughtFailureException`. This is a reporting envelope, not the
exception that crossed the original boundary:

```csharp
internal sealed class BattlementCaughtFailureException : Exception
{
    private readonly string originalStackTrace;

    public BattlementCaughtFailureException(BattlementError error)
        : base(DiagnosticMessage(error)) =>
        originalStackTrace = DiagnosticStackTrace(error);

    public override string StackTrace => originalStackTrace;
    public override string ToString() => $"{GetType().FullName}: {Message}\n{StackTrace}";
}
```

The envelope has a stable type and a sanitized message containing the
`BattlementError.EventName`, source, and failure type. It deliberately omits
the unique error ID from the exception message and type so occurrences of the
same defect can group together. Its `StackTrace` and `ToString` preserve the
normalized original stack trace, including Rust frames for a native panic,
instead of substituting the managed reporting call site. The immediately
preceding ordinary Battlement error log contains `error_id`, so a Diagnostics
occurrence whose recent-log buffer is nonzero can be correlated with the
complete bounded local JSON error report.

The envelope message is limited to 512 Unicode scalar values. Its stack trace
is limited to 32,768 Unicode scalar values after removing ANSI escapes, nulls,
and control characters other than line breaks and tabs. Truncation preserves
the start and end with an explicit omitted-character marker. It never includes
serialized protocol payloads, metadata values, authentication values, or
recent-record contents. Rust panic messages and caught exception messages are
sanitized separately from their stack traces before inclusion.

The reporter calls its injected exception-log boundary exactly once per
eligible `BattlementError.Id`. The production implementation calls
`Debug.LogException`. Unity's documented contract treats that call as a
Diagnostics test-report source, so it can create an exception issue without
terminating the player. A normal return means only that Unity accepted the
local exception log; it does not acknowledge upload or Dashboard ingestion.

`Debug.LogException` itself raises Unity's log callback. To prevent a feedback
loop, `BattlementUnityErrors` recognizes the internal
`BattlementCaughtFailureException` type marker and does not enqueue that one
callback into `BattlementErrorReporter`. This suppression affects only
Battlement's recapture path. It does not suppress Unity's console entry or
Diagnostics processing, and ordinary game exceptions with the same message
remain unaffected because they do not have the internal envelope marker.

The bridge does not read or mutate `CloudState`, metadata, or revision.
Correlation uses the preceding bounded log rather than temporary global
metadata, because the runner can dispose immediately after a terminal error
and Unity does not promise when report metadata is snapshotted. A thrown report
call or failed report-envelope construction produces only
`battlement.error.exception_report_failed` with the error ID and stable
failure category. The original local error is already durable, and the failing
session may not safely emit another Cloud action.

## Crash and report lifecycle

Unity detects and captures supported native crashes, managed exceptions, and
Android ANRs according to the built player, platform, and Unity configuration.
Battlement is not in the crash-time call path. This is required because Rust,
managed C#, and the Unity main thread may all be unavailable during a native
failure.

The last successfully applied metadata values and the recent Unity log buffer
are candidates for attachment to a report. Unity owns snapshot timing and may
omit context when the process terminates abruptly. Battlement never performs a
last-second callback or synchronous network operation during a crash.

The module exposes no `Flush` command because Unity's built-in Diagnostics API
does not provide a supported flush or upload acknowledgement. It exposes no
`RecordException` command because application logic should throw or log real
exceptions through its normal error boundary. Core Battlement reports failures
that those boundaries necessarily catch. Test
tooling may trigger a known exception or native crash outside the protocol.

Reports can arrive after a later application launch, can be grouped with
other occurrences, and can require uploaded symbols for useful stack traces.
These are operational facts, not Battlement state transitions. Cloud state
does not contain report counts, last-report timestamps, problem IDs, or
Dashboard URLs.

## Privacy and data governance

Diagnostics data collection is managed at the project/build level and does not
use the Unity Consent API. `AnalyticsIntent`, `AdsIntent`, and any Analytics
privacy URL are unrelated to this module. Battlement must not display a consent
dialog whose result falsely claims to control Diagnostics collection.

The developer is responsible for:

- configuring collection and usage in Unity's Editor and Dashboard;
- satisfying platform certification and disclosure requirements;
- determining whether a separate notice, permission, or opt-out is legally
  required;
- minimizing metadata, log, panic-message, and exception-message contents;
- configuring retention and access controls; and
- responding to Data Subject Requests through Unity's supported process.

There is no per-user deletion command in the supported runtime API. A game that
needs an account-level privacy workflow implements it outside Battlement Cloud
and follows Unity's current Data Subject Request documentation. The absence of
a runtime command must remain visible in product and legal review.

## Fake client contract

`battlement-cloud-fake` supplies `CloudFake`, an in-memory implementation of
the complete contract. Its default state has exception capture enabled, log
buffer size 10, no metadata, no failure, and revision zero. Tests can construct
it with other valid local settings or as absent.

`battlement-fake::FakeClient` contains a configured `CloudFake`. An absent fake
omits `Connect.cloud` and returns `ModuleUnavailable`. A present fake executes
`CommandBody::Diagnostics` through the same validation and ownership rules as
Unity.

The public test surface is:

```rust
impl CloudFake {
    pub fn absent() -> Self;
    pub fn configured(configuration: DiagnosticsConfiguration) -> Self;
    pub fn state(&self) -> Option<&CloudState>;
    pub fn command_results(&self) -> &[FakeCloudCommandResult];
    pub fn breadcrumbs(&self) -> &[DiagnosticsBreadcrumb];
    pub fn state_reports(&self) -> &[CloudStateReport];
    pub fn fail_next_operation(&mut self, failure: CloudFailure);
    pub fn make_next_state_uncertain(&mut self);
}

pub struct DiagnosticsBreadcrumb {
    pub severity: DiagnosticsLogSeverity,
    pub message: String,
}

pub struct FakeCloudCommandResult {
    pub command_id: CommandId,
    pub outcome: FakeCloudCommandOutcome,
}

pub enum FakeCloudCommandOutcome {
    Completed,
    Failed(CoreErrorCode),
}
```

`Default` constructs the documented present fake. `FakeClient` takes ownership
through `connect_with_cloud(engine, cloud)` and exposes `cloud()` and
`cloud_mut()`. Its `reconnect(engine)` method retains fake backend settings and
journals while replacing the Battlement session and module clone. Clone-owned
metadata is restored on disconnect and therefore starts empty after reconnect
unless the test models an external writer.

The fake tracks:

- exception-capture and log-buffer configuration;
- clone-owned metadata and simulated external prior values;
- normalized breadcrumbs in command order;
- every command result with its command ID;
- state revisions, failures, reports, causes, and command correlation; and
- compensation and compare-before-restore behavior.

`fail_next_operation` applies to exactly one backend operation and then resets
to success. `make_next_state_uncertain` causes the next mutating command's
compensation or readback to fail and then resets. Calling either gesture in a
state that cannot consume it is allowed; the next eligible operation consumes
it. Test-developer errors, such as constructing an invalid `CloudState`
directly through an unchecked internal helper, panic rather than return a
recoverable result.

The breadcrumb journal contains only normalized messages that reached the fake
backend. Failed and invalid commands add no entry. The fake never creates or
uploads a crash report and never pretends that a breadcrumb is remotely
visible. The Rust fake does not model Unity-host `BattlementError` observation;
that behavior has no Rust protocol surface and is covered by Unity tests with
an injected Diagnostics backend.

## Logging and diagnostics

Production Battlement logs may contain the command kind, command ID, metadata
key, configured numeric values, revision, stable failure category, and whether
compensation succeeded. They must not contain metadata values, breadcrumb
messages, stack traces returned by a backend exception, serialized Cloud
messages, report contents, authentication values, or player identifiers.

The phrase “diagnostic log” can refer either to Battlement's own developer log
or a deliberate `WriteBreadcrumb` command. Implementations keep those paths
separate. An internal Battlement failure never automatically becomes a player
breadcrumb, and a player breadcrumb is never echoed into the internal log.
Eligible caught failures can separately become exception reports through the
core error bridge. That bridge reuses the already-created local error rather than
turning arbitrary internal log records into Diagnostics issues.

Unity's Dashboard, integrations, and notification rules are operational tools
outside the runtime protocol. Battlement neither enables them nor stores their
credentials.

## Automated validation

Rust black-box tests cover protocol round trips for every state, command,
action cause, severity, and stable failure. Property tests cover metadata key
and value boundaries, duplicate keys, empty values, normalized breadcrumbs,
control characters, collection limits, log-buffer boundaries, deterministic
metadata ordering, and unknown optional fields.

`battlement-cloud-fake` tests cover absent-module failures, configuration
changes and no-ops, metadata set and clear, foreign-key protection,
transactional compensation, uncertain state, breadcrumb normalization, state
report correlation, external-state reconciliation, ownership epochs, failure
replacement and recovery, reconnect behavior, and teardown restoration.
`battlement-fake` integration tests prove the composed fake drives a real Rust
engine through public APIs.

C# protocol tests deserialize Rust fixtures and serialize equivalent C# values
for all Cloud variants. They reject invalid keys, lengths, severities, buffer
sizes, duplicate entries, missing required fields, duplicate object
properties, and unknown union variants before calling the backend.

Unity black-box executor tests inject `IDiagnosticsBackend`. They cover:

- no API call before module preparation;
- initial setting reads and immediate connection state;
- exact set order and readback for configuration;
- metadata mutation order and deterministic published ordering;
- rollback after each possible partial write;
- mismatched readback and compensation failure;
- compare-before-restore for metadata and settings;
- exact Unity logger selection for each severity;
- state action ordering and deferred-response safety;
- stale callbacks and action suppression after clone disposal; and
- the absence of metadata values and breadcrumb text in internal logs.

Core Unity error-pipeline tests inject `IBattlementCaughtFailureReporter`. They
cover every disposition branch, including native panics, caught C# failures,
ignored assertions, already-logged Unity exceptions, and ignored ordinary
failures. They also cover:

- one report call containing the normalized original Rust or C# stack and no
  unique error ID in the grouping message;
- identical behavior with the Diagnostics module present or absent;
- report invocation even when the current engine capture flag is false;
- suppression of the bridge envelope from Battlement's Unity-log recapture
  without suppressing Unity's exception log;
- report failure isolation and exactly-once invocation per error ID; and
- unchanged local persistence, stack-trace UI, and recovery behavior.

Activation tests exercise these distinct configurations:

- without the Diagnostics module selected, `Connect.cloud` is absent and all
  Diagnostics commands fail with `ModuleUnavailable`, while caught Battlement
  failures still reach the core exception reporter;
- with the module selected, the runner creates exactly one clone and reads the
  engine settings without changing them; and
- enabling or disabling Unity's project/build collection setting never changes
  protocol shape and is never reported as runtime state.

Play Mode tests with Domain Reload disabled prove serialized module selection
does not retain runtime state or mutate the ScriptableObject asset. Separate
tests create two runners and require the second to fail before preparation.
Lifecycle tests disable and re-enable one runner and require a new session,
fresh clone, restored process-global settings, cleared owned metadata, and
exact action-source release.

Assembly tests prove the base package declares no Analytics, Services Core, or
deprecated Cloud Diagnostics package dependency. They compile both the core
exception bridge and Diagnostics assembly against the minimum supported Unity
version and verify serialized module references preserve their concrete type
under Mono and IL2CPP managed stripping.

No automated test deliberately crashes the ordinary Editor test process.
Native and managed crash ingestion use isolated release-validation players as
described below.

## Cloud sample

A separate Cloud sample contains one Unity project and one Rust rules crate. Its
distributed project is unlinked from Unity Cloud and has Diagnostics collection
disabled. It contains no Battlement-owned cloud Project ID. Setup instructions
require the developer to link a project they control, visibly verify its
organization, name, and Project ID, and explicitly enable collection for the
active build profile. Importing the package sample into an existing game keeps
that game's own project linkage; it never supplies Battlement's QA linkage.

Internal ingestion QA uses an undistributed copy linked to a non-production
Battlement Unity project. That binding is local release infrastructure and is
never committed to the public sample. The sample creates one
`BattlementDiagnosticsModule` asset and assigns it to
`BattlementRunner.modules`; this enables enrichment commands, not caught-error
reporting.

The sample demonstrates:

- initial and requested Cloud state reports;
- enabling and disabling managed-exception capture;
- recent-log buffer sizes of zero, ten, and fifty;
- setting, replacing, and clearing custom metadata;
- sparse info, warning, and error breadcrumbs;
- automatic exception reports for a caught Rust panic and caught fatal C#
  exception, including their original stacks, both with and without the module;
- deduplication of a Unity exception that Diagnostics already observed;
- absent-module behavior; and
- injected backend rejection, compensation, and recovery.

An Editor-only Cloud QA window belongs to the package. It displays Unity's
project-level collection setting as build configuration, never as runtime
Cloud state. It can select injected backends and configure exactly one backend
operation to fail or return mismatched readback. It also provides buttons that
launch isolated test-player flows for a caught Rust panic, caught C# exception,
ordinary Unity exception, and native crash. Those buttons are QA tooling, not
protocol commands and not compiled into a release player.

The sample documentation warns that exception and native-crash tests can
terminate the player, reports may take time to ingest, project quotas and
retention apply, and symbols must match the exact build.

## Manual QA

Use the separate Cloud sample and a non-production Unity project. Use a unique
build version for each report-ingestion pass. Upload matching symbols before
judging native stack traces. Record delayed ingestion as delayed rather than as
an immediate protocol failure.

1. Confirm the distributed sample starts unlinked with Diagnostics collection
   disabled. Link it to the non-production QA project, verify the displayed
   organization, project name, and Project ID, enable collection for the active
   build profile, and confirm its runner lists one
   `BattlementDiagnosticsModule`. Enter Play Mode. Confirm
   `Connect.modules` reports `battlement.diagnostics`, `Connect.cloud` is
   present at revision zero, and its settings match the values read from
   `CrashReportHandler`.
2. Send `ReportState`. Confirm one equal-revision `ReportRequested` action
   contains the requesting command ID and no backend mutation occurs.
3. Configure exception capture off and log buffer size zero. Confirm one
   `ConfigurationChanged` action, exact readback, and no claim that project
   collection was disabled. Repeat the same command and confirm it succeeds
   without a revision change or unsolicited action.
4. Configure exception capture on and log buffer size ten. Write info, warning,
   and error breadcrumbs. Confirm each normalized line uses the correct Unity
   logger and no breadcrumb creates an exception object.
5. Set the recommended `battlement.session_id`, `battlement.rules_version`,
   `battlement.scene`, and `battlement.state` metadata. Replace the scene and
   state values in one batch. Confirm one revision per changing batch, sorted
   state entries, and exact values from `CrashReportHandler.GetUserMetadata`.
6. Clear one owned key and confirm Unity returns no value. Attempt to clear a
   valid key not owned by the clone and confirm
   `DiagnosticsMetadataInvalid` without a backend write.
7. In the injected harness, fail the second write of a multi-key batch.
   Confirm the first write is restored, the command fails with
   `DiagnosticsOperationFailed`, and published metadata is unchanged. Repeat
   with failed compensation and confirm `DiagnosticsStateUncertain` plus an
   authoritative `OperationFailed` snapshot.
8. Before entering Play Mode, have separate sample code set a metadata key.
   Let Battlement overwrite it, then stop Play Mode. Confirm Battlement restores
   the prior value. Repeat while external sample code overwrites Battlement's
   value. Send `ReportState`, confirm `ExternalStateObserved` removes the key
   from clone-owned state, and confirm teardown preserves the external value.
9. Build an isolated development player with exception capture enabled, ten
   buffered logs, and unique metadata. Trigger the QA harness's real managed
   exception. Confirm the Diagnostics Dashboard later shows the exception,
   matching build version, metadata, recent breadcrumbs, and symbolicated
   managed frames. Treat the command-side setup as locally complete even while
   ingestion is pending.
10. In the injected core error harness, return a native `PANIC` result with a
    known Rust backtrace. Confirm `BattlementError` saves and presents the
    original failure, then confirm the caught-failure reporter receives exactly
    one exception envelope whose stack contains the known Rust frames.
    Confirm the envelope message omits the unique error ID and its own Unity
    log callback does not produce a second `BattlementError`.
11. Repeat the caught Rust panic in an isolated player linked to the
    non-production project. Confirm one exception occurrence reaches the
    Diagnostics Dashboard without crashing the player, its occurrence details
    retain the Rust frames, and its recent logs contain the local `error_id`
    needed to open the matching bounded JSON report.
12. Trigger one C# exception caught by a Battlement safety boundary, then one
    ordinary `Debug.LogException`. Confirm the caught exception produces one
    bridged Diagnostics occurrence. Confirm the ordinary Unity exception
    produces only its original occurrence even though Battlement also captures
    it for local presentation.
13. Remove the Diagnostics module and repeat the caught Rust panic in the
    injected core harness. Confirm local persistence, stack-trace UI, recovery,
    and the single exception-report call are unchanged. Then disable Unity's
    exception capture setting and repeat. Confirm the local exception log still
    occurs while Dashboard ingestion is not expected. Restore exception capture
    before continuing.
14. Build an isolated player for a platform with supported native crash
    reporting and matching uploaded symbols. Trigger the QA harness's native
    crash. Restart if Unity requires a subsequent launch for upload. Confirm
    the Dashboard later shows the native issue and useful symbolicated frames.
15. On Android 11 or later, use Unity's supported ANR validation procedure.
    Confirm the Dashboard receives an ANR issue. Do not model its arrival as a
    Battlement action.
16. Set log buffer size zero and create another isolated test issue. Confirm
    ordinary Unity logging still occurs but recent logs are not expected in the
    report. Restore the original value after the test.
17. Disable Diagnostics collection for a dedicated build profile while leaving
    the module selected. Run configuration, metadata, breadcrumb, and report
    commands. Confirm their local outcomes remain deterministic and the Cloud
    state continues to say `External`; do not expect Dashboard ingestion.
18. Remove the module while leaving Diagnostics collection enabled. Confirm
    `Connect.modules` omits `battlement.diagnostics`, `Connect.cloud` is absent,
    Diagnostics commands fail with `ModuleUnavailable`, and the game otherwise
    starts normally. Trigger a caught Rust panic and confirm the core bridge
    still emits its Unity exception log. Confirm this does not claim that
    engine-owned collection is disabled.
19. Create a second active `BattlementRunner`. Confirm it fails before it can
    prepare a Diagnostics clone and that the original runner remains usable.
    Destroy the original, create a replacement, and confirm settings and
    metadata ownership were released according to compare-before-restore.
20. Build WebGL and each supported native target used by the project. Confirm
    local command behavior separately from remote report support. Document
    platform limitations from Unity's current release documentation rather
    than inventing cross-platform guarantees in Battlement.

## Implementation sequence

Implement this design in vertical slices:

1. Add `battlement-cloud` values, validation, Serde fixtures, and the core
   `Connect`, command, action, and constructor integration.
2. Add `battlement-cloud-fake` and compose it into `battlement-fake`.
3. Add C# protocol records and cross-language fixtures.
4. Add the core caught-failure exception bridge and its deduplication and
   failure-isolation tests.
5. Add `BattlementDiagnosticsModule`, `IDiagnosticsBackend`, executor,
   ownership restoration, state actions, activation, lifecycle, rollback, and
   privacy-sensitive logging tests.
6. Add the unlinked public sample, locally linked non-production QA copy,
   Editor QA window, isolated crash players, and release-validation
   documentation.

Each slice must leave the public contract internally consistent. The runtime
implementation must not add Unity Analytics, Services Core, deprecated Cloud
Diagnostics, or a runtime collection toggle as an expedient substitute for
the engine-integrated Diagnostics model.
