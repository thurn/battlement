# Application environment

`App` provides `ApplicationState` automatically, seeded from `Connect` and
updated by Unity's focus and pause observations. Components read it with
`use_application_state()` from the Reactant prelude. Updates reach memoized
consumers and preserve the session and component state. An isolated preview can
nest `application::provider(state)` with controlled observations.

`use_viewport_size()` returns physical screen dimensions, seeded from the current
connection and updated by geometry observations. Use `use_geometry(element_ref)`
for logical element measurements inside containers; panel scaling can make those
differ from physical screen pixels.

For a specialized `runtime::Reactant` integration, supply an application provider
and forward host observations yourself. Reading application hooks without the
corresponding provider is a developer error.

`is_active()` is true when focused and not paused. This is an application
activity signal, not a measurement of desktop window occlusion. Applications
can use it to suspend audio or effects while inactive. The platform signals
come from Unity's [focus callback](https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnApplicationFocus.html)
and [pause callback](https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnApplicationPause.html).

## External links

`Command::open_external_url(url)` creates a typed
`CommandBody::ApplicationOpenUrl` request. Issue it from the link's normal
activation callback with a captured application handle:

```rust
let app = use_app();
Button::new("Documentation").on_click(move || {
    app.send(Command::open_external_url("https://docs.unity3d.com/"));
})
```

The handle queues commands after the UI commit and preserves the originating
action's attribution. Asset-bearing commands automatically prepare their
dependencies before execution.
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
