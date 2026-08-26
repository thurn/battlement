# Battlement Cloud Analytics technical design

Status: proposed implementation contract

This document is normative for `battlement-cloud`,
`battlement-cloud-fake`, `BattlementAnalyticsModule`, and its conditionally
compiled C# support in `com.battlement.client`. It defines the first Battlement
Cloud capability: Unity Analytics for Unity 6000.5 with Analytics SDK 6.3.0.

**Unity Gaming Services (UGS)** is Unity's shared runtime for cloud products.
**Developer Data** is Unity's framework for project data controls and end-user
consent. **Analytics consent** means the Developer Data `AnalyticsIntent`
value. **Cloud state** is Battlement's complete observable snapshot of UGS and
Analytics state. These terms have these meanings throughout this document.

## Related information

- The [Battlement technical design](technical-design.md) defines sessions,
  snapshots, commands, errors, input gating, the generic `BattlementModule`
  composition model, and the Unity-to-Rust boundary.
- The [Battlement UI technical design](battlement-ui-technical-design.md)
  defines Rust-authored UI documents, deferred UI event handling, and input
  ownership.
- The [UI implementation plan](battlement-ui-implementation-plan.md) is the
  in-progress implementation on which both built-in and custom consent
  presentation depend.
- Unity's [Analytics setup][unity-analytics-setup] defines the supported
  initialization and Developer Data consent sequence.
- Unity's [Services Core API][unity-services-core] defines
  `UnityServices.InitializeAsync` and initialization states.
- Unity's [UGS environments][unity-environments] defines Environment Selector
  behavior and the `production` environment.
- Unity's [user-consent guide][unity-user-consent] defines
  `EndUserConsent`, `ConsentState`, and `ConsentStatus`.
- Unity's [Analytics privacy guide][unity-analytics-privacy] defines the
  privacy URL and data-deletion flow.
- Unity's [Analytics 6.3 API reference][unity-analytics-sdk-api] pins the SDK
  surface used by the Analytics module and the conversion contract in this
  document.
- Unity's [event-recording guide][unity-record-event],
  [custom-event guide][unity-custom-event], and
  [CustomEvent API guide][unity-custom-event-api] define event acceptance and
  supported parameter types.
- Unity's [standard-event reference][unity-standard-events],
  [ad-impression guide][unity-ad-impression], and
  [transaction guide][unity-transactions] define the manually recorded
  standard events.
- Unity's [SDK behavior guide][unity-sdk-behavior] defines buffering, the
  60-second upload cadence, disk caching, and manual flush behavior.
- Unity's [external-user-ID guide][unity-external-user-id] defines effective
  identity and explicitly requires applications to persist external IDs.
- Unity's [Event Manager][unity-event-manager],
  [Event Browser][unity-event-browser], and
  [debugging guide][unity-debugging] define the server-side schema and
  operational verification tools.

[unity-analytics-setup]:
  https://docs.unity.com/en-us/analytics/get-started/get-started
[unity-services-core]:
  https://docs.unity.com/en-us/services/services-core-api
[unity-environments]:
  https://docs.unity.com/en-us/services/service-environments
[unity-user-consent]:
  https://docs.unity.com/en-us/cloud/developer-data/user-consent
[unity-analytics-privacy]:
  https://docs.unity.com/en-us/analytics/privacy-and-consent/manage-data-privacy
[unity-analytics-sdk-api]:
  https://docs.unity3d.com/Packages/com.unity.services.analytics@6.3/api/
[unity-record-event]:
  https://docs.unity.com/en-us/analytics/events/record-event
[unity-custom-event]:
  https://docs.unity.com/en-us/analytics/events/custom-event
[unity-custom-event-api]:
  https://docs.unity.com/en-us/analytics/sdks-and-apis/custom-event-class
[unity-standard-events]:
  https://docs.unity.com/en-us/analytics/events/standard-events
[unity-ad-impression]:
  https://docs.unity.com/en-us/analytics/events/record-ad-impression-events
[unity-transactions]:
  https://docs.unity.com/en-us/analytics/events/record-transaction-events
[unity-sdk-behavior]:
  https://docs.unity.com/en-us/analytics/sdks-and-apis/sdk-behaviour
[unity-external-user-id]:
  https://docs.unity.com/en-us/analytics/events/custom-user-id-support
[unity-event-manager]:
  https://docs.unity.com/en-us/analytics/events/event-manager
[unity-event-browser]:
  https://docs.unity.com/en-us/analytics/events/event-browser
[unity-debugging]:
  https://docs.unity.com/en-us/analytics/sdks-and-apis/debugging

## Summary

Battlement Cloud adds typed Rust commands for Unity Analytics while preserving
Battlement's thin-client model. Rust decides which manual events to record and
when to change consent or identity. Unity owns UGS initialization, Developer
Data persistence, Analytics buffering and upload, the Analytics session, and
the actual platform UI used by the built-in consent dialog.

Cloud is optional at runtime. The base Rust and C# protocols always recognize
Cloud commands and state. A game opts in by adding one
`BattlementAnalyticsModule` asset to the serialized module list on
`BattlementRunner`. Only that selected runtime clone may initialize UGS or call
Analytics. Without the module, `Connect.cloud` is absent and Analytics Cloud
commands fail with the core `ModuleUnavailable` error, even when the Unity
cloud SDKs are installed.

`com.battlement.client` does not depend on Unity Analytics. It contains an
Analytics module assembly whose assembly definition uses a package version
define and matching define constraint. Unity compiles and references that
assembly only when the game separately installs
`com.unity.services.analytics` 6.3.0. Installing the SDK makes
`BattlementAnalyticsModule` available in the Editor but does not activate it.
Services Core remains private plumbing used by the Analytics implementation;
it has no module asset. Future UGS-backed modules contribute only their
user-facing service assets and must share a private, per-runner Services Core
initialization task when they are introduced.

There is one Analytics stream. The Developer Data consent value gates Unity's
automatic events and all Battlement manual events. Battlement does not offer
unsupported per-event switches for Unity's automatic events. Unity Diagnostics
is controlled separately at the project level and is outside this design.

## Battlement execution model

`BattlementRunner` is the one Unity host that owns a Battlement session. At
connection time Unity sends `Connect`, a platform snapshot, to the Rust rules
engine. Rust returns ordered batches of `Command` values. Each command has a
nonzero UUID `command_id`, one `CommandBody`, and a `blocking` Boolean that
defaults to true. A batch group finishes only when all of its blocking commands
finish; later groups may start after that point.

Unity sends an `Action` to Rust when platform or user state changes. Each action
has a session-unique UUID, the current session ID, and one `ActionBody`. Rust
returns any resulting commands synchronously. `InputSetEnabled(false)` blocks
gameplay and Rust-owned UI input actions, but it does not block system state
actions such as Cloud reports.

Unity APIs and the Rust transport execute on Unity's main thread. If an action
occurs inside a Unity callback where applying the Rust response would be
reentrant, the runner completes the callback first and then applies the
response from a FIFO deferred queue. This is the safe deferred-response path.
It preserves action and command order without calling Rust concurrently.

`Battlement.UI` is a mandatory assembly in `com.battlement.client`. Every
runner owns a system overlay root even when the Rust snapshot contains no UI
document. Package-owned modal UI attaches to that root; Rust-authored UI
attaches beneath it. Destroying the runner destroys both roots and ends the
session.

## Ownership and dependency boundaries

`battlement-cloud` owns the public Rust domain values, builders, validation,
and Serde forms for Cloud state, consent, identity, custom Analytics events,
and Unity's manually recorded standard events. It has no native library and
does not initialize UGS.

The core `battlement` crate depends on `battlement-cloud`. It embeds the Cloud
payload types directly in `Connect`, `CommandBody`, and `ActionBody`, and adds
the `Command` convenience constructors described below. This direction avoids
a dependency cycle: `battlement-cloud` never depends on `battlement`.

`battlement-cloud-fake` depends on the Cloud value crate and the core
`battlement` crate, whose `CommandId` and `CoreErrorCode` types appear in its
public test API. This does not create a cycle because the core crate does not
depend on the fake. The existing `battlement-fake` crate composes that state
machine into `FakeClient`; `battlement-cloud-fake` does not depend on
`battlement-fake`.

The base `com.battlement.client` package always contains all Cloud protocol
records in `Battlement.Protocol`. `Battlement.Runtime` contains the generic
`BattlementModule` infrastructure and unavailable-module command path defined
by the main technical design. Neither base assembly references Unity Services
Core or Analytics, and the package manifest declares neither as a dependency.

The same package contains one `Battlement.Cloud.Analytics` assembly conditioned
on exactly `com.unity.services.analytics` `[6.3.0]`. It references
`Battlement.Protocol`, `Battlement.Runtime`, `Battlement.UI`, Unity Services
Core, Analytics, and the Developer Data consent API. It defines
`BattlementAnalyticsModule`, which owns UGS initialization, consent, Analytics
state, events, privacy operations, and the built-in consent UI. It has no
static startup hook. The game installs Analytics separately in its project
manifest, creates the Analytics module asset, and selects it on
`BattlementRunner`. The published Battlement package therefore does not pull
UGS or Analytics into games that do not use them.

### Analytics module

The only user-facing asset in this design extends the generic module base:

```csharp
[CreateAssetMenu]
public sealed class BattlementAnalyticsModule : BattlementModule
{
    public override string ModuleId => "com.unity.services.analytics";
}

public enum CloudCommandOrigin
{
    None,
    PointerClick,
    UiPointerClick,
    UiNavigationSubmit,
    Other,
}

public sealed record CloudExecutionContext(
    Command Command,
    SessionId SessionId,
    CloudCommandOrigin Origin
);
```

During the generic preparation phase, the Analytics clone loads persistence,
applies pending privacy state, and restores external identity. During
initialization, it starts and retains exactly one Services Core task. Because
every selected module is prepared first, UGS cannot start before Analytics has
applied its privacy state.

The module asset remains immutable. Its per-session clone owns Analytics state,
subscriptions, dialogs, the UGS task, and deferred reports.
`BattlementRunner` disposes it at session teardown. Installing the SDK without
selecting the Analytics module asset performs no registration, initialization,
or SDK call.

The runner validates session and action correlation before constructing
`CloudExecutionContext`. `UiPointerClick` and `UiNavigationSubmit` require a
`VisualElement` action whose click source has that exact kind. Cloud uses
`Origin` only for WebGL popup-policy validation; it does not alter Analytics
event contents.

The Analytics module runtime receives these injectable boundaries:

```csharp
public interface IServicesBackend
{
    ServicesInitializationState State { get; }
    string ExternalUserId { get; set; }
    Task InitializeAsync();
}

public interface IConsentBackend
{
    AnalyticsConsent GetAnalyticsConsent();
    void SetAnalyticsConsent(AnalyticsConsent consent);
}

public interface IAnalyticsBackend
{
    string GetAnalyticsUserId();
    string SessionId { get; }
    string PrivacyUrl { get; }
    void RecordEvent(Unity.Services.Analytics.Event value);
    void Flush();
    void RequestDataDeletion();
}

public interface ICloudStorage
{
    CloudPersistenceRecord Load();
    void Save(CloudPersistenceRecord value);
}

public interface IUrlOpener
{
    void Open(string absoluteHttpsUrl);
}

internal enum AnalyticsTraceMutation
{
    CustomAdd,
    PropertySet,
    CollectionAdd,
}

internal enum AnalyticsTraceValueKind
{
    String,
    Int32,
    Int64,
    Float32,
    Float64,
    Boolean,
    DateTimeUtc,
    SdkEnum,
    Composite,
}

internal sealed record AnalyticsConversionTraceEntry(
    AnalyticsTraceMutation Mutation,
    string SdkMember,
    AnalyticsTraceValueKind ValueKind,
    object Value
);

internal interface IAnalyticsConversionTrace
{
    void Observe(AnalyticsConversionTraceEntry entry);
}
```

The production implementations are thin adapters over Services Core,
`EndUserConsent`, Analytics 6.3, one serialized PlayerPrefs record, and
`Application.OpenURL`. `AnalyticsEventConverter` maps the normalized protocol
union to a new SDK `Event` before calling the injected Analytics backend. The
production backend passes that event to
`AnalyticsService.Instance.RecordEvent`. Backend spies receive the converted
SDK instance, so tests replace neither the executor nor the converter. Backend
methods either return normally or throw, and all except initialization execute
synchronously on the main thread.

The production conversion trace is a no-op and never stores or logs values.
Immediately before each public SDK mutation, converter tests can inject a spy
that receives one entry: `CustomAdd` for `CustomEvent.Add`, `PropertySet` for a
standard-event setter, and `CollectionAdd` for each product appended to a
standard-event collection. `SdkMember` is the exact public SDK parameter,
property, or collection name. A `CollectionAdd` has `Composite` value kind and
an ordered read-only list of `PropertySet` entries for its nested SDK fields.
`ValueKind` and the boxed `Value` retain the exact CLR type or SDK enum member
assigned.

For custom values, the converter boxes `string`, `int`, `long`, `float`,
`double`, `bool`, or UTC `DateTime` and passes it to the public
`CustomEvent.Add(string, object)` method. A pinned-6.3 source-backed test proves
that `Add` dispatches each boxed type to its corresponding protected setter.
Analytics 6.3 then stores `int` as `long` and `float` as `double`, so neither
the final SDK `Event` nor Debug Panel retains those two width distinctions.
Width identity is normative on the wire and at the boxed `Add` boundary; Unity
receives its documented integer and floating buckets.

Every assignment through `IServicesBackend.ExternalUserId` uses an
assign-and-verify helper. It returns successfully only when the setter returns
normally and a getter readback, with null and empty normalized to `None`,
equals the requested value. It performs the readback even when the setter
throws, because Services Core can mutate its backing value before a subscriber
throws. A throwing assignment remains a failure even if readback shows the
requested value; the caller follows its compensation rules below.

`ICloudStorage.Save` is an atomic durable boundary. Its PlayerPrefs adapter uses
two namespaced, checksummed record slots with monotonically increasing storage
generations and a committed-slot marker. Loading selects the newest committed
valid slot. A normal return makes the complete record durable; a thrown call
leaves the prior record authoritative. A crash after commit is recovered as a
completed save, even when the calling command never returned.

Serde is Rust's serialization framework. PlayerPrefs is Unity's local
key-value persistence facility.

## Startup and UGS initialization

`BattlementRunner.OnEnable` clones its selected modules using the generic
module lifecycle. If no `BattlementAnalyticsModule` is selected, Analytics is
unavailable, the runner omits `Connect.cloud`, and this design makes no UGS API
call. Merely installing Analytics or Services Core has no runtime effect.

When `BattlementAnalyticsModule` is selected, its runtime clone starts in this
order:

1. Read Battlement's serialized Cloud persistence record.
2. If deletion intent exists, synchronously set Developer Data consent to
   `Denied` and verify it by readback before touching UGS. Otherwise retain the
   current consent.
3. Assign and verify the pending deletion target when one exists, otherwise
   the persisted external ID, or an empty value when neither exists, through
   `UnityServices.ExternalUserId`.
4. Supply no Battlement environment override to Services Core, so Unity uses
   the Environment Selector value and otherwise defaults to `production`.
5. Start one shared `UnityServices.InitializeAsync` task.
6. Expose the task and current Cloud state to the attached runner.

The pinned UGS initialization API has no cancellation or deinitialization
operation. Disposing the Analytics runtime clone therefore cancels
Battlement-owned waits and prevents Battlement from observing or applying a
later completion; it does not claim to stop Unity's underlying initialization.
The clone marks itself disposed before invoking cleanup. Before changing Cloud
state, persistence, Unity identity, or emitting an action, every asynchronous
continuation verifies that the clone is not disposed. A stale continuation may
release resources it owns but has no other effect.

If either pending-deletion preflight assignment throws or its readback does
not match, the Analytics module does not call `InitializeAsync`. It creates a
terminal `Failed` state with the `Consent` or `ExternalUserId` failure that
actually blocked the preflight and emits `InitializationFailed`. Commands
requiring UGS fail with `CloudInitializationFailed`. An explicit
`RetryCloudInitialization` repeats consent denial and identity verification
before it may initialize. This prevents Unity automatic events from starting
under granted consent after a crash that persisted intent but had not yet
denied consent.

The Analytics runtime clone owns its initialization task. When
`UnityServices.State` is `Uninitialized`, it calls
`UnityServices.InitializeAsync`; when it is `Initialized`, it adopts the
existing Services Core instance. `Initializing` without an observable task is
a `Configuration` failure. Battlement never starts a competing initialization
call. After the shared task completes, the Analytics module verifies that its
service instance, session ID, and privacy URL are readable before becoming
`Ready`.

The initialization task starts when the runner attaches and does not delay
creation of the Rust engine. Consequently, the first `Connect` commonly reports
`Initializing`. Restoring the external ID before initialization ensures that
Analytics does not record startup events under an installation ID and then
switch identities immediately afterward.

Every Cloud operation that requires an initialized Analytics SDK awaits the
same task. The Analytics module never starts a second initialization
concurrently. Commands that do not require UGS, including consent presentation,
consent changes, external-ID persistence, and state reports, remain available
while initialization is in progress or failed.

Initialization has three wire states:

- `Initializing` means one shared initialization attempt is running.
- `Ready` means Services Core completed and the Analytics instance is usable.
- `Failed` requires `CloudState.failure` to contain a stable category, Unity
  error code when available, and a sanitized message. It is terminal until
  Rust explicitly sends `RetryCloudInitialization`.

After preparation registers its action source, the Analytics module publishes
a Cloud state report for every transition to `Ready` or `Failed`. A transition
during connection is not retained for historical replay: `Connect.cloud` is
authoritative. The runner registers the action source before initialization
and reads its connection snapshot afterward, so a transition is represented by
`Connect.cloud` or by a later higher-revision action.

`RetryCloudInitialization` is valid only in `Failed`. It repeats the complete
pending-deletion consent and identity preflight, or restores and verifies the
ordinary persisted external ID, and preserves Unity's environment-selection
behavior. It first creates and installs a replacement shared task, then changes
state to `Initializing`, and then emits a correlated state action. A command
responding to `RetryStarted` therefore always joins the replacement task. An
initialization completion may mutate the clone only while its task is still
the clone's current initialization task. A failed preflight completes that
task as failed without invoking UGS and emits the correlated terminal failure.
Calling retry in `Initializing` or `Ready` fails with
`CloudRetryUnavailable`. A failed command never retries UGS implicitly.

Before a retry invokes Services Core, it inspects `UnityServices.State`.
`Uninitialized` starts a new `InitializeAsync` call. `Initialized` verifies
that `AnalyticsService.Instance`, `SessionID`, and `PrivacyUrl` are readable and
then completes the replacement task successfully. `Initializing` without the
module clone's task is a `Configuration` failure. Any exception during those
checks fails the replacement attempt.

An Analytics operation that was already waiting follows the attempt it joined.
If that attempt fails, the operation fails with `CloudInitializationFailed`.
It is not automatically carried into a later retry. Rust may send the operation
again after observing `Ready`.

The Analytics runtime clone owns initialization, consent and identity state,
persistence, and the current initialization task for its session. Task identity
prevents a replaced initialization attempt from mutating the clone; clone
disposal prevents work from an ended runner session from mutating anything.
Neither lifetime uses a numeric generation counter.

Runner teardown closes the dialog, cancels session subscriptions and deferred
actions, disposes the Analytics runtime clone, and discards reports not
submitted to that session. Services Core itself remains process-global after
successful initialization, but no module runtime, task wrapper, state, or
module selection is process-static. A replacement runner creates a fresh clone
and either initializes an uninitialized Services Core instance or adopts an
already initialized one under the checks above.

## Protocol contract

All Cloud values use the existing JSON protocol: struct fields retain their
Rust `snake_case` names, unit enum variants are strings, and variants carrying
data use Serde's externally tagged single-property objects. The value tags keep
values that look identical in JSON, such as `Int32(4)` and `Int64(4)`, distinct.

Required fields are always emitted. Optional fields are omitted when `None`
and accepted as either omitted or JSON `null`; writers always omit them.
Readers reject unknown enum variants and duplicate object properties, but
ignore unknown struct properties so a malformed optional addition cannot
change a known field. Lists preserve wire order. Object property order is not
significant. Rust produces minified UTF-8, and C# must deserialize the same
shapes without numeric coercion.

### Connection state

`Connect` gains an optional `cloud: Option<CloudState>` field. Absence means
the runner did not select a valid `BattlementAnalyticsModule`. Presence means
its runtime clone is active, including while initialization is in progress or
failed. Installing Analytics without adding the asset to
`BattlementRunner.modules` leaves the field absent. The generic
`Connect.modules` list contains `com.unity.services.analytics`; Services Core
is an implementation dependency and is not reported as a selected module.

Conceptually, the Rust state is:

```rust
pub struct CloudState {
    pub revision: u64,
    pub initialization: CloudInitialization,
    pub analytics_consent: AnalyticsConsent,
    pub effective_user_id: Option<String>,
    pub persisted_external_user_id: Option<String>,
    pub analytics_session_id: Option<String>,
    pub privacy_url: Option<String>,
    pub data_deletion_pending: bool,
    pub failure: Option<CloudFailure>,
}

pub enum CloudInitialization {
    Initializing,
    Ready,
    Failed,
}

pub enum AnalyticsConsent {
    Unspecified,
    Granted,
    Denied,
}

pub struct CloudFailure {
    pub operation: CloudOperation,
    pub kind: CloudFailureKind,
    pub unity_error_code: Option<i64>,
    pub message: String,
}
```

`effective_user_id`, `analytics_session_id`, and `privacy_url` are `None`
until Analytics is ready. `effective_user_id` is the value returned by
`GetAnalyticsUserID`: either the external ID or Unity's installation ID.
`analytics_session_id` comes from `AnalyticsService.Instance.SessionID`, and
`privacy_url` comes from `AnalyticsService.Instance.PrivacyUrl`.
`persisted_external_user_id` is Battlement's local value and is observable even
before UGS is ready. The two IDs intentionally differ after a persisted value
has changed but before Analytics has become ready.

`Ready` requires all three ready-only fields to be `Some`; `Initializing` and
`Failed` require them to be `None`. `data_deletion_pending` exactly reflects a
persisted deletion intent. While it is true,
`persisted_external_user_id` is `None`, although the effective ID may
temporarily remain the deletion target. Consent is `Denied` once the privacy
mutation succeeds. A reported `Consent` failure is the only state in which a
pending deletion may temporarily retain another value; the pending-deletion
gate still suppresses every manual recording and flush command in that state.

`revision` starts at zero for each Analytics runtime clone and increments after
every state mutation. A new Battlement session creates a new clone and resets
the revision. `CloudFailure.operation` is `Initialization`,
`Consent`, `Storage`,
`AnalyticsEvent`, `ExternalUserId`, `PrivacyUrl`, or `DataDeletion`. Its kind is
`Configuration`, `Network`, `Timeout`, `ServiceUnavailable`, or `Unknown`.
The optional numeric code is Unity's `RequestFailedException.ErrorCode` when
one exists. The sanitized message is limited to 512 Unicode scalar values,
replaces control characters with spaces, and contains no nested stack trace,
user ID, event parameter, receipt, request body, or token.

`failure` is the most recent unresolved operation failure. It is required when
initialization is `Failed`. Its operation is `Initialization` for a Services
Core or Analytics readiness failure, or `Consent` or `ExternalUserId` when
that pending-deletion preflight prevented initialization. Starting an
initialization retry clears only the failure responsible for the failed state.
A successful operation clears `failure` only when its operation matches. A
newer failure replaces an older one; the failure responsible for a `Failed`
initialization state has priority over deletion and other operation failures.

### Commands and Rust conveniences

`CommandBody` gains these first-class variants:

- `RetryCloudInitialization`
- `ShowAnalyticsConsent(ShowAnalyticsConsentPayload)`
- `SetAnalyticsConsent(SetAnalyticsConsentPayload)`
- `RecordAnalyticsEvent(RecordAnalyticsEventPayload)`
- `FlushAnalytics`
- `SetAnalyticsExternalUserId(SetAnalyticsExternalUserIdPayload)`
- `ClearAnalyticsExternalUserId`
- `ReportAnalyticsState`
- `OpenAnalyticsPrivacyUrl`
- `RequestAnalyticsDataDeletion`

The non-unit payloads are exact:

```rust
pub struct ShowAnalyticsConsentPayload {
    pub title: String,
    pub body: String,
}

pub struct SetAnalyticsConsentPayload {
    pub consent: AnalyticsConsent,
}

pub struct RecordAnalyticsEventPayload {
    pub event: AnalyticsEvent,
}

pub struct SetAnalyticsExternalUserIdPayload {
    pub user_id: String,
}
```

The unit variants serialize as JSON strings, consistently with other command
bodies. The core `Command` type offers matching constructors:

```rust
Command::retry_cloud_initialization()
Command::show_analytics_consent(title, body)
Command::set_analytics_consent(consent)
Command::record_analytics_event(event)
Command::flush_analytics()
Command::set_analytics_external_user_id(user_id)
Command::clear_analytics_external_user_id()
Command::report_analytics_state()
Command::open_analytics_privacy_url()
Command::request_analytics_data_deletion()
```

These constructors generate a `CommandId` in the same way as existing
Battlement helpers. Callers that need a predetermined ID construct `Command`
directly. Every Cloud constructor creates a blocking command. A Cloud body with
`blocking: false` fails ordinary command validation before executor dispatch.
This is independent of current consent or initialization state and avoids a
data-dependent scheduling contract.

Blocking completion is specific to each command. Retry completes after the
replacement task is installed, not when UGS becomes ready. Show completes after
the modal opens. Set, clear, and report commands complete after their state
action is safely enqueued. Record and flush complete after suppression or the
SDK call. Privacy open completes after `Application.OpenURL` is called. Deletion
completes after `RequestDataDeletion` returns without throwing and Unity owns
the retryable request.

### State actions

`ActionBody` gains one system-level variant,
`CloudState(CloudStateReport)`. It is used for requested reports and unsolicited
state changes:

```rust
pub struct CloudStateReport {
    pub state: CloudState,
    pub cause: CloudStateCause,
    pub originating_command_id: Option<CommandId>,
}
```

`CloudStateCause` has `ReportRequested`, `RetryStarted`,
`InitializationReady`, `InitializationFailed`, `ConsentChanged`,
`ConsentDialogSelection`, `ConsentDialogFailed`, `ExternalUserIdChanged`,
`DataDeletionRequested`, `DataDeletionAccepted`, and
`DataDeletionRetryFailed`, plus `OperationFailed` and `OperationRecovered` for
other operation failure transitions.

A report caused directly by a command includes that command's ID. The terminal
ready or failed report for an explicit retry retains the retry command ID. A
startup initialization transition and startup deletion retry omit it.
`ReportAnalyticsState` sends a report immediately, even when initialization is
not ready, with cause `ReportRequested` and the requesting command ID. It does
not wait for a terminal state.

Originating command IDs are session-scoped. A delayed transition includes the
command ID only while its source remains the clone's current session. Clone
disposal drops later completions, so a command ID never crosses a session
boundary.

Cloud state actions are system actions, not gameplay input. They bypass the
`InputSetEnabled(false)` gameplay gate. They still use the runtime's existing
safe deferred-response path: if Unity is inside a UI Toolkit callback or
another non-reentrant dispatch boundary, the action is queued and the Rust
response is applied after that boundary closes.

The action is a complete snapshot, not a patch. Rust replaces its remembered
Cloud state only when the report revision is greater than the revision it
already holds. Actions remain ordered in the session and use normal action IDs
for deduplication.

The Analytics runtime clone does not retain state-transition reports for
historical replay. `Connect.cloud` contains its newest state and revision. A
report created after the action source is registered but before the connection
response is applied waits in the runner's ordinary deferred queue. After
connection, the runner discards queued Cloud reports whose revision is less
than or equal to the `Connect.cloud` revision and submits reports with higher
revisions. Runner teardown discards reports not yet submitted; reports already
submitted belong only to the ending session.

### Stable failures

The generic `ModuleUnavailable` error is used when either required module is
absent. The base `CoreErrorCode` union also gains these Cloud codes:

- `CloudInitializationFailed`: a required initialization attempt failed.
- `CloudRetryUnavailable`: retry was sent outside `Failed`.
- `AnalyticsConsentRequired`: a gated operation was sent while consent was
  `Unspecified`.
- `AnalyticsConsentDialogOpen`: a second built-in dialog was requested while
  the first was open.
- `AnalyticsDataDeletionFailed`: Unity did not accept a deletion request.
- `CloudOperationFailed`: another consent, persistence, identity, Analytics,
  or URL operation failed after its payload was valid.

Malformed or semantically invalid payloads use the existing property and
command validation errors and identify the field path. No sensitive value is
included in an error. After payload validation, `RecordAnalyticsEvent` and
`FlushAnalytics` observed under `Denied` complete successfully as suppressed
no-ops without constructing or calling the Analytics backend. This denied
recording behavior never throws. A request to change consent to `Denied` can
still fail observably if the consent backend itself throws.

Unity failures map deterministically. `RequestFailedException` error codes 1,
2, and 3 map to `Network`, `Timeout`, and `ServiceUnavailable`; all other codes
map to `Unknown` and retain the numeric code. A missing project link,
conflicting external Services Core initialization, or invalid runtime
configuration maps to `Configuration`. Other exceptions map to `Unknown`.
Payload validation exceptions never reach this mapping.

An initialization mapping produces `CloudInitializationFailed`. A deletion
mapping produces `AnalyticsDataDeletionFailed`. Consent, storage, identity,
event-backend, flush, and URL mappings produce `CloudOperationFailed`. Each
also updates `CloudState.failure` with its operation and emits a state report
when state changed. A malformed or non-HTTPS privacy URL is a `PrivacyUrl`
`Configuration` failure. Sanitization occurs before either the state or command
error is constructed. A consent or identity failure during the mandatory
pending-deletion initialization preflight is the exception: its operation
remains exact, but the retry command fails with `CloudInitializationFailed`
because no initialization task was permitted to start.

## Consent behavior

Battlement reads and writes `EndUserConsent` through Unity's Developer Data
API. The three wire values map exactly to `ConsentStatus.Unspecified`,
`ConsentStatus.Granted`, and `ConsentStatus.Denied`. Unity owns persistence of
this consent state. Rust and Battlement's PlayerPrefs keys do not duplicate it.

The initial value appears in `Connect.cloud.analytics_consent`. A later change
appears in a Cloud state action. `SetAnalyticsConsent` accepts only `Granted`
or `Denied`; setting `Unspecified` is rejected because this command represents
an affirmative user or application decision, not the absence of one.

Changing consent is independent of UGS initialization. The executor updates
Developer Data synchronously on Unity's main thread, refreshes Cloud state,
emits a correlated `ConsentChanged` state action, and completes the command.
It emits that action even when the requested value equals the current value.
The built-in dialog emits the separate uncorrelated state action described
below because its choice happens after the show command has completed.

### Operation gates

The executor validates a record payload before consulting consent. This keeps
invalid developer input visible even when a test or user has denied Analytics.
After validation:

- `Granted`: wait for the shared initialization task, re-read consent after
  the wait, and record only if it is still `Granted`.
- `Denied`: complete successfully without constructing or recording a Unity
  event.
- `Unspecified`: fail with `AnalyticsConsentRequired`.

The post-wait check closes the race in which the user declines while a granted
command awaits UGS. A post-wait `Denied` is a successful no-op. A post-wait
`Unspecified` fails with `AnalyticsConsentRequired`.

`FlushAnalytics` uses the same consent gate. It is a successful no-op while
denied, fails while unspecified, and waits for UGS before calling `Flush` while
granted. This prevents a Battlement flush request from uploading buffered data
after the application has withdrawn consent.

External-ID changes, consent commands, dialog commands, state reports, and
deletion's immediate privacy mutations do not use the recording gate. Opening
the Analytics privacy URL waits for UGS because the URL is obtained from the
Analytics SDK.

While `data_deletion_pending` is true, setting consent to `Granted`, setting an
external user ID, or opening the built-in consent dialog fails with
`AnalyticsDataDeletionFailed`. Denying consent, clearing the ID, reporting
state, and reissuing deletion remain valid. This prevents a fixed `Accept`
button or an identity change from reopening collection before Unity accepts
the pending deletion request.

### Built-in consent dialog

`ShowAnalyticsConsent` takes exactly two strings: `title` and `body`. Both are
plain text; rich-text interpretation in UI Toolkit, Unity's user-interface
system, is disabled. Trimming must leave each nonempty. The title is limited to
200 Unicode scalar values and the body to 4,000. Invalid text fails before any
overlay is created.

The Analytics module creates a module-owned Battlement UI modal. It
is a responsive, full-panel overlay with a title, scrollable body when
necessary, and fixed buttons labeled `Accept` and `Decline`. It respects the
current Battlement UI panel theme, safe area, focus navigation, scaling, and
supported pointer, keyboard, and controller activation behavior. The buttons
and labels cannot be renamed by the command.

The show command completes when the overlay has been constructed, attached,
focused, and made visible. It does not wait for a choice. An explicit command
opens the dialog even when consent is already granted or denied. If the dialog
is already open, a second show command fails with
`AnalyticsConsentDialogOpen` and does not replace its text or focus.

The runner's system overlay root exists before Cloud command dispatch. If
construction, attachment, or focus acquisition fails, the executor destroys
the partial tree, releases any lease it acquired, and fails the show command
with `CloudOperationFailed`. It records a correlated `Consent`
`CloudState.failure`, increments the state revision, and emits
`OperationFailed`; `ConsentDialogFailed` is reserved for a later asynchronous
button selection. Cancellation caused by runner teardown performs cleanup
without changing Cloud state. Escape, browser back, and controller cancel do
not dismiss the dialog; only `Accept`, `Decline`, or runner teardown closes it.
Application suspension leaves it open.

Opening the modal acquires a separate temporary input-suppression lease. The
lease blocks gameplay pointer, keyboard, and controller input and prevents
interaction with Rust-owned UI beneath the overlay, while leaving the modal's
own controls operable. Releasing it restores the exact gameplay-input state
that existed before the modal; it does not blindly enable input.

The prior consent remains active while the dialog is open. Pressing a button
performs this ordered transaction on Unity's main thread:

1. Set Developer Data Analytics consent to `Granted` for `Accept` or `Denied`
   for `Decline`.
2. Refresh in-memory Cloud state from the selected value.
3. Close the package modal token, which detaches the overlay, marks the dialog
   closed, and releases input suppression idempotently.
4. Destroy detached UI objects on a best-effort basis.
5. Emit `CloudState` with cause `ConsentDialogSelection`.

The state action is emitted even when the selected value equals the prior
value. It has no originating command ID because the show command has already
completed; the action represents later user input. Decline follows the same
successful path as accept and never throws.

If writing Developer Data consent throws, the handler catches the exception,
leaves the prior consent and modal input lease active, re-enables both buttons,
and emits `ConsentDialogFailed` with a `Consent` failure. No exception escapes
the UI callback, including for `Decline`. A later button press retries the
complete selection transaction. After a successful setter return, the executor
uses the selected value as the refreshed consent state.

Closing the modal token and enqueuing a module state report are nonthrowing
package operations except for process-fatal allocation failure. The token
changes its ownership state before calling Unity cleanup, so a Unity exception
cannot retain an input lease or leave the dialog logically open. Cleanup
exceptions are sanitized and accumulated. The selected consent remains in
force, `ConsentDialogSelection` is still emitted, then the executor records a
`Consent` failure and emits `ConsentDialogFailed`. Report ordering therefore
distinguishes a completed choice from failed presentation cleanup.

Runner teardown closes an open overlay and releases its lease without changing
consent or emitting a selection action. Scene changes do not close it because
the active runner and its Battlement UI root outlive controlled content scenes.
A transport disconnect without runner teardown leaves the overlay open. Its
later selection updates the clone without emitting an action while
disconnected; the next `Connect.cloud` snapshot contains the resulting state.

The built-in dialog is a presentation convenience. It does not determine
whether consent is legally required, choose legally sufficient wording,
capture different processing purposes, record a legal audit trail, or replace
a consent-management platform.

### Custom consent UI

A custom consent screen is an ordinary Rust-owned `battlement-ui` document.
The game owns its tree, localization, styles, accessibility text, additional
choices, links, and lifecycle. Its accept and decline actions return
`SetAnalyticsConsent(Granted)` and `SetAnalyticsConsent(Denied)` commands.

The Cloud API does not accept, store, mutate, or destroy a custom UI tree. The
game uses ordinary Battlement UI commands to show and close that UI and uses
ordinary input gating if it needs modal behavior. This keeps one source of
truth for custom UI and lets the fake client exercise consent flows without a
second UI document model.

## Manual Analytics events

`RecordAnalyticsEvent` contains one `AnalyticsEvent` value. Its wire variants
are `Custom`, `AcquisitionSource`, `AdImpression`, `Transaction`, and
`TransactionFailed`. Unity's automatic lifecycle events are not valid values
for this command.

```rust
pub enum AnalyticsEvent {
    Custom(CustomAnalyticsEvent),
    AcquisitionSource(AcquisitionSourceEvent),
    AdImpression(AdImpressionEvent),
    Transaction(TransactionEvent),
    TransactionFailed(TransactionFailedEvent),
}
```

A successful record command has two consent-dependent meanings. Under
`Granted`, it means the Analytics 6.3 SDK accepted the event into its local
buffer. Under `Denied`, it means Battlement validated and deliberately
suppressed the event without constructing an SDK event. It never means Unity
uploaded the event, matched its Event Manager schema, enabled it, or accepted
it into reporting. Those checks happen later and cannot become synchronous
Battlement errors. The SDK may also cache an accepted event offline.

The executor converts a validated wire value to the corresponding SDK event
class and calls `AnalyticsService.Instance.RecordEvent` exactly once. It does
not reuse mutable SDK event objects. Event logs contain the event name and
outcome only, never parameter names and values together, receipts, user IDs,
or serialized event bodies.

All SDK types in this section are in `Unity.Services.Analytics`. Custom events
use `new CustomEvent(name)`. The four standard variants use the parameterless
`AcquisitionSourceEvent`, `AdImpressionEvent`, `TransactionEvent`, and
`TransactionFailedEvent` constructors. The converter assigns only present
optional fields and always assigns every Battlement-required field.

### Shared string and collection limits

Battlement imposes deterministic limits before crossing into C#:

- Event and parameter names contain 1 to 100 ASCII characters.
- Ordinary event string fields contain at most 1,024 Unicode scalar values.
  Required strings must also be nonempty after trimming.
- Custom string parameter values contain at most 100 Unicode scalar values.
  Empty values are allowed. This makes Unity's documented recommendation to
  avoid larger parameter values an enforceable Battlement contract.
- A custom event contains at most 1,000 parameters.
- A receipt contains at most 1 MiB of UTF-8 and a receipt signature at most
  64 KiB. Empty optional receipt fields are rejected when present.
- Each product collection contains at most 1,000 entries.
- The minified `serde_json` encoding of `AnalyticsEvent`, excluding its outer
  command wrapper, must be no larger than 3 MiB of UTF-8. This leaves headroom
  below Unity's 4 MiB event or batch limit.

Limits count Unicode scalar values, not UTF-16 code units, except where the
limit explicitly names UTF-8 bytes. Required and optional standard-event
strings are preserved exactly but are rejected when leading or trailing
Unicode whitespace is present; an optional string is also rejected when empty.
Custom string parameter values are preserved exactly, including whitespace.
Rust validation is authoritative and C# repeats it at the untrusted JSON
boundary.

### Custom events

A custom event has a name and an ordered list of uniquely named parameters.
Names are case-sensitive and must match `^[A-Za-z][A-Za-z0-9_]*$`. A parameter
name may not appear twice with the same case. `score` and `Score` are distinct
because Unity schemas treat them as distinct. Names reserved by Unity's
current Event Manager are rejected; the maintained list is sourced from
Unity's [reserved-parameter reference][unity-reserved-parameters].

```rust
pub struct CustomAnalyticsEvent {
    pub name: String,
    pub parameters: Vec<AnalyticsParameter>,
}

pub struct AnalyticsParameter {
    pub name: String,
    pub value: AnalyticsParameterValue,
}
```

[unity-reserved-parameters]:
  https://docs.unity.com/en-us/analytics/events/reserved-parameter-names

The parameter union is:

```rust
pub enum AnalyticsParameterValue {
    String(String),
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    Boolean(bool),
    TimestampUtc(i64),
}
```

`TimestampUtc` is Unix milliseconds since 1970-01-01T00:00:00Z. Its value must
be in the inclusive range `-62135596800000..=253402300799999`, which converts
to a .NET UTC `DateTime` in years 0001 through 9999. C# uses UTC explicitly; it
never interprets the value in the device's local time zone. Both floating
variants reject NaN and positive or negative infinity.

The distinct tags preserve numeric identity across JSON. For example:

```json
{
  "RecordAnalyticsEvent": {
    "event": {
      "Custom": {
        "name": "levelCompleted",
        "parameters": [
          { "name": "score", "value": { "Int32": 1250 } },
          { "name": "totalScore", "value": { "Int64": 9007199254740991 } },
          { "name": "accuracy", "value": { "Float32": 0.875 } },
          { "name": "elapsed", "value": { "Float64": 42.125 } },
          { "name": "perfect", "value": { "Boolean": true } },
          {
            "name": "completedAt",
            "value": { "TimestampUtc": 1787688000000 }
          }
        ]
      }
    }
  }
}
```

The dashboard event must already exist, be enabled, and define matching
case-sensitive parameter types. Battlement validates wire syntax and SDK
types, not dashboard configuration.

### Acquisition source

`AcquisitionSourceEvent` mirrors every Analytics 6.3
`AcquisitionSourceEvent` property.

```rust
pub struct AcquisitionSourceEvent {
    pub acquisition_channel: String,
    pub acquisition_campaign_id: String,
    pub acquisition_creative_id: String,
    pub acquisition_campaign_name: String,
    pub acquisition_provider: String,
    pub acquisition_cost: Option<f32>,
    pub acquisition_cost_currency: Option<String>,
    pub acquisition_network: Option<String>,
    pub acquisition_campaign_type: Option<String>,
}
```

Required nonempty strings are:

- `acquisition_channel`
- `acquisition_campaign_id`
- `acquisition_creative_id`
- `acquisition_campaign_name`
- `acquisition_provider`

Optional fields are `acquisition_cost: f32`,
`acquisition_cost_currency`, `acquisition_network`, and
`acquisition_campaign_type`. Cost must be finite and nonnegative. When present,
the currency is exactly three uppercase ASCII letters representing an ISO 4217
code. Cost and currency must either both be present or both be absent; the
validator checks code syntax, not membership in a changing currency registry.
Battlement maps these fields to Unity's corresponding
`AcquisitionSourceEvent` properties and records the SDK event named
`acquisitionSource`.

The setter mapping is `AcquisitionChannel`, `AcquisitionCampaignId`,
`AcquisitionCreativeId`, `AcquisitionCampaignName`, `AcquisitionProvider`,
`AcquisitionCost`, `AcquisitionCostCurrency`, `AcquisitionNetwork`, and
`AcquisitionCampaignType`, in wire-field order.

### Ad impression

`AdImpressionEvent` mirrors every Analytics 6.3 `AdImpressionEvent` property.
Unity marks `placement_id` and `placement_name` as required, so Battlement
requires both as nonempty strings. All other fields are optional:

```rust
pub struct AdImpressionEvent {
    pub placement_id: String,
    pub placement_name: String,
    pub completion_status: Option<AdCompletionStatus>,
    pub provider: Option<AdProvider>,
    pub placement_type: Option<AdPlacementType>,
    pub ad_ecpm_usd: Option<f64>,
    pub store_destination_id: Option<String>,
    pub sdk_version: Option<String>,
    pub impression_id: Option<String>,
    pub media_type: Option<String>,
    pub time_watched_ms: Option<i64>,
    pub time_close_button_shown_ms: Option<i64>,
    pub length_ms: Option<i64>,
    pub has_clicked: Option<bool>,
    pub source: Option<String>,
    pub status_callback: Option<String>,
}
```

- `completion_status: AdCompletionStatus`
- `provider: AdProvider`
- `placement_type: AdPlacementType`
- `ad_ecpm_usd: f64`
- `store_destination_id: String`
- `sdk_version: String`
- `impression_id: String`
- `media_type: String`
- `time_watched_ms: i64`
- `time_close_button_shown_ms: i64`
- `length_ms: i64`
- `has_clicked: bool`
- `source: String`
- `status_callback: String`

`ad_ecpm_usd` must be finite and nonnegative. All millisecond durations must be
nonnegative. If both watched time and length are present, watched time may not
exceed length. If both close-button time and length are present, close-button
time may not exceed length.

The enums mirror the complete SDK sets:

- `AdCompletionStatus`: `Completed`, `Partial`, `Incomplete`.
- `AdPlacementType`: `Banner`, `Rewarded`, `Interstitial`, `Other`.
- `AdProvider`: `AdColony`, `AdMob`, `Amazon`, `AppLovin`, `ChartBoost`,
  `Facebook`, `Fyber`, `Hyprmx`, `Inmobi`, `Maio`, `Pangle`, `Tapjoy`,
  `UnityAds`, `Vungle`, `IronSource`, `Other`.

`IronSource` maps to the Analytics 6.3 SDK's `IrnSource` enum member. The wire
spelling is the correctly branded Rust name and is not derived from the SDK's
misspelling. The other ad-provider and completion-status wire variants map to
same-spelled SDK members. Placement wire values map exhaustively as follows:

- `Banner` to `AdPlacementType.BANNER`;
- `Rewarded` to `AdPlacementType.REWARDED`;
- `Interstitial` to `AdPlacementType.INTERSTITIAL`; and
- `Other` to `AdPlacementType.OTHER`.

The remaining setter mapping is `PlacementId`, `PlacementName`,
`AdCompletionStatus`, `AdProvider`, `PlacementType`, `AdEcpmUsd`,
`AdStoreDestinationId`, `AdSdkVersion`, `AdImpressionId`, `AdMediaType`,
`AdTimeWatchedMs`, `AdTimeCloseButtonShownMs`, `AdLengthMs`, `AdHasClicked`,
`AdSource`, and `AdStatusCallback`, in struct-field order.

### Transactions

`TransactionEvent` and `TransactionFailedEvent` share one complete transaction
payload. Required fields are a nonempty `transaction_name` and a
`transaction_type` other than `Invalid`. A failed transaction additionally
requires a nonempty `failure_reason`.

```rust
pub struct TransactionEvent {
    pub transaction_name: String,
    pub transaction_type: TransactionType,
    pub transaction_id: Option<String>,
    pub payment_country: Option<String>,
    pub product_id: Option<String>,
    pub store_item_sku_id: Option<String>,
    pub store_item_id: Option<String>,
    pub store_id: Option<String>,
    pub store_source_id: Option<String>,
    pub transaction_receipt: Option<String>,
    pub transaction_receipt_signature: Option<String>,
    pub transaction_server: Option<TransactionServer>,
    pub transactor_id: Option<String>,
    pub products_spent: TransactionProducts,
    pub products_received: TransactionProducts,
}

pub struct TransactionFailedEvent {
    #[serde(flatten)]
    pub transaction: TransactionEvent,
    pub failure_reason: String,
}
```

`serde(flatten)` makes the failed-event wire object contain all transaction
fields beside `failure_reason`; there is no nested `transaction` property.

`products_spent` and `products_received` are required protocol objects whose
members default to empty. C# constructs `TransactionEvent` or
`TransactionFailedEvent`, assigns every present scalar through the exact SDK
setter, and appends product records to `SpentRealCurrency`,
`SpentVirtualCurrencies`, `SpentItems`, `ReceivedRealCurrency`,
`ReceivedVirtualCurrencies`, and `ReceivedItems`. The SDK itself supplies
`sdkVersion`; it is not a Battlement field.

The optional scalar fields mirror Analytics 6.3:

- `transaction_id`
- `payment_country`
- `product_id`
- `store_item_sku_id`
- `store_item_id`
- `store_id`
- `store_source_id`
- `transaction_receipt`
- `transaction_receipt_signature`
- `transaction_server`
- `transactor_id`

The exact SDK enum sets are:

- `TransactionType`: `Invalid`, `Sale`, `Purchase`, `Trade`.
- `TransactionServer`: `Apple`, `Amazon`, `Google`, `Valve`.
- `VirtualCurrencyType`: `Grind`, `Premium`, `PremiumGrind`.

These are title-case wire variants, not C# identifiers. Conversion maps
`Invalid`, `Sale`, `Purchase`, and `Trade` to `TransactionType.INVALID`,
`.SALE`, `.PURCHASE`, and `.TRADE`; maps `Apple`, `Amazon`, `Google`, and
`Valve` to `TransactionServer.APPLE`, `.AMAZON`, `.GOOGLE`, and `.VALVE`; and
maps `Grind`, `Premium`, and `PremiumGrind` to
`VirtualCurrencyType.GRIND`, `.PREMIUM`, and `.PREMIUM_GRIND`.

`Invalid` exists on the wire to mirror the SDK enum but is a sentinel, not a
recordable transaction type. Validation rejects it.

Both `products_spent` and `products_received` use this shape:

```rust
pub struct TransactionProducts {
    pub real_currency: Option<TransactionRealCurrency>,
    pub virtual_currencies: Vec<TransactionVirtualCurrency>,
    pub items: Vec<TransactionItem>,
}

pub struct TransactionRealCurrency {
    pub currency_code: String,
    pub amount_minor_units: i64,
}

pub struct TransactionVirtualCurrency {
    pub name: String,
    pub currency_type: VirtualCurrencyType,
    pub amount: i64,
}

pub struct TransactionItem {
    pub name: String,
    pub item_type: String,
    pub amount: i64,
}
```

`TransactionRealCurrency` has a required three-uppercase-letter ISO 4217 code
and a nonnegative `i64` amount in that currency's minor units.
`TransactionVirtualCurrency` has a nonempty name, a
`VirtualCurrencyType`, and a positive `i64` amount. `TransactionItem` has a
nonempty name, a nonempty application-defined type, and a positive `i64`
amount. Empty product collections are valid.

Their SDK field mappings are `RealCurrencyType` and `RealCurrencyAmount`;
`VirtualCurrencyName`, `VirtualCurrencyType`, and `VirtualCurrencyAmount`; and
`ItemName`, `ItemType`, and `ItemAmount`, respectively.

`payment_country`, when present, is exactly two uppercase ASCII letters. As
with currency, Battlement validates ISO 3166-1 alpha-2 syntax rather than
embedding a changing country registry.

Scalar setters map to `TransactionName`, `TransactionType`, `TransactionId`,
`PaymentCountry`, `ProductId`, `StoreItemSkuId`, `StoreItemId`, `StoreId`,
`StoreSourceId`, `TransactionReceipt`, `TransactionReceiptSignature`,
`TransactionServer`, and `TransactorID`. Failed events use the same setters on
`TransactionFailedEvent` plus `FailureReason`.

A receipt signature requires a receipt and `TransactionServer::Google`. A
receipt used for Apple validation requires `TransactionServer::Apple`.
Battlement passes receipt text through as an opaque string and never parses,
logs, or normalizes it.

Unity IAP 4.2 and later automatically forwards successful and failed
transactions to Analytics. Games using that forwarding must not also send the
same transactions through Battlement; doing so double-counts transactions and
inflates reported revenue. Battlement cannot detect this configuration and
therefore cannot deduplicate it.

## Automatic events, buffering, and upload

Analytics 6.3 owns these fixed automatic events:

- `sdkStart`: start of an Analytics session.
- `gameStarted`: start of a session after consent permits collection.
- `clientDevice`: device information at session start.
- `newPlayer`: when consent is newly granted, or during initialization when
  the current user ID differs from the previously recorded user ID.
- `gameRunning`: every 60 seconds when no other event was recorded.
- `gameEnded`: graceful application shutdown, subject to platform shutdown
  time and disk caching.

Battlement does not synthesize, record, rename, or journal these events. The
Analytics consent value controls the whole Analytics stream. There is no client
API to enable or disable one automatic event independently, so Battlement does
not invent one.

The SDK adds common fields such as user ID, session ID, event timestamp, event
UUID, and platform. Rust does not set them. Changing an external user ID starts
a new Analytics identity and may start a new Analytics session; Cloud state
reports the effective values returned by the SDK.

Analytics buffers accepted events in memory and attempts upload every 60
seconds. This interval is not configurable. If upload fails, accepted events
remain in memory; at shutdown, supported platforms can cache up to 5 MiB to
disk for a later session. Events retain the user, session, and common values
from recording time.

`FlushAnalytics` triggers the SDK's immediate upload attempt. Success means
the call was issued, not that the network request or server ingestion
succeeded. It does not clear local buffers, await the Event Browser, or bypass
the consent gate.

## Identity and privacy operations

### External user IDs

`SetAnalyticsExternalUserId` requires a nonempty value after trimming and
limits it to 256 Unicode scalar values. The exact untrimmed value is rejected;
the command never silently changes an identifier. `ClearAnalyticsExternalUserId`
restores Unity's installation ID as the effective Analytics user ID.

Set and clear use one recoverable main-thread transaction:

1. Update `external_user_id` in Battlement's package-owned persistence record.
2. Save the serialized record to PlayerPrefs.
3. Assign the same value, or an empty string for clear, to
   `UnityServices.ExternalUserId`.
4. Refresh Cloud state and emit a correlated `ExternalUserIdChanged` report.

If assignment throws or fails readback, the command fails even when the setter
already mutated its backing value. The executor first attempts to restore and
save the previous persistence record. A normal save makes the prior ID
authoritative; a thrown save leaves the newly saved ID authoritative under the
atomic storage contract and records `Storage`. It then assigns and verifies
whichever ID is authoritative. Failure of that identity compensation takes
precedence as `ExternalUserId`; otherwise the earlier assignment or storage
failure remains. The executor refreshes state, emits correlated
`OperationFailed`, and returns `CloudOperationFailed`. Thus persistence and
Unity identity either agree at return or the reported identity failure makes
their divergence explicit for startup recovery. No event can interleave with
this transaction on the main thread.

The package owns a namespaced key whose contents are not part of the public
protocol. No other Battlement component reads it. Unity Analytics does not
persist external IDs, which is why Battlement does so and restores the value
before every initialization attempt.

The new ID applies only to events recorded afterward. Previously buffered or
uploaded events retain their old user ID. Changing IDs can increase Unity
Monthly Active User counts and affect billing. Games should set a stable ID
before granting consent when they need cross-device identity.

An ID command may execute while UGS is initializing. Its assignment is
linearized on the main thread. If it occurs before Analytics activates, startup
automatic events use the new ID; if activation occurs first, those events use
the previous restored ID and later events use the new one. The correlated state
report makes that boundary observable.

### Privacy URL

`CloudState.privacy_url` is the URL returned by
`AnalyticsService.Instance.PrivacyUrl` after UGS is ready.
`OpenAnalyticsPrivacyUrl` waits for readiness, verifies that the URL is an
absolute `https` URL, and passes it to `Application.OpenURL`. Success means only
that Battlement issued the void API call; it cannot prove that a platform
displayed the page.

On WebGL, the command is valid only when Analytics is already ready and the
current synchronous Rust response was caused by a pointer click or navigation
submit. Waiting for initialization would lose browser user activation, so that
case fails with `CloudOperationFailed`. The runner passes this originating
action context to the executor. Popup policy may still block the void platform
call, which is not synchronously observable.

Rust can query the URL without opening it by sending `ReportAnalyticsState`.
Games may render that value as an ordinary Battlement UI link or button.

### Data deletion

Battlement persists external identity and deletion intent in one serialized
PlayerPrefs record. A deletion intent contains the external ID that was
controlled by Battlement when the request began, or `None` when the installation
ID was effective. The target is required because Analytics 6.3 constructs
`ddnaForgetMe` with the SDK's current user ID.

```rust
pub struct CloudPersistenceRecord {
    pub external_user_id: Option<String>,
    pub deletion_intent: Option<AnalyticsDeletionIntent>,
}

pub struct AnalyticsDeletionIntent {
    pub target_external_user_id: Option<String>,
}
```

Target selection does not require ready-only Analytics state. An existing
intent always keeps its stored target. A new request compares the persistence
record's `external_user_id` with `IServicesBackend.ExternalUserId`, normalizing
null and empty to `None`. Equal values select that external ID or `None` for the
installation ID. Any divergence is an `ExternalUserId` `Configuration` failure
and the command stops before changing consent or persistence. This rule is the
same in `Initializing`, `Ready`, and `Failed`.

`RequestAnalyticsDataDeletion` begins this crash-recoverable, idempotent
transaction on Unity's main thread, regardless of initialization state:

1. Replace the persisted record with `external_user_id: None` and a pending
   deletion intent containing the current target external ID; then save it.
2. Set Analytics consent to `Denied`.
3. Keep, or temporarily restore, the target in
   `UnityServices.ExternalUserId` until the SDK captures the deletion request.
4. Refresh state and emit a correlated `DataDeletionRequested` report.

Persisting intent first ensures a crash at any later point resumes deletion.
On startup, pending intent takes precedence over the normally empty persisted
external ID: the Analytics module denies consent and verifies it, restores and
verifies the target, and only then begins UGS initialization and follows the
same request path.

If the first save throws, its atomic storage contract leaves the prior record
authoritative. Battlement changes neither consent nor the effective external
ID, records an in-memory `Storage` failure, emits a correlated
`OperationFailed` report, and fails the command with
`AnalyticsDataDeletionFailed`. There is no pending deletion to retry.

Once the intent save returns, it is authoritative until every acceptance and
cleanup step succeeds. If denying consent or installing the target throws,
or if either readback does not match, Battlement does not call the deletion
API. It keeps the intent, reports the actual `Consent` or `ExternalUserId`
failure with
`DataDeletionRetryFailed`, and fails the current command with
`AnalyticsDataDeletionFailed`. Startup and an explicit deletion command retry
the privacy mutation before attempting deletion. The pending-deletion gate
blocks manual recording and flush even if the consent setter failed and the
observable consent has not yet become `Denied`.

The successful immediate transaction is atomic at the Battlement boundary:
Rust observes no state between its steps, and its first report has denied
consent, no persisted external ID, and pending intent together. PlayerPrefs
and Developer Data are separate stores, so crash recovery comes from durable
intent rather than a cross-store commit primitive.

The command waits for UGS when an attempt is in progress, uses Analytics
immediately when ready, and verifies by readback that consent is `Denied` and
that `UnityServices.ExternalUserId` still equals the stored target immediately
before calling `RequestDataDeletion`. A mismatch is a `Consent` or
`ExternalUserId` failure and no SDK call occurs. If initialization is already
`Failed`, it retains the privacy mutations and intent but fails with
`CloudInitializationFailed`; Rust must retry initialization. Denying first is
mandatory because Analytics 6.3 throws if deletion is requested while consent
is granted.

The normal return from `RequestDataDeletion` is Unity's acceptance boundary.
The pinned Analytics 6.3 integration has a source-backed regression test proving
that the call synchronously captures the current Analytics user ID before it
returns. Unity's privacy documentation defines the SDK's persisted,
cross-restart retry behavior. Battlement relies on both properties and never
changes the target before this return.

After a normal return, Battlement clears `UnityServices.ExternalUserId` and
then atomically saves a record with no deletion intent. Only after both
operations return does it clear a matching failure, refresh state, emit
`DataDeletionAccepted`, and complete the command. The direct report retains
the command ID; a startup retry report does not.

If clearing the Unity external ID throws or readback is nonempty, Battlement
keeps the persisted intent and uses assign-and-verify to reinstall its stored
target. It reports `ExternalUserId` with `DataDeletionRetryFailed` whether or
not clearing had already mutated the backing value. A failed restoration takes
precedence in the sanitized details. The current command fails, and the next
retry verifies the target, repeats the deletion call, and retries cleanup.
Unity may treat that repeated call as an already pending request; Battlement
still requires a normal return before trying cleanup again.

If saving the cleared intent throws, the atomic storage contract leaves the
pending record authoritative. Battlement uses assign-and-verify to restore its
target into `UnityServices.ExternalUserId` for the next retry. A successful
restoration reports a `Storage` failure; a failed restoration reports the
`ExternalUserId` failure instead. In both cases it emits
`DataDeletionRetryFailed`, fails the command, and does not emit
`DataDeletionAccepted`. The next retry repeats the deletion call before
cleanup. These rules also apply to startup retry, except that its reports omit
an originating command ID.

Completion means Unity accepted a retryable request; it does not mean the
server-side purge has finished. Unity retries the `ddnaForgetMe` request across
network failures and application restarts. Battlement does not clear Unity's
Analytics PlayerPrefs keys.

If initialization or the deletion call fails, Battlement keeps its intent.
After a successful privacy mutation, consent remains denied and the target
external ID remains installed solely so the next retry addresses the same
Analytics identity. A deletion-call failure emits
`DataDeletionRetryFailed` with sanitized details and fails the current command.
An already failed initialization retains its existing initialization report
and fails the command with `CloudInitializationFailed`. Rust explicitly sends
`RetryCloudInitialization`; after readiness, reissuing
`RequestAnalyticsDataDeletion` retries the request.

After successful initialization, a startup pending deletion runs before public
state becomes `Ready` and before waiting recording commands are released. If
the SDK accepts it, Battlement emits `DataDeletionAccepted` and then
`InitializationReady`, both with the final state. If the SDK call throws,
Battlement publishes `Ready` with pending deletion, emits
`DataDeletionRetryFailed`, and then emits `InitializationReady`; waiting record
commands re-check denied consent and succeed as no-ops.

The package checks pending deletion before processing a later consent grant,
external-ID set, or built-in dialog. Those commands fail with
`AnalyticsDataDeletionFailed` until Unity accepts the pending request. This
prevents new collection under an identity awaiting deletion.

## Fake client contract

`battlement-cloud-fake` supplies `CloudFake`, an in-memory implementation of
the complete contract. Its default state is `Ready` with `Granted` consent, a
deterministic session ID, installation ID, and privacy URL. Tests can instead
construct it as absent, initializing, or failed.

`battlement-fake::FakeClient` contains a configured `CloudFake`. An absent
fake omits `Connect.cloud` and returns `ModuleUnavailable`. A present fake
executes first-class Cloud commands through the same validation and consent
gate as Unity.

The public test surface is:

```rust
impl CloudFake {
    pub fn absent() -> Self;
    pub fn initializing(consent: AnalyticsConsent) -> Self;
    pub fn failed(consent: AnalyticsConsent, failure: CloudFailure) -> Self;
    pub fn state(&self) -> Option<&CloudState>;
    pub fn dialog(&self) -> Option<&FakeConsentDialog>;
    pub fn pending_command_ids(&self) -> Vec<CommandId>;
    pub fn accepted_events(&self) -> &[AnalyticsEvent];
    pub fn command_results(&self) -> &[FakeCloudCommandResult];
    pub fn deletion_attempts(&self) -> &[FakeDeletionAttempt];
    pub fn flush_command_ids(&self) -> &[CommandId];
    pub fn opened_privacy_urls(&self) -> &[String];
    pub fn state_reports(&self) -> &[CloudStateReport];
    pub fn complete_initialization(&mut self);
    pub fn fail_initialization(&mut self, failure: CloudFailure);
    pub fn accept_analytics_consent_dialog(&mut self);
    pub fn decline_analytics_consent_dialog(&mut self);
    pub fn accept_next_data_deletion(&mut self);
    pub fn fail_next_data_deletion(&mut self, failure: CloudFailure);
}

pub struct FakeConsentDialog {
    pub title: String,
    pub body: String,
}

pub struct FakeCloudCommandResult {
    pub command_id: CommandId,
    pub outcome: FakeCloudCommandOutcome,
}

pub enum FakeCloudCommandOutcome {
    Completed,
    Recorded,
    Suppressed,
    Failed(CoreErrorCode),
}

pub struct FakeDeletionAttempt {
    pub target_external_user_id: Option<String>,
    pub accepted: bool,
}
```

`Default` constructs the documented ready-and-granted fake. `FakeClient` takes
ownership through `connect_with_cloud(engine, cloud)` and exposes `cloud()` and
`cloud_mut()`. Its `reconnect(engine)` method retains that same owned Cloud
fake while replacing only the Battlement session and engine.

Completion and failure gestures panic when no matching initialization or
dialog operation is active because that is a test-developer error.
Initialization owns pending command values until a completion gesture executes
them in arrival order or a failure gesture completes them with
`CloudInitializationFailed`. A configured deletion outcome applies to one SDK
acceptance attempt and then returns to accept-by-default behavior.

The fake tracks:

- initialization attempts, waiters, terminal failures, and retries;
- Developer Data consent and every consent transition;
- whether the built-in dialog is open and its title and body;
- temporary modal input suppression;
- persisted and effective user IDs and Analytics session ID;
- pending and accepted deletion requests;
- flush requests;
- privacy-URL open calls;
- accepted manual events in normalized order; and
- emitted Cloud state reports with causes and command correlation.

The accepted-event journal contains only events that would reach
`RecordEvent`: valid and granted commands after successful initialization.
Denied commands succeed without accepted-event journal entries. A separate
command-result journal records `command_id` and `Completed`, `Recorded`,
`Suppressed`, or
`Failed(CoreErrorCode)` for every Cloud command. The fake never creates Unity's
automatic events. Normalized event order is the order in which granted pending
commands reach the fake backend, after initialization and consent re-checks.

The fake exposes `accept_analytics_consent_dialog()` and
`decline_analytics_consent_dialog()` gestures. Either gesture requires an open
dialog, updates consent before emitting the state action, closes the dialog,
and releases only the modal input lease. Re-entering a dialog, teardown, and
selecting the current consent value match Unity behavior.

Tests control initialization with explicit complete and fail gestures. A
recording command sent while initializing remains pending until one of those
gestures resolves the attempt. Tests control deletion acceptance or failure
independently so persisted retry behavior can be exercised across fake
reconnections.

Reconnecting a `FakeClient` retains the same `CloudFake` persistence record,
consent, Unity deletion state, and journals while creating a new Battlement
session. Existing report-journal entries remain available for test inspection
but are not delivered to the replacement engine. Constructing a new
`CloudFake` starts fresh. A privacy-open command appends the validated URL to
its journal; it never launches a real browser.

## Logging and diagnostics

Production logs may contain the command kind, command ID, event name, consent
state, initialization state, stable failure category, and Unity error code.
They must not contain custom parameter values, external or
effective user IDs, transaction IDs, receipts, receipt signatures, deletion
identifiers, URLs with query strings, or serialized Cloud messages.
Event names are schema identifiers, but games must still avoid putting user
data into them.

Verbose Services Core logging and the Analytics Debug Panel are developer
tools and are not enabled automatically by Battlement. The separate Cloud
sample documents how to enable them in its non-production environment. Unity
Diagnostics remains outside Analytics consent and outside every Battlement
Cloud command and state field in this design.

## Automated validation

Rust black-box tests cover protocol round trips for every state, command,
action cause, standard-event field, SDK enum, and custom parameter variant.
They assert that `Int32`, `Int64`, `Float32`, `Float64`, Boolean, and timestamp
tags survive JSON with their exact identities. Property tests cover name
syntax, reserved and duplicate parameters, string boundaries, finite numbers,
timestamp boundaries, collection limits, currency records, and nested product
validation.

`battlement-cloud-fake` tests cover absent-module failures, initialization
waiting, failure and explicit retry, consent re-checking after a wait, all
three consent gates, dialog re-entry, unchanged-value selections, state-report
correlation, external-ID restoration, deletion acceptance and retry, flush,
and every complete standard-event conversion. Every manual variant and flush
has an explicit denied-path assertion: completed result, `Suppressed` journal,
and no backend construction or call. `battlement-fake` integration tests prove
the composed fake drives a real Rust engine through public APIs.

C# protocol tests deserialize Rust fixtures and serialize equivalent C# values
for all Cloud variants. They reject invalid tags, out-of-range values, unknown
union variants, missing required fields, duplicate custom parameters, and
oversized strings before calling the backend.

Unity black-box executor tests inject an Analytics backend interface rather
than invoking Unity's network service. They cover initialization ordering,
one shared task, consent changes during a wait, exact event conversion, SDK
acceptance errors, flush, effective identity, privacy URL validation, deletion
ordering, persisted retry, state transition actions, teardown, and the absence
of sensitive values in logs. Converter tests inject the trace and assert
every trace entry before the final `Event` reaches the backend. A pinned source
test covers boxed `CustomEvent.Add` dispatch. Tests expect the final SDK object
to collapse `int` to `long` and `float` to `double`, exactly as Analytics 6.3
does. Because standard SDK events are write-only, their scalar, enum, and
product mapping assertions use the trace rather than backend inspection; the
backend spy asserts the final event subclass and single record call.

Those tests also cover authoritative pre-connect and reconnect snapshots,
revision filtering, stale continuations after module-clone disposal, adoption
of an already initialized Services Core instance, rejection of an unobservable
Services Core initialization in progress, external-ID changes on both sides of
Analytics activation, and setters that mutate identity before throwing. They
cover crash recovery after every deletion persistence step, preservation of the
target deletion identity, retry-before-waiter ordering, deletion acceptance
reports, and operation-specific failure clearing.

Activation tests exercise these distinct configurations:

- without Analytics installed, the base client compiles and cannot serialize
  `BattlementAnalyticsModule`;
- with Analytics installed but its module unselected on `BattlementRunner`,
  `Connect.modules` omits `com.unity.services.analytics`, `Connect.cloud` is
  absent, and Battlement invokes neither Services Core nor Analytics; and
- with `BattlementAnalyticsModule` selected, the runner creates exactly one
  clone and begins exactly one UGS initialization attempt after preparation.

Play Mode tests with Domain Reload disabled prove serialized module selection
does not retain runtime state or initialize UGS before attachment. One test
holds initialization incomplete, tears down its runner, creates a fresh clone,
and then completes the old task. The completion may not change the new clone's
state or persistence, alter Unity identity, or emit an action. The new clone
handles the resulting Services Core state only through the documented
`Uninitialized`, `Initializing`, and `Initialized` paths. Separate tests create
two runners and require the second to throw before connection. Lifecycle tests
disable and re-enable one runner and require a new session, restored persisted
Cloud state, fresh runtime clones, and exact guard release without mutating the
module assets.

Package-manifest tests prove `com.battlement.client` declares no Analytics or
Services Core dependency. Assembly-definition tests pin the Analytics module
assembly's exact `[6.3.0]` version define and matching define constraint,
compile projects with and without Analytics, and verify serialized Analytics
module references preserve their concrete type under Mono and IL2CPP managed
stripping. Generic runner tests cover missing references, duplicate assets,
duplicate module IDs and concrete types, two-phase list-order startup, missing
module requirements, and the absence of SDK calls before attachment.

Battlement UI integration tests prove that the built-in modal is responsive,
traps focus, blocks underlying Rust UI and gameplay input, remains interactive
while gameplay input is disabled, preserves the prior consent until selection,
and restores the exact prior input state. Teardown tests prove that closing an
unanswered dialog does not change consent or leak an input lease.

UI tests also cover long unbroken plain text, safe-area insets, 16:9 and narrow
portrait panels, tab order `Accept` then `Decline`, controller navigation,
screen-reader labels matching visible text, ignored Escape/back/cancel input,
suspension, transport disconnect, and failure during construction or consent
selection.

## Cloud sample

A separate Cloud sample contains one Unity project and one Rust rules crate. It
is linked to a non-production UGS environment and names that environment
visibly so manual events cannot be mistaken for production data. The sample
installs Analytics 6.3 explicitly, which brings its Services Core dependency.
It creates one `BattlementAnalyticsModule` asset and assigns it to
`BattlementRunner.modules`. It contains no game-specific C#.

The sample demonstrates:

- startup and requested Cloud state reports;
- the built-in title-and-body consent dialog;
- a localized custom consent screen built as an ordinary Battlement UI tree;
- unspecified, granted, and denied event recording;
- one custom event using every parameter type;
- acquisition-source, ad-impression, transaction, and failed-transaction
  events;
- external-ID set, clear, restoration, and effective-ID reporting;
- manual flush and offline recovery;
- privacy URL display and open;
- data deletion and persisted retry; and
- a deterministically injected initialization failure followed by explicit
  Rust retry.

The custom event schemas are created and enabled in the sample environment's
Event Manager. Sample documentation includes the Unity IAP double-reporting
warning and the legal limitations of both consent presentations.

The Analytics module assembly supplies an Editor-only Cloud QA window. Its reset
control refuses to run while deletion is pending, sets Analytics consent to
`Unspecified`, clears the Battlement persistence record, and assigns an empty
external ID. It does not delete Unity's Analytics retry or disk-cache keys.
Before entering Play Mode, the window can select the injected backends and
configure exactly one initialization or deletion call to fail with a chosen
stable category. This deterministic harness is package-owned sample and test
tooling, not a runtime Cloud command or game-specific C# setup.

## Manual QA

Use the separate Cloud sample and a non-production UGS environment. Run live
SDK checks in the sample and conversion/failure checks in the injected-backend
Play Mode harness when a step says so. In Editor checks, open the Analytics
Debug Panel before recording; it does not show earlier events. Give the run a
unique event-name suffix, filter Event Browser by that name and current user
ID, and inspect `sessionID` in raw payloads. Allow up to 15 minutes for
ingestion before recording a delayed result rather than a failure.

1. Confirm the sample has Analytics 6.3 installed and its runner lists one
   `BattlementAnalyticsModule` asset. Run the sample reset menu, then enter Play
   Mode. Confirm `Connect.modules` reports `com.unity.services.analytics` but
   not Services Core, `Connect.cloud` is present, consent is `Unspecified`, and
   initialization is `Initializing` or `Ready`. In the injected harness, hold
   the initialization barrier until after connection and then release it;
   confirm an `Initializing` connection snapshot followed by an
   `InitializationReady` action. Confirm the Debug Panel and Event Browser show
   the sample's selected non-production environment.
2. Record each manual event while consent is `Unspecified`. Confirm every
   command fails with `AnalyticsConsentRequired` and no event enters the Debug
   Panel. In the injected harness, confirm no backend event call occurs.
3. Show the built-in dialog with sample title and body. Confirm the command
   completes while the dialog remains open, the prior consent stays active,
   gameplay and underlying Rust UI are blocked, focus remains in the modal,
   and a second show command fails with `AnalyticsConsentDialogOpen`.
4. Select `Decline`. Confirm no exception is thrown, consent becomes `Denied`
   before the modal closes, input returns to its exact prior state, and one
   `ConsentDialogSelection` state action is emitted. Record valid custom and
   standard events and flush; confirm all succeed as silent no-ops.
5. Reopen the built-in dialog while denied, select `Decline` again, and confirm
   another state action is emitted even though the value did not change.
6. Reopen and select `Accept`. Confirm consent becomes `Granted`; then record a
   custom event containing string, `i32`, `i64`, `f32`, `f64`, Boolean, and UTC
   timestamp parameters. Confirm the Debug Panel raw JSON shows the expected
   integer and floating values. In the injected conversion trace, confirm the
   exact boxed `Int32`, `Int64`, `Float32`, and `Float64` inputs to
   `CustomEvent.Add`. Confirm Event Browser later shows the event in the
   selected environment.
7. Record complete acquisition-source, ad-impression, transaction, and failed
   transaction events. Confirm every optional field and product collection
   appears in the injected conversion trace with the expected SDK property,
   collection, enum member, and boxed value. Confirm the backend receives the
   correct write-only SDK event subclass exactly once. Confirm representative
   fields in Debug Panel raw JSON. Do not enable Unity IAP forwarding for this
   check.
8. Disable network access in the Editor, record events while granted, and call
   flush. Confirm command success does not claim upload success and the Debug
   Panel shows the attempted local events. Exit Play Mode normally so the SDK
   can cache, re-enter while still offline, then restore network and flush.
   Confirm Event Browser raw payloads retain the original `sessionID` and user
   ID.
9. Set an external user ID, confirm both persisted and effective IDs, and
   restart the player. Confirm the persisted value is restored before
   initialization. Clear it and confirm later events use Unity's installation
   ID while earlier events keep the old ID.
10. Use the custom Battlement UI consent screen to deny and grant. Confirm its
    tree remains Rust-owned, its buttons send `SetAnalyticsConsent`, and its
    lifecycle is controlled only by ordinary UI commands.
11. In the Cloud QA window, select injected backends and configure the next
    initialization call to fail immediately with `Network`. Enter Play Mode and
    confirm a sanitized `InitializationFailed` report, required-operation
    failures, and no implicit retry. Leave the one-shot backend at its default
    success outcome, send `RetryCloudInitialization`, confirm `RetryStarted`,
    and confirm a terminal `InitializationReady` report.
12. In the injected harness, configure the next deletion call to fail. Request
    data deletion while granted. Confirm consent is denied, the persisted
    external ID is cleared, deletion intent is saved, the state report precedes
    the failing SDK call, and the target external ID remains installed. Restart
    with the default accept outcome and confirm retry occurs after
    initialization. Confirm completion means accepted request, not completed
    server purge.
13. Remove the Analytics module asset from the runner while leaving Analytics
    installed, then run the same Rust rules. Confirm `Connect.modules` omits
    `com.unity.services.analytics`, `Connect.cloud` is absent, Battlement does
    not call `UnityServices.InitializeAsync`, the game otherwise starts
    normally, and every Cloud command fails with `ModuleUnavailable`. Restore
    the asset, then remove Analytics. Confirm the base client still compiles,
    the missing module reference produces a configuration warning, and
    Battlement still does not initialize UGS.
14. Create a second active `BattlementRunner`. Confirm it throws before it can
    connect and that the original runner remains usable. Destroy the original,
    create one replacement, and confirm module runtime and modal resources were
    released.
15. Build and run WebGL. Repeat built-in consent, custom consent, granted and
    denied recording, persisted consent across reloads, offline buffering
    followed by network restoration in the same page,
    external-ID restoration, privacy URL opening from a direct user gesture,
    data-deletion acceptance, and Cloud absence in separate module-present and
    module-absent builds. Use current Chrome and Safari, clear site data
    before the first run, and confirm the privacy command reports only that
    `Application.OpenURL` was issued; popup blocking remains platform behavior.
16. In Unity's Debug Panel, confirm accepted local events and payload types. In
    Event Manager, deliberately disable or mismatch a custom schema and confirm
    Battlement still reports local SDK acceptance while Unity later marks the
    event invalid. Use Event Browser to verify enabled, matching events after
    ingestion delay.
