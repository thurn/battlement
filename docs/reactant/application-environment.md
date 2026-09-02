# Application environment

`battlement::application::ApplicationState` carries Unity's `focused` and
`paused` observations. `Connect.application_state` seeds the engine's model.
`ActionBody::ApplicationStateChanged` delivers subsequent changes, including
while ordinary input is disabled. Duplicate observations are coalesced.
Suspending and resuming preserves the session and component state.

The engine stores each observation in its model and refreshes Reactant. Provide
that model value around the application:

```rust
application::provider(game.application_state).child(Application { /* props */ })
```

Components read the nearest provider with
`application::use_application_state()`. Context changes reach memoized
consumers. An isolated preview can nest another provider with controlled
observations. Reading without a provider is a developer error.

`is_active()` is true when focused and not paused. This is an application
activity signal, not a measurement of desktop window occlusion. Applications
can use it to suspend audio or effects while inactive. The platform signals
come from Unity's [focus callback](https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnApplicationFocus.html)
and [pause callback](https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnApplicationPause.html).

## External links

`Command::open_external_url(url)` creates a typed
`CommandBody::ApplicationOpenUrl` request. Issue it from the link's normal
activation callback, using the engine's ordinary response command queue.
The accessibility layer declares the link and its activation; it does not
perform platform work itself.

The Unity host validates that the URL is absolute and dispatches it through
[Application.OpenURL](https://docs.unity3d.com/6000.5/Documentation/ScriptReference/Application.OpenURL.html).
Completion acknowledges dispatch to the platform handler, not page loading or
browser success. Hosts can supply `BattlementRunnerOptions.openExternalUrl`
to integrate another handler. Tests supply a recording handler without opening
an external application. The fake client retains the request in its ordinary
executed-command journal.

The Reactant Layout Gallery displays application activity and provides an
**Open Unity documentation** link. Public context tests cover activity changes
and nested preview isolation. Host tests cover lifecycle delivery with input
disabled, resume without session restart, and external URL command dispatch.
