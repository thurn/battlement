# Reactant Animations

Reactant Animations is the animation, gesture, and layout-projection system for
Reactant user interfaces. It gives every Reactant host façade a Motion-inspired
builder API while preserving Reactant's Rust component model and Unity UI
Toolkit host tree.

The authoring goal is mechanical familiarity. A typical component written with
Motion for React should translate into Rust one expression at a time. `Button`,
`View`, `Label`, and the other Reactant façades own their animation
builders and motion state directly.

The execution goal is 60 Hz motion in native macOS and desktop WebGL players,
as measured by the on-demand checks in
[Performance requirements](#performance-requirements). Rust declares animation
state. Unity recognizes gestures, evaluates motion-value graphs, samples
timelines, projects layout, and applies final UI Toolkit values on every
rendered frame. Animation does not require a Rust render or a network exchange
per frame.

## Related information

- [Reactant technical design](reactant-technical-design.md) defines sessions,
  commits, snapshots, and the Rust-to-Unity boundary extended here.
- [Components and rendering](component-authoring.md) defines host façades,
  sealed render values, keys, and the focused prelude.
- [Reactant host façades](host-facades.md) defines host ownership, private
  lowering, method-order independence, and the `Ui` protocol-type boundary.
- [Hooks and effects](hooks-and-effects.md) defines positional hooks and stable
  hook-owned handles.
- [Reconciliation, events, and portals](reconciliation-events-and-portals.md)
  defines logical identity, host identity, event ordering, and portals.
- [Refs and geometry](refs-geometry-and-floating-ui.md) defines `ElementRef`,
  asynchronous geometry, and panel ownership.
- [Generated UI assets](asset-generator.md) defines an optional source of
  immutable textures. Reactant Animations also works with ordinary prepared
  textures, styles, custom geometry, and shaders.
- [Motion for React][motion-react] is the naming and behavioral reference for
  targets, transitions, gestures, presence, layout, and motion values.
- [Motion layout animation][motion-layout] explains the projection behavior
  adapted to UI Toolkit.
- The [settings mockup][settings-mockup] at commit
  `2451ea9cc6f76b356b1102ee37b82c478853122a` is the immutable acceptance
  fixture used by this design's translation ledger.
- [Unity UI Toolkit transitions][unity-transitions] describes the native
  transition facility that Reactant may use as an internal optimization.
- [Unity visual element scheduler][unity-scheduler] describes Unity's
  panel-attached scheduler. Reactant uses it for delayed element work, not as
  its frame driver.
- [Unity PlayerLoop][unity-player-loop] defines the pinned pre-layout and
  post-layout integration points used by `MotionWorld`.

[motion-react]: https://motion.dev/docs/react
[motion-layout]: https://motion.dev/docs/react-layout-animations
[settings-mockup]:
  https://github.com/thurn/mockups/tree/2451ea9cc6f76b356b1102ee37b82c478853122a
[unity-transitions]:
  https://docs.unity3d.com/Manual/UIE-Transitions.html
[unity-scheduler]:
  https://docs.unity3d.com/6000.0/Documentation/ScriptReference/UIElements.IVisualElementScheduler.html
[unity-player-loop]:
  https://github.com/Unity-Technologies/UnityCsReference/blob/9d487cab41b00c50af020b56d27a3c768d54f770/Runtime/Export/PlayerLoop/PlayerLoop.bindings.cs

## Requirements

Reactant Animations must provide one coherent contract for all UI-relevant
Motion families:

- initial, animate, and exit targets;
- named and computed variants with propagation and orchestration;
- tween, spring, inertia, keyframe, and discrete interpolation;
- hover, tap, focus, pan, drag, and in-view gesture states and callbacks;
- presence modes and manual removal holds;
- layout and shared-layout projection;
- drag constraints, momentum, direction locking, and reorder;
- motion values, transforms, springs, velocity, time, and scroll values;
- declarative CSS-style transitions and reusable keyframe animations;
- stable imperative controls, scoped selectors, and sequences;
- animation lifecycle callbacks and explicit value subscriptions; and
- system and application reduced-motion policies.

The release targets are native macOS desktop and desktop WebGL only. Mobile is
an architectural constraint: the frame loop may not require threads, per-frame
managed allocation, desktop-only shaders, or mouse-only input. Touch semantics
remain defined and the runtime must remain suitable for mobile-class CPU and
memory budgets, but there is no mobile player, device profile, performance
gate, or manual QA target.

The contract covers UI hosts and typed UI values. Browser-only DOM mutation,
arbitrary CSS selectors, SVG path machinery, and runtime CSS string parsing are
not part of Reactant. Equivalent UI effects use typed values, decoration
layers, prepared textures, or a purpose-built Unity rendering implementation.

Motion APIs whose purpose is to run JavaScript on every browser frame become
Unity-local graphs rather than Rust frame callbacks. `use_time`, motion-value
expressions, and explicit coalesced subscriptions cover those cases.
`LazyMotion` has no counterpart because Rust and Unity animation code is linked
at build time. Browser view transitions have no counterpart; Reactant presence
and shared layout animate the live UI Toolkit hosts instead.

The compatibility baseline is exactly Motion `13.1.1`, the version installed by
the pinned mockup. A later Motion release does not change this contract until
the design explicitly adopts it. The supported core is:

| Motion family | Reactant contract |
|---|---|
| `motion.*` | Motion builders on hosts and explicitly forwarding components |
| targets and variants | Typed styles, keyframes, names, and custom data |
| transitions | Tween, spring, inertia, repeats, and orchestration |
| gestures | Hover, tap, focus, pan, drag, in-view, and reorder |
| presence | `Sync`, `Wait`, `PopLayout`, exit holds, and presence hooks |
| layout | Position, size, shared layout, groups, and scroll roots |
| motion values | Values, transforms, springs, velocity, time, and scroll |
| imperative animation | Controls, typed scopes, sequences, and playback |
| accessibility | Inherited and application-observable reduced motion |
| CSS animation | Typed transitions, pseudo-styles, and reusable keyframes |

Reactant does not guess which host belongs to an arbitrary component. A
component may render zero, one, or many hosts, so implicit selection would make
a structural refactor change behavior. A component may instead implement the
explicit forwarding contract in
[Component forwarding](#component-forwarding). DOM selectors and mutation,
SVG path animation, runtime CSS strings, `useAnimationFrame` callbacks,
`AnimateActivity`, `AnimateView`, and Motion+ components are outside the
compatible core. Their absence is compile-time API absence, not a runtime
fallback.

The pinned settings mockup is the minimum feature bar. Its animation patterns
are listed in
[Mockup translation coverage](#mockup-translation-coverage). The design also
covers common Motion APIs that the mockup does not happen to use, so application
code does not need another animation subsystem as it grows.

## Authoring model

### Host façades own animation builders

Every Reactant host façade exposes the complete Motion builder
surface. Motion configuration is private façade state and lowers with ordinary
properties, children, handlers, keys, refs, and portal targets into the same
host node.

```rust
Button::new(trox::tx("Settings", "User-facing copy in this example."))
    .animate(StyleTarget::new().y(0.0).scale(1.0))
    .while_hover(StyleTarget::new().y(-1.0))
    .while_tap(StyleTarget::new().scale(0.955))
    .transition(
        Transition::spring()
            .stiffness(520.0)
            .damping(32.0)
            .mass(0.7),
    )
```

Motion builders may be interleaved with every other valid host method. Lowering
creates no wrapper Unity UI Toolkit `VisualElement`, changes no physical
parent, and consumes no additional logical sibling position. Generic variant
and custom-data types may change the inferred façade specialization, but every
specialization retains the complete ordinary and Reactant host API.

```rust
View::new()
    .child(Label::new(trox::tx("Settings", "User-facing copy in this example.")))
    .animate(StyleTarget::new().opacity(1.0))
    .style(panel_style())
    .element_ref(panel_ref)
    .on_pointer_down(handle_pointer_down)
    .exit(StyleTarget::new().opacity(0.0))
    .key("settings-panel")
```

Host-level method order is not observable. Fluent ordering remains meaningful
inside nested values such as `MotionTarget`, `Transition`, and keyframe
builders; completing one of those values does not restrict later host calls.

Unity may attach private paint resources such as decoration meshes to implement
a requested property. Those resources are not Reactant hosts and do not change
logical hierarchy, physical parenting, input, layout, or focus.

There are no `MotionButton`, `MotionVisualElement`, `MotionHost`, or
`Motion::new` APIs. Reactant defines no animation macro. Ordinary Rust types,
closures, enums, nested builders, and the host façades provide the complete
authoring surface. `NoVariant` is a sealed zero-sized marker used until a
variant map establishes the application's name and custom-data types.

### Component forwarding

Custom components can opt into the same Motion builder surface without adding
a Unity wrapper. **Motion forwarding** means that the component explicitly
accepts a complete `MotionProps` value and applies it to one host selected by
the component author.

```rust
pub trait MotionComponent: Component + Sized {
    fn with_motion(self, motion: MotionProps) -> Self;
}
```

`MotionComponentExt` is implemented for `MotionComponent` values and collects
the same targets, transitions, gestures, variants, layout settings, and
callbacks as host façades. Before rendering, its Rust-only adapter calls
`with_motion`. The component must forward that value unchanged to exactly one
stable host façade in every render branch. Applying forwarded props does not
restrict later methods on that façade. The host is eligible for a property only
when its property-catalog capability accepts the requested value shape.

Failure to forward, forwarding to multiple hosts, or changing the selected host
without changing component identity is a developer error detected while
lowering. The adapter contributes no logical position, physical parent, layout
box, event target, or focus target. Components that cannot promise one stable
host expose their own narrower builders instead of implementing
`MotionComponent`.

```rust
SettingsCard::new(settings)
    .animate(StyleTarget::new().opacity(1.0).y(0.0))
    .exit(StyleTarget::new().opacity(0.0).y(-8.0))
```

### Motion targets

`StyleTarget` contains optional typed presentation values for animation. A
missing property does not participate in that target layer. `MotionTarget`
combines a style with an optional transition and orchestration metadata.

```rust
pub struct StyleTarget { /* private fields */ }
pub struct MotionTarget { /* private fields */ }
pub enum InitialTarget<Name = NoVariant> {
    Target(MotionTarget),
    Variant(Name),
    Disabled,
}

impl MotionTarget {
    pub fn new(style: StyleTarget) -> Self;
    pub fn transition(self, value: Transition) -> Self;
    pub fn transition_end(self, value: StyleTarget) -> Self;
}
```

`animate`, `exit`, and gesture target builders accept
`impl Into<MotionTarget>`, and `StyleTarget` converts directly. `initial`
accepts `impl InitialValue`, a sealed input implemented only for `bool`,
`StyleTarget`, and `MotionTarget`. The distinct `initial_variant(name)` builder
selects `InitialTarget::Variant`, avoiding overlapping generic
`From<MotionTarget>` and
`From<Name>` implementations. `false` selects `Disabled`; `true` is a developer
error because it has no Motion meaning. Application variant-name types never
implement `InitialValue`.

Every base host starts with the sealed `NoVariant` name type. Calling
`.variants(...)` changes the façade's name type while preserving a previously
selected disabled or concrete initial target. This gives `.initial(false)` a
known type even when it appears before `.variants(...)` and prevents an
unconstrained generic at the call site.

```rust
View::new()
    .initial(StyleTarget::new().opacity(0.0).x(-17.0))
    .animate(StyleTarget::new().opacity(1.0).x(0.0))
    .exit(
        MotionTarget::new(StyleTarget::new().opacity(0.0).x(10.0))
            .transition(Transition::tween().duration_secs(0.15)),
    )
```

`.initial(false)` suppresses the initial transition and starts at the current
animate target. An omitted `initial` target starts from the committed static
style or the current presentation value when an existing host is retargeted.

`transition_end` applies its values atomically after successful completion. It
does not run after cancellation and bypasses `StyleTransition`; otherwise the
supposedly terminal assignment would begin another animation. A new target uses
those values as part of its static presentation baseline.

### Transition types

`Transition` is one builder with tween, spring, inertia, and immediate
constructors. It carries a default transition and optional property overrides.

```rust
let transition = Transition::spring()
    .stiffness(400.0)
    .damping(30.0)
    .property(
        MotionProperty::Opacity,
        Transition::tween()
            .duration_secs(0.18)
            .ease(Easing::EaseOut),
    )
    .property(
        MotionProperty::Layout,
        Transition::spring().stiffness(500.0).damping(38.0),
    );
```

A tween supports duration, delay, easing, repeat, repeat delay, repeat type,
and keyframe times. Easing is typed:

```rust
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    CubicBezier([f32; 4]),
    Steps { count: u32, position: StepPosition },
}
```

`Easing::CubicBezier` validates that the two x coordinates are in `0..=1`.
`.ease(value)` applies one easing to every segment. `.easings(values)` supplies
one easing per segment, matching Motion's easing-array form. A missing array
entry makes that segment linear, and entries beyond the segment count are
ignored, matching Motion 13.1.1.

Transition-level keyframe `times` must begin at zero, end at one, and be
nondecreasing. Equal adjacent values create a zero-duration segment that takes
the later keyframe at that boundary. Unity applies times independently to each
property whose keyframe count equals the number of times. A property with a
different count ignores the transition-level times and distributes its own
keyframes evenly. This is the Motion fallback used by the modal mockup, whose
four-entry transition times are
shared by a three-keyframe exit target. Explicit property-local
`Keyframes::times` must match that property's count and fail validation when it
does not.

A spring supports stiffness, damping, mass, initial velocity, rest speed, rest
delta, duration, and bounce. Physical parameters and duration/bounce are two
exclusive configuration forms. Mixing them panics during render validation.
Reactant uses the closed-form numerical contract below.

An inertia transition supports initial velocity, power, time constant, minimum,
maximum, bounce stiffness, bounce damping, and a serializable target modifier.
`InertiaTarget` supports identity, nearest multiple, floor multiple, ceiling
multiple, and clamp. The common Motion `modifyTarget` closure translates to a
typed modifier because an arbitrary Rust closure cannot execute in Unity.

```rust
Transition::inertia()
    .power(0.8)
    .target(InertiaTarget::nearest_multiple(50.0))
```

Inertia is used by drag momentum and may animate numeric motion values directly.

`Repeat::Count(n)` means `n` additional iterations after the first.
`Repeat::Forever` has no terminal completion. `RepeatType` is `Loop`, `Reverse`,
or `Mirror`. Reverse plays the sampled timeline backward; mirror swaps each
property's origin and target before reevaluating its transition.

Negative delay begins at the corresponding elapsed point. A negative delay
whose magnitude exceeds one finite iteration advances through repeats and
repeat delays. If it exceeds a finite animation's total duration, the animation
commits its final value and completion boundary without showing an intermediate
sample.

### Numerical contract

The mockup's installed Motion 13.1.1 sampler is the compatibility baseline.
Reactant freezes its relevant defaults and equations here so a later Motion
release cannot silently change application behavior.

An unspecified transition uses these property defaults, in order. The
more-than-two-keyframes rule takes precedence over the property-specific rules:

- more than two keyframes use a `0.8` second tween;
- non-transform values use a `0.3` second tween with cubic Bézier
  `[0.25, 0.1, 0.35, 1]`;
- translate and rotate use a spring with stiffness `500`, damping `25`, and
  rest speed `10`; and
- scale uses stiffness `550`, damping `30`, and rest speed `10`, except a scale
  target of zero uses critical damping `2 * sqrt(550)`.

An explicit `Transition::tween()` defaults to `0.3` seconds and `EaseInOut`.
Its normalized scalar sample is the cubic Bézier or step result evaluated in
`f64`, clamped to the segment domain, then mixed into the property value.

A physical spring solves:

```text
mass * position'' + damping * position'
    + stiffness * (position - target) = 0
```

`Transition::spring()` defaults to stiffness `100`, damping `10`, mass `1`, and
the incoming presentation velocity. Reactant evaluates the closed-form
underdamped, critically damped, or overdamped solution in `f64`; it does not use
frame-dependent Euler integration. A spring completes when both absolute
velocity and target delta meet their thresholds. For an initial delta below
five units, defaults are `0.01` units per second and `0.005` units. Otherwise
they are `2` units per second and `0.5` units. Completion emits the exact
target.

A duration spring defaults to `0.8` seconds and bounce `0.3`, ignores incoming
velocity, clamps duration to `0.01..=10` seconds, and sets damping ratio to
`clamp(1 - bounce, 0.05, 1)`. It starts the undamped angular frequency at
`5 / duration` and performs exactly 11 `f64` Newton-Raphson iterations against
Motion 13.1.1's `0.001` displacement envelope. A non-finite result falls back
to stiffness `100` and damping `10`. The resulting stiffness is
`frequency² * mass`; damping is
`2 * ratio * sqrt(mass * stiffness)`. These constants, iteration count, and
fallback are normative and become checked-in conformance vectors for Reactant's
normative Unity sampler. A
visual-duration spring uses angular root
`2 * PI / (1.2 * visual_duration)` and derives stiffness and damping from that
root and the clamped bounce ratio.

Inertia defaults to power `0.8`, time constant `325` milliseconds, rest delta
`0.5`, bounce stiffness `500`, and bounce damping `10`. Before constraints, its
sample is:

```text
amplitude = modified_target(origin + power * velocity) - origin
delta(t) = -amplitude * exp(-t / time_constant)
value(t) = origin + amplitude + delta(t)
```

Crossing a minimum or maximum starts the physical boundary spring from the
crossing value and derived velocity. Mirror repeats reverse keyframes and
negate initial velocity. Reverse repeats reverse logical time. Seek evaluates
the same closed-form generators and never advances them incrementally.

All spring and inertia generators are pure functions of origin, target,
incoming velocity, options, and elapsed logical time. Neither controlled nor
production sampling advances their physical state incrementally. Unity stores
logical time in integer microseconds, computes generators in
`f64`, and converts final scalar channels to `f32` only when applying UI Toolkit
values. Conformance vectors cover every damping regime, duration springs,
inertia with and without bounds, repeats, negative delay, interruption, and
seek. At named sample times, scalar and velocity values must differ from the
checked-in Motion 13.1.1 vectors by at most `1e-5`; terminal boundary identities
and times must match exactly.

### Property keyframes

Target properties accept a scalar, a `MotionValue`, or typed keyframes. Rust
method names ending in `_value` bind a motion value because Rust has no method
overloading.

```rust
let opacity = Keyframes::new([0.0, 0.38, 0.22, 0.0])
    .times([0.0, 0.1, 0.72, 1.0]);

View::new()
    .animate(
        StyleTarget::new()
            .y(1_000.0)
            .opacity_keyframes(opacity),
    )
    .transition(
        Transition::tween()
            .duration_secs(0.42)
            .ease(Easing::Linear),
    )
```

Each property may have a different number of keyframes when it provides its own
`times`. A property without local times uses the transition's times only when
the counts match. Otherwise, its keyframes are distributed evenly. A
single-value keyframe sequence is a constant target. An empty sequence panics.

Property keyframes are interruptible. Retargeting samples the current
presentation value and velocity before creating the new track. A spring adopts
the prior velocity in compatible units unless the new transition explicitly
sets one. A tween starts from the current value but does not synthesize spring
velocity.

### Typed CSS-style transitions

`StyleTransition` provides the CSS transition model for ordinary static style
changes. It is useful when application state or a pseudo-state changes a style
and the author does not need Motion variants.

```rust
Button::new(trox::tx("Delete", "User-facing copy in this example."))
    .style(Style::new().background_color(Color::hex("#15121a")))
    .hover_style(Style::new().background_color(Color::hex("#2a121c")))
    .style_transition(
        StyleTransition::new()
            .property(
                StyleProperty::BackgroundColor,
                Transition::tween()
                    .duration_secs(0.18)
                    .ease(Easing::EaseOut),
            ),
    )
```

The transition observes changes to the resolved static style from Reactant
renders and supported UI pseudo-states. It never observes sampled Motion output
or `transition_end` assignments.

`hover_style`, `focus_style`, `active_style`, and `disabled_style` are ordinary
typed host-property builders and may be interleaved with children, events, and
Motion methods. When several states match, their declared styles merge in this
fixed low-to-high precedence: hover, focus, active, then disabled. Each state
replaces only properties it declares. Repeated builders for one state merge in
call order, with the later value winning for an overlapping property. The
resolved pseudo-style is part of the static baseline below Motion gesture
layers. Unity recognizes and resolves these states locally.

The public builder assigns a complete tween to each property. It has no
last-selected-property state:

```rust
impl StyleTransition {
    pub fn new() -> Self;
    pub fn property(
        self,
        property: StyleProperty,
        transition: Transition,
    ) -> Self;
    pub fn all(self, transition: Transition) -> Self;
    pub fn allow_discrete(self, value: bool) -> Self;
}
```

Only tween and immediate transitions are accepted; spring or inertia panics
during render validation. Repeating `property` replaces that property's entry.
`all` supplies a default for changed properties without an explicit entry,
regardless of builder call order. It expands at validation time to every
changed interpolable property and never includes discrete properties
implicitly.

Discrete transitions require `.allow_discrete(true)`. They switch at 50 percent
of the active interval. `Display::None` and hidden visibility switch at the end
when disappearing and at the start when appearing, matching their useful CSS
behavior. Without `allow_discrete`, a discrete static change applies
immediately.

### Reusable CSS-style animations

`Keyframes<StyleTarget>` is an ordinary cloneable Rust value. `Animation`
applies it with CSS-style playback settings. There is no global string registry
and no runtime lookup by animation name.

Keyframes normalize independently per property. Interior frames that omit a
property do not create a sample for it; interpolation spans the nearest earlier
and later frames that declare it. If the first or last declared frame is not at
zero or one, Reactant inserts the resolved underlying property value at that
endpoint. The underlying value is captured from lower-priority layers when the
animation generation is installed. A property omitted from every frame creates
no track. This matches the CSS keyframe rule without turning a missing
`StyleTarget` field into an implicit hold.

```rust
fn grid_breathe() -> Keyframes<StyleTarget> {
    Keyframes::new([
        StyleTarget::new().y(-6.0).scale_y(0.96).opacity(0.58),
        StyleTarget::new().y(12.0).scale_y(1.02).opacity(1.0),
    ])
}

View::new()
    .animation(
        Animation::new(grid_breathe())
            .duration_secs(5.2)
            .ease(Easing::EaseInOut)
            .iterations(AnimationIterations::Forever)
            .direction(AnimationDirection::Alternate)
            .fill(AnimationFill::Both)
            .play_state(AnimationPlayState::Running)
            .diagnostic_name("arcade-grid-breathe"),
    )
```

`Animation` supports duration, delay, one easing or segment easings, iteration
count, direction, fill, composition, and play state. Changing only play state
preserves elapsed time. Changing keyframes, duration, delay, easing, iteration
count, direction, fill, or composition creates a new slot generation and
restarts it. Changing the stable animation key deliberately does the same.

CSS and Motion count repetitions differently. `Repeat::Count(n)` continues to
mean `n` additional Motion iterations after the first.
`AnimationIterations::Count(n)` means exactly `n` total CSS-style plays,
`Once` means one play, and `Forever` has no terminal completion. A count of zero
installs no track; delay, direction, fill, and lifecycle callbacks have no
effect.

`Paused` installs fill output but does not advance logical time. `Running`
resumes a paused generation. Removing an animation cancels it after its last
subscribed update. A finite animation remains at the value selected by its fill
mode after completion; an unfilled property immediately reveals the next lower
owner.

The diagnostic name appears in traces and developer diagnostics only. It does
not establish identity. List position establishes animation identity across
renders. Dynamic lists use `.animation_key(key)` on each `Animation`, with the
same owned `Eq + Hash + Clone + 'static` key rules as Reactant children.

A host may receive an ordered list with `.animations(iterator)`. If multiple
animations target one property, the later animation wins by default.
`AnimationComposition::Add` or `Accumulate` is available for numeric values and
transforms with defined addition. Validation rejects additive composition for
colors, assets, clips, filters, and other values without a stable additive
operation.

`Replace` emits the animation's sampled value. `Add` combines that sample with
the lower resolved layer every frame. `Accumulate` additionally combines one
completed-iteration delta, `target - origin`, for each completed iteration
before sampling the current iteration. Numeric values, compatible lengths,
translation, rotation, and skew add. Scale multiplies, using one as its
identity. Transform components are combined independently and emitted in the
canonical alias order. An authored transform list is additive only when both
lists have compatible operations; every other combination fails validation.

### Decoration layers

CSS pseudo-elements in the mockup become typed decoration layers. A decoration
is paint associated with one host, not a logical child. It is excluded from
focus, accessibility, picking, event propagation, and layout by construction.

```rust
Button::new(trox::tx("Upgrade", "User-facing copy in this example."))
    .before(
        Decoration::new()
            .position(DecorationPosition::Fill)
            .background(premium_shine_gradient())
            .animation(
                Animation::new(shine_sweep())
                    .duration_secs(2.4)
                    .iterations(AnimationIterations::Forever),
            ),
    )
    .after(
        Decoration::new()
            .position(DecorationPosition::Border)
            .render_asset(premium_border_asset()),
    )
```

`before` paints behind host content but above the host background. `after`
paints above content. Both remain inside the host's clip. A decoration may opt
into `.overflow(DecorationOverflow::Visible)` only when the host and panel
permit overflow.

`.before_all(iterator)` and `.after_all(iterator)` attach ordered decoration
lists for particles, grid lines, and similar effects. List position identifies
a stable decoration by default. `.key(value)` gives a dynamic decoration the
same owned key identity guarantees as a Reactant child.

Decorations support `Style`, `StyleTarget`, `StyleTransition`, `Animation`, and
motion-value bindings. They do not support children, refs, handlers, focus, or
layout projection. Generated textures are valid backgrounds. Their texture
selection is discrete, while opacity, transform, tint, clip, and other host
properties remain interpolable.

Use an ordinary child when an effect needs independent accessibility, input,
layout, a ref, or nested content. Decoration layers are convenience and safety,
not a separate renderer.

## Variants and orchestration

### Typed variant maps

Variant names are ordinary application enums or other owned values. A map is
generic over its name and custom-data types.

```rust
#[derive(Clone, Copy, Eq, Hash)]
enum TabVariant {
    Enter,
    Center,
    Exit,
}

let variants = Variants::<TabVariant, i32>::new()
    .resolver(TabVariant::Enter, |direction| {
        MotionTarget::new(
            StyleTarget::new()
                .opacity(0.0)
                .x(*direction as f32 * 58.0)
                .scale(0.99),
        )
    })
    .target(
        TabVariant::Center,
        MotionTarget::new(
            StyleTarget::new().opacity(1.0).x(0.0).scale(1.0),
        )
        .transition(
            Transition::tween()
                .duration_secs(0.36)
                .ease(Easing::CubicBezier([0.16, 1.0, 0.3, 1.0])),
        ),
    )
    .resolver(TabVariant::Exit, |direction| {
        MotionTarget::new(
            StyleTarget::new()
                .opacity(0.0)
                .x(*direction as f32 * -34.0)
                .scale(1.01),
        )
        .transition(
            Transition::tween()
                .duration_secs(0.15)
                .ease(Easing::CubicBezier([0.7, 0.0, 1.0, 0.5])),
        )
    });

View::new()
    .custom(direction)
    .variants(variants)
    .initial_variant(TabVariant::Enter)
    .animate_variant(TabVariant::Center)
    .exit_variant(TabVariant::Exit)
```

`target` stores a static value. `resolver` stores an `Rc<dyn Fn(&Custom) ->
MotionTarget>` and runs during Reactant render, never in Unity. Resolver output
must therefore be pure and deterministic. A missing selected variant panics
before commit with the host and variant type in the diagnostic.

A layer may select one variant or an ordered `VariantList<Name>`. The builder
counterparts are `animate_variant(name)` and `animate_variants(names)`, with the
same initial, exit, and gesture forms. Lists resolve left to right. Later
variants replace earlier values only for properties they declare, and the last
declared transition for an owned property wins. Empty lists select no properties
for that layer. Duplicate names are a developer error.

Presence custom data is snapshotted when an element begins exiting. A later
parent render cannot change custom input for an already departing subtree. This
matches the reason Motion supplies custom data through `AnimatePresence`.

### Propagation

A named `animate`, `initial`, `exit`, or gesture variant propagates through
logical descendants that have a compatible `Variants<Name, Custom>` map and do
not select their own value for that layer. Propagation follows the Reactant
component tree through fragments and portals rather than the physical Unity
parent.

`.inherit_variants(false)` stops all inherited variant layers at that host and
its descendants. Selecting a local animate variant replaces only the inherited
animate layer; inherited gesture and exit layers remain independently eligible.

Parent transitions support:

```rust
Transition::tween()
    .when(TransitionWhen::BeforeChildren)
    .delay_children_secs(0.12)
    .stagger_children_secs(0.04)
    .stagger_direction(StaggerDirection::Forward)
```

`BeforeChildren` waits for the parent's finite tracks before starting children.
`AfterChildren` waits for all finite descendant tracks participating in that
propagation. Infinite tracks do not block orchestration and produce a developer
diagnostic if used as the only track in a blocking phase. Stagger order is
logical child order after reconciliation; reverse starts from the last child.
## Value model and interpolation

### Typed property catalog

`StyleTarget` can syntactically represent every public Battlement UI style
property plus Motion aliases and the additional core presentation values required
by the mockup. A particular host still accepts only the property and value-shape
combinations declared by its renderer capability; descriptor validation rejects
unsupported combinations before activation. Implementation establishes one
authoritative animation-property metadata catalog. Every entry
declares its Rust value type, canonical unit, initial value, interpolation
category, percentage reference box, additive rule, wire encoding, and Unity
writer. Generation fails when any public style property lacks an entry. Each
generated builder and serializer uses that same entry, so the Rust and Unity
catalogs cannot drift.

Interpolated values include:

- finite scalars, opacity, flex values, and numeric text metrics;
- lengths, percentages, angles, transform origins, and radii;
- core RGBA colors that Motion can interpolate;
- translate, rotate, scale, skew, and ordered transform lists;
- compatible gradients, shadows, filters, clips, and masks; and
- layout insets, sizes, gaps, padding, margins, and border widths.

`Length` preserves `Px`, `Percent`, and typed `Calc` components. Mixed
lengths interpolate in component form and resolve against the property-specific
reference box at sample time. Reactant never converts a percentage to pixels
once at animation start and then ignores resize.

Structured values interpolate only when they have compatible shapes:

- gradients require the same kind and stop count; missing stop positions are
  normalized before playback;
- filter lists use the Motion complex-value token rules described below;
- shadow lists require equal counts and matching inset/outer kinds;
- polygons require equal vertex counts; and
- transform lists require compatible operations in the same order.

Incompatible structured values use discrete interpolation. Keyframe and Motion
tracks switch at the midpoint of the segment. Appearance and disappearance of
`display` and visibility use the same endpoint exceptions as CSS-style
transitions.

Colors use Motion 13.1.1's square-root linear RGB mixer. For each RGB channel,
Reactant evaluates `sqrt(from² + progress * (to² - from²))`. Alpha
interpolates linearly and is not premultiplied into the color channels.

Filter segments reproduce Motion's complex-value mixer even when function names
change. The target filter list supplies the output template. Origin numeric,
color, and variable tokens are matched to target tokens by token class and
order; a missing origin numeric token becomes zero. Interpolation is allowed
when variable and color counts match and the origin has at least as many numeric
tokens as the target. Otherwise the segment is discrete. Consequently,
`blur(0px)` to `brightness(1.7)` samples as `brightness(0.85)` halfway through,
which is the behavior required by the modal mockup and Motion 13.1.1.

Discrete values include enums without a numerical meaning, fonts, textures,
materials, asset handles, picking mode, overflow policy, and text content.
Typed keyframes may animate them. Springs may not target them. A discrete value
inside a spring target panics during render validation.

NaN, infinity, a negative duration, invalid percentages, mismatched keyframe
metadata, and incompatible additive composition are developer errors and panic
before the Reactant commit is emitted.

### Transform aliases

`x`, `y`, `z`, `rotate`, `rotate_x`, `rotate_y`, `scale`, `scale_x`, `scale_y`,
`skew_x`, and `skew_y` are independent Motion channels. Reactant composes them
in this order:

1. translate;
2. rotate x, rotate y, then rotate z;
3. skew x, then skew y; and
4. scale.

Static UI Toolkit transform values form the baseline underneath those channels.
An explicit animated `transform_list` owns the complete animated transform and
cannot be combined with aliases in the same host animation layer. Validation
panics rather than relying on builder call order.

Layout projection is applied outside the authored transform so projection can
counter-scale descendants without changing application values. Drag translation
is a gesture layer above layout projection and below an active exit.

### Layer priority and property ownership

Reactant resolves Motion layers from lowest to highest priority:

1. static style and completed `transition_end` values;
2. the animate target or imperative controls;
3. in-view;
4. focus;
5. hover;
6. tap;
7. drag; and
8. exit.

Each higher layer overrides only properties it declares. When it deactivates,
those properties animate toward the next resolved lower layer from their current
presentation values.

Initial is a mount origin, not a persistent layer. Layout projection composes
outside this property stack. Reduced motion modifies selected transitions after
layer resolution.

A static style may always provide a baseline for an animated property. A host
may not give the same property to a Motion layer and any declared CSS-style
`Animation` or `StyleTransition`. Validation is conservative: a transition
descriptor conflicts even when its pseudo-state is currently inactive, because
Unity may activate it without a Rust commit. Reactant panics with the host path,
property, and two owners. It does not choose a winner based on call order.

Two CSS-style animations may deliberately overlap according to their list
order and composition setting. Two imperative starts through the same control
slot use most-recent-started ownership and cancel the superseded track.

## Motion values

### Handles and local graphs

`MotionValue<T>` is a stable, cloneable, `!Send + !Sync` handle owned by one
Reactant runtime and hook slot. `T` implements the sealed `MotionValueType`
trait and is one of the typed scalar, color, length, transform, or structured
values in the animation property catalog. Its current presentation value and
velocity are authoritative in Unity.

```rust
let x = use_motion_value(0.0);
let opacity = use_transform(
    x.clone(),
    InputRange::new([0.0, 100.0]),
    OutputRange::new([0.0, 1.0]),
);

View::new()
    .motion_style(
        StyleTarget::new()
            .x_value(x.clone())
            .opacity_value(opacity),
    )
```

The public hook family includes:

```rust
pub fn use_motion_value<T: MotionValueType>(initial: T) -> MotionValue<T>;
pub fn use_spring<T: SpringValue>(
    source: MotionValue<T>,
    value: SpringOptions,
) -> MotionValue<T>;
pub fn use_transform<I, O>(
    source: MotionValue<I>,
    input: InputRange<I>,
    output: OutputRange<O>,
) -> MotionValue<O>
where
    I: MotionValueType,
    O: MotionValueType;
pub fn use_velocity(source: MotionValue<f32>) -> MotionValue<f32>;
pub fn use_time() -> MotionValue<Duration>;
pub fn use_motion_time(source: MotionTimeSource) -> MotionValue<Duration>;
pub fn use_scroll(value: ScrollOptions) -> ScrollMotionValues;
pub fn use_motion_expression<T: MotionValueType>(
    expression: MotionExpression<T>,
) -> MotionValue<T>;
```

The stable handle exposes the following engine commands. `animate` returns a
playback handle for that invocation. `jump` writes a value, zeros velocity, and
detaches passive effects; `set` preserves passive-effect routing; and `stop`
freezes the current presentation value and zeros velocity.

```rust
impl<T: MotionValueType> MotionValue<T> {
    pub fn set(&self, value: T);
    pub fn jump(&self, value: T);
    pub fn stop(&self);
    pub fn animate(
        &self,
        value: T,
        transition: Transition,
    ) -> AnimationPlayback;
    pub fn get(&self) -> T;
}
```

`SpringValue` is a sealed subset of `MotionValueType`: finite scalars, colors,
same-basis lengths, and decomposable transform channels. It supplies a fixed
ordered scalar-channel representation, per-channel unit and rest thresholds,
and a norm used for completion. All channels must meet both thresholds.
Percent and `Calc` lengths spring only when both endpoints have the same basis;
otherwise render validation panics. Discrete values, gradients, filters,
shadows, clips, masks, and complete transform lists do not implement
`SpringValue`.

The range form covers the common Motion translation. Multiple inputs and
calculated values use a typed expression rather than a Rust closure:

```rust
let distance = use_motion_expression(
    MotionExpression::input(x.clone())
        .pow(2.0)
        .add(MotionExpression::input(y.clone()).pow(2.0))
        .sqrt(),
);
```

Expression nodes are a closed serializable set covering arithmetic, ranges,
clamp, modulo and wrap, minimum and maximum, powers and exponential decay,
color mixing, lengths, and transforms. Reactant does not pretend that an
arbitrary Rust closure can execute in Unity. Application-specific Rust
calculations require an explicit value subscription and therefore update only
at the Rust exchange rate.

### Time sources

`use_time()` is the direct Motion counterpart and reads unscaled runtime time.
`use_motion_time(source)` selects another Unity-local clock without creating
per-frame Rust traffic.

```rust
pub enum MotionTimeSource {
    Unscaled,
    Scaled,
    Controlled(ControlledMotionClock),
    Audio(AudioPlayback),
}
```

`AudioPlayback` is a stable, cloneable handle identifying one Battlement-owned
audio operation. The audio play command creates it before submission so Rust
can use the same identity for stop, volume, and motion-source operations. Unity
publishes the operation's current playhead, playing state, and discontinuities
directly to `MotionWorld`; it does not send a time sample to Rust each frame.

An unavailable or not-yet-started audio source reports duration zero. Pausing
or buffering freezes its motion time. Seeking, looping, or replacing playback
is a discontinuity: dependent graph nodes evaluate once at the new playhead and
retain no velocity across the jump. Stopping the operation freezes the final
playhead until every dependent descriptor releases the source.

The source abstraction is closed. Applications combine the supplied time,
scroll, pointer, and geometry values through `MotionExpression`; they cannot
register arbitrary Unity callbacks or string-named native sources.

The settings mockup's heartbeat is therefore application-authored math rather
than a library feature:

```rust
let time = use_motion_time(MotionTimeSource::Audio(music));
let phase = use_motion_expression(
    MotionExpression::input(time).sub_secs(1.04).wrap_secs(0.0, 60.0 / 56.0),
);
```

The application derives pulse strength, scale, brightness, and glow from this
phase and shares those resulting motion values through ordinary Reactant
context. The library knows nothing about beats or the mockup's visual formula.

Motion's `useMotionTemplate` translates to `use_motion_expression` with typed
`TransformList`, `FilterList`, gradient, or length expression nodes. There is no
formatted runtime CSS string. Motion's `useWillChange` has no value-level API in
Reactant because `MotionWorld` automatically prepares and releases property
writers, custom geometry, and layout-projection state for every active
descriptor. Removing that source line is the mechanical translation.

The Unity graph is acyclic. Reactant validates dependencies and reports a cycle
before commit. Unity evaluates only dirty nodes, once per frame, in topological
order. A graph may feed any number of hosts without extra Rust work.

`MotionValue::set`, `jump`, `stop`, and `animate` are allowed in event handlers,
effects, and engine-thread application callbacks. They panic during render.
`set` retargets through an attached passive effect when a component needs to
synchronize an external value.

`MotionValue::get` returns the mount value, the last locally issued value whose
Unity acknowledgement has arrived, or the last value delivered by a lifecycle
event or explicit subscription carrying that exact motion-value identity. It
may lag Unity by one exchange. It is forbidden during render because using a
sampled client value as render input would create an implicit subscription and
unstable feedback loop.

### Subscriptions

`use_motion_value_event` explicitly subscribes Rust to a value event.

```rust
use_motion_value_event(
    x.clone(),
    MotionValueEvent::Change,
    move |latest: f32| analytics.record_drag(latest),
);
```

Change, velocity, and animation-frame samples are batched into at most one
payload per rendered frame. If transport cannot keep up, the newest sample
replaces an unsent sample for the same subscription. Start, complete, and cancel
boundaries are never dropped and retain their order around coalesced samples.

Unsubscribed values send no per-frame traffic. Rust-side `get` advances only on
ordinary lifecycle messages, an explicit subscription, or another protocol
response that already carries the value's checkpoint.

### Scroll and in-view

`use_scroll` follows the Motion shape while naming Unity sources explicitly.

```rust
let target = use_element_ref();
let scroll = use_scroll(
    ScrollOptions::new()
        .target(target.clone())
        .container(ScrollContainer::Nearest)
        .axis(Axis::Vertical)
        .offset(ScrollOffset::new(
            ScrollEdge::StartEnd,
            ScrollEdge::EndStart,
        )),
);

let opacity = use_transform(
    scroll.progress(),
    InputRange::new([0.0, 0.5, 1.0]),
    OutputRange::new([0.0, 1.0, 0.0]),
);

View::new()
    .element_ref(target)
    .motion_style(StyleTarget::new().opacity_value(opacity))
    .while_in_view(StyleTarget::new().y(0.0).opacity(1.0))
    .viewport(
        ViewportOptions::new()
            .once(true)
            .amount(ViewportAmount::Fraction(0.5)),
    )
```

The result contains x, y, x-progress, and y-progress motion values. A source may
be a specific scroll-view ref, the nearest physical scroll ancestor, or the
panel viewport. Target and container refs must belong to the same panel.

Offsets use typed edge pairs, pixels, percentages, or normalized progress.
Unity recomputes them after relevant geometry or viewport changes.
`clamp(false)` allows progress outside `0..=1`.

`while_in_view` uses the same measurement registry. `amount` is `Some`, `All`,
or a fraction in `0..=1`; margin is a typed four-sided length. `once(true)`
keeps the layer active after its first entry. An unavailable target is out of
view and retains the last scroll value until a valid measurement arrives.

`on_viewport_enter` and `on_viewport_leave` receive the same measured entry as
ordinary Reactant event callbacks. Boundaries are reliable and ordered;
continuous intersection changes are available only through an explicit
subscription and are coalesced once per rendered frame.

`use_in_view(element_ref, options)` exposes the same observation as a Rust
boolean. Calling it establishes an explicit observation subscription. Entry or
exit marks the owning component dirty and rerenders it in the next Reactant
event batch; it is not a synchronous geometry read. `use_page_in_view` becomes
`use_in_view` with the panel viewport as its target.

## Presence

### AnimatePresence

`AnimatePresence` is a logical Reactant render value. It detects keyed children
removed from its latest output and retains their committed component trees until
removal is safe.

```rust
AnimatePresence::new()
    .initial(false)
    .mode(PresenceMode::Wait)
    .on_exit_complete(move |_game: &mut Game| set_closing.set(false))
    .child(open.then(|| {
        Panel::new()
            .key("settings")
            .animate(StyleTarget::new().opacity(1.0))
            .exit(StyleTarget::new().opacity(0.0))
    }))
```

The modes are:

- `Sync`: entering and exiting children coexist immediately.
- `Wait`: new children remain pending until every current exit completes. It
  supports one entering logical child and panics if a render supplies more.
- `PopLayout`: exiting hosts are removed from layout flow at their last measured
  bounds while their trees continue in a presence-owned overlay.

`Sync` is the default. A retained subtree keeps its components, hook slots,
effects, logical event ancestry, host IDs, and physical hosts. The last
committed props remain available because the parent can no longer render new
props into it.

Starting exit marks retained components that consume a presence hook dirty.
Reactant renders them with their last committed props and a false presence
value before it starts their exit descriptors. State, reducer, context, and
resource work inside the retained subtree may continue to rerender it while it
exits. Its absent parent does not rerender or supply new props.

The removal commit freezes the exit target, transition, variant custom data,
and automatic-track set for that presence generation. Retained renders may
update ordinary content, handlers, local state, context, resources, and effects,
but they cannot replace or add automatic exit tracks. Effects run normally and
their final cleanup occurs during the eventual unmount.

Exiting hosts remain focusable and interactive by default, matching Motion.
Applications use `use_is_present()` to set inert, picking, accessibility, or
focus behavior explicitly.

```rust
let is_present = use_is_present();

View::new()
    .picking_mode(if is_present {
        PickingMode::Position
    } else {
        PickingMode::Ignore
    })
    .aria_hidden(!is_present)
```

`AnimatePresence::custom(value)` supplies custom data to exiting variant
resolvers. The departing subtree receives a snapshot of the value selected by
the render that removed it.

### Completion and manual holds

Automatic removal waits for all finite exit tracks in the retained logical
subtree. Infinite exit tracks are invalid and panic before the removal commit.
An element with no exit track is immediately complete unless it acquires a
manual presence hold.

`use_presence()` returns a stable `Presence` handle. Calling the hook opts its
component into one manual hold whenever that component starts exiting.

```rust
let presence = use_presence();
let controls = use_animation_controls();
let is_present = presence.is_present();
let effect_presence = presence.clone();
let effect_controls = controls.clone();

use_effect(
    move || {
        if !is_present {
            let complete_presence = effect_presence.clone();
            effect_controls
                .start(exit_sequence())
                .on_complete(move || complete_presence.safe_to_remove());
        }
    },
    is_present,
);
```

`safe_to_remove` is idempotent for one exit generation. Calling it while present
is a no-op. A remount creates a new generation and cannot be released by an old
callback. Development builds report a retained manual hold after the configured
diagnostic interval, including the component path and hook position. Reactant
never forces removal because external work may intentionally be long-lived.

When every automatic track and manual hold completes, Reactant runs ordinary
effect cleanup, detaches refs, emits host removals, and invokes
`on_exit_complete`. Cleanup order remains the order defined by Reactant
reconciliation.

Removing an ancestor outside `AnimatePresence`, destroying the document, or
destroying the Reactant runtime cancels all tracks and unmounts immediately.
Cancellation callbacks run only while the runtime still has an event boundary
in which to dispatch them.

## Layout animation

### Layout projection

`.layout(Layout::Both)` opts a host into projection when reconciliation changes
its measured size or position. `Layout::Position` projects translation only;
`Layout::Size` projects size only.

```rust
View::new()
    .layout(Layout::Both)
    .transition(
        Transition::spring()
            .property(
                MotionProperty::Layout,
                Transition::spring().stiffness(500.0).damping(38.0),
            ),
    )
```

Unity captures pre-commit bounds, applies the complete host mutation batch,
runs UI Toolkit layout once, and captures post-commit bounds. It applies an
inverse translate-and-scale projection so the element initially appears at its
old bounds, then animates that projection to identity.

Projection accounts for transform origin, scroll offsets, panel scale, and
ancestor projection. Children participating in layout receive a corrective
inverse scale so borders, radii, text, shadows, and nested transforms do not
stretch. A child may use `Layout::Position` when size correction would be
undesirable.

An explicit animate target for width, height, margin, padding, inset, gap, or
another layout property is different. Reactant samples and applies that real
property each frame, allowing UI Toolkit to reflow. `.layout(...)` always means
projection of a state-driven layout change.

Layout projection is interruptible. A new layout commit samples the currently
projected visual bounds, discards the old projection target, and begins from the
visible result. It never snaps back to the prior unprojected layout.

`layout_scroll(true)` on a scroll container tells projection to capture its
scroll offset. `layout_root(true)` establishes a fixed projection root whose
panel-space origin is not inherited from scrolling ancestors. Reactant reports
a diagnostic when a projected descendant crosses an unmarked scroll boundary.

### Shared layout

`.layout_id(value)` matches an old and new host inside the nearest
`LayoutGroup`.

```rust
LayoutGroup::new("settings-tabs").child(
    tabs.into_iter().map(|tab| {
        Button::new(tab.label)
            .key(tab.id)
            .child(
                (tab.id == selected).then(|| {
                    View::new()
                        .layout_id("active-tab")
                        .layout(Layout::Both)
                }),
            )
    }),
)
```

Layout IDs are owned `Eq + Hash + Clone + 'static` values. Identity is the
nearest layout-group identity, value type, and value. A duplicate live ID in one
group is allowed only during a presence handoff between one source and one
destination. Any other duplicate panics before commit.

The destination host's layout transition controls the handoff. Reactant
projects from source bounds and crossfades when both hosts remain visible. If
the source is exiting, presence retains it until the shared transition and its
own exit tracks complete.

Shared layout is limited to one physical UI Toolkit panel. Portals into the same
panel remain eligible because matching follows logical groups and measurements
carry physical panel identity. A match spanning panels, displays, or world-space
documents is diagnosed and treated as two independent layout animations.
## Gestures, drag, and reorder

### Gesture layers

Every host supports `while_hover`, `while_tap`, `while_focus`,
`while_focus_visible`, and `while_drag`, with target or named-variant forms. Corresponding start, end, and
cancel callbacks use ordinary Reactant event handlers. When a gesture ends, its
properties reveal the current lower animation or value binding, or the latest
authored static style. A focus-visible highlight therefore clears on blur or
a pointer modality change without requiring an explicit `animate` target.

```rust
Button::new(trox::tx("Apply", "User-facing copy in this example."))
    .while_hover(StyleTarget::new().y(-1.0).filter_brightness(1.08))
    .while_tap(StyleTarget::new().scale(0.96))
    .on_hover_start(move |_game: &mut Game, event| {
        log.hover(event.pointer_type)
    })
    .on_tap(move |game: &mut Game, _event| game.apply_settings())
```

Unity recognizes these gestures locally from UI Toolkit pointer, navigation,
and focus events. Hover ignores touch pointers. Tap captures the initiating
pointer, remains active while the pointer is within the configured slop, and
cancels on capture loss, disablement, or an incompatible drag. Keyboard and
gamepad submit produce the same tap layer and callback.

Gesture callbacks enter Reactant's existing capture and bubble event batch.
Visual gesture activation does not wait for that callback or its resulting Rust
render.

Pan uses Motion-shaped event phases without adding a persistent target layer:

```rust
View::new()
    .on_pan_session_start(handle_pan_session)
    .on_pan_start(handle_pan_start)
    .on_pan(handle_pan_move)
    .on_pan_end(handle_pan_end)
```

Session start, threshold-crossing start, end, and cancellation are reliable.
`on_pan` samples contain point, delta, offset, and velocity and are coalesced
once per rendered frame. Drag recognition is built on the same local recognizer
but owns translation and adds constraints, elasticity, and momentum.

`MotionConfig::gesture(GestureConfig)` establishes inherited recognition
defaults, and the same fields are available as host builders. Motion 13.1.1 is
the baseline: pan and drag start after three panel pixels, direction lock picks
y after ten vertical pixels or otherwise x after ten horizontal pixels, and
drag elasticity defaults to `0.35`. Elastic overshoot is
`bound + overshoot * factor`. Tap remains active inside the hit rectangle plus
a default slop of three panel pixels for mouse or pen and eight for touch.

Only the initiating primary pointer owns a gesture; additional pointers are
ignored until release. Pointer capture loss, host disablement, document blur,
or removal emits cancellation exactly once. Drag recognition first cancels the
tap layer and its callback, then emits drag start. Keyboard and gamepad submit
cannot become drag. Focus targets the exact focused host; focus-within is an
explicit separate target. These constants are configurable without changing
the event ordering.

### Drag

Drag configuration is attached directly to a host.

```rust
let constraints = use_element_ref();

View::new()
    .element_ref(constraints.clone())
    .child(
        View::new()
            .drag(DragAxis::Both)
            .drag_constraints(DragConstraints::element(constraints))
            .drag_elastic(DragElastic::sides(0.12, 0.12, 0.08, 0.08))
            .drag_momentum(true)
            .drag_direction_lock(true)
            .while_drag(StyleTarget::new().scale(1.03))
            .on_drag_end(move |game: &mut Game, event| {
                game.commit_position(event.offset());
            })
            .on_drag_momentum_complete(move |game: &mut Game, event| {
                game.commit_position(event.offset());
            }),
    )
```

Constraints may be typed pixel bounds or the padding box of an attached element
ref in the same panel. Unity refreshes element constraints after geometry
changes. The drag origin is the current presentation transform, including a
running animation or layout projection.

Drag applies locally through motion values and emits start, direction-lock,
end, and cancel events. Move samples are opt-in through `on_drag` or a bound
motion-value subscription and are coalesced once per rendered frame. Velocity
and momentum remain local. The end event includes point, delta, offset,
velocity, constraint state, and whether momentum will continue.

`on_drag` reports pointer-driven movement only. When momentum is enabled,
`on_drag_end` closes pointer ownership and identifies the pending momentum
generation. Unity then emits one reliable `on_drag_momentum_complete` boundary
with its final offset, velocity, resolved constraint, and generation. A bound
motion-value subscription may observe coalesced momentum samples. Cancellation
emits neither completion nor another drag end. Rust commits or corrects the
semantic terminal position from the completion boundary.

Rust application state remains authoritative. A drag may update presentation
optimistically, but the next declarative target can accept, adjust, or reverse
the result. Disconnect cancels pointer ownership and momentum. Reconnect starts
from the last acknowledged motion value or the current declarative target.

`use_drag_controls()` allows a different host to initiate drag. Starting is an
event/effect operation and names the target by stable control binding, not by a
Unity object ID.

The remaining common Motion drag options have direct typed counterparts:

- `drag_snap_to_origin(axes)` animates the selected offsets back to their drag
  origins after release and suppresses unconstrained momentum on those axes.
- `drag_listener(false)` disables pointer initiation on the draggable host so
  only `DragControls::start` can begin it.
- `drag_propagation(true)` permits an eligible ancestor recognizer to continue
  after the child claims drag; the default cancels ancestor drag recognition.
- `drag_transition(value)` supplies the inertia and boundary-spring settings.
- `DragStartOptions::snap_to_cursor(true)` makes controls start with the dragged
  host centered on the initiating pointer.

Motion's synchronous `onMeasureDragConstraints` closure cannot cross the Unity
boundary. `DragConstraintAdjustment` provides serializable inset, outset, axis,
and clamp operations applied during local measurement. An optional
`on_drag_constraints_measured` callback observes the final bounds in the next
Reactant event batch but cannot rewrite the active measurement retroactively.

### Reorder

`ReorderGroup<T>` provides collection semantics. `ReorderItem<T, H>` owns one
explicit Reactant host façade `H`; it never accepts an arbitrary component or
inserts a wrapper. The host is the measured, projected, pickable, and draggable
item. Its children may contain arbitrary components.

```rust
impl<T, H: private::HostFacade> ReorderItem<T, H> {
    pub fn new(id: T, host: H) -> Self;
}
```

```rust
ReorderGroup::new(Axis::Vertical, items.clone())
    .on_reorder(move |game: &mut Game, order| game.set_order(order))
    .children(items.into_iter().map(|item| {
        let id = item.id.clone();
        ReorderItem::new(
            id.clone(),
            View::new().child(PlayerRow::new(item)),
        )
        .key(id)
    }))
```

Unity projects neighboring items out of the dragged item's path and emits a
semantic proposed order whenever the crossed midpoint changes. The latest
proposal is coalesced per rendered frame. Rust commits the accepted order; keys
and layout projection reconcile it without a visual snap.

The group supports horizontal and vertical lists. Grid and arbitrary two-axis
reorder are not inferred; applications implement those semantics with drag,
layout, and explicit order events.

## Imperative animation

### Stable controls

`use_animation_controls()` returns a stable typed `AnimationControls<Name>`
handle. Rust infers `Name` from its starts and bound hosts. A host binds it with
`.animation_controls(controls.clone())` only when its variant map uses the same
`Name`. Binding zero hosts is valid; binding several hosts broadcasts commands,
but every host has the same compile-time variant-name type.

```rust
let controls = use_animation_controls();
let click_controls = controls.clone();

Button::new(trox::tx("Replay", "User-facing copy in this example."))
    .on_click(move |_game: &mut Game| {
        click_controls.start(
            MotionTarget::new(
                StyleTarget::new()
                    .scale_keyframes(Keyframes::new([1.0, 1.08, 1.0])),
            )
            .transition(Transition::tween().duration_secs(0.28)),
        );
    });

View::new()
    .animation_controls(controls)
    .animate(StyleTarget::new().scale(1.0))
```

Controls may start a target or named variant, set a presentation value without
animation, and stop all controlled tracks. Starting before the host is attached
queues the latest target for that binding. Starting after detach is a no-op.

```rust
pub fn use_animation_controls<Name: VariantName>()
    -> AnimationControls<Name>;

pub enum ControlTarget<Name: VariantName> {
    Target(MotionTarget),
    Variant(Name),
}

impl<Name: VariantName> AnimationControls<Name> {
    pub fn start(&self, target: impl Into<ControlTarget<Name>>)
        -> AnimationPlayback;
    pub fn set(&self, target: impl Into<ControlTarget<Name>>);
    pub fn stop(&self);
    pub fn clear(&self);
}
```

`clear` removes the imperative slot and reveals the next lower layer. A control
binding has at most one pending start while detached. A later `start`, `set`,
`stop`, or `clear` replaces or clears that pending command deterministically.

An imperative target occupies the animate layer. A new declarative animate
target supersedes it on the next commit. Gesture and exit layers retain their
higher priority.

### Scoped sequences

`use_animation_scope()` returns one cloneable `AnimationScope`. Calling
`.animation_scope(scope.clone())` attaches its root to a host, while
`scope.start(...)` is the scoped animate function. A sequence addresses a
closed set of typed selectors:

- an `ElementRef`;
- a motion name attached with `.motion_name(value)`;
- the scope root;
- direct children; or
- all descendants.

There are no USS selector strings.

```rust
impl AnimationScope {
    pub fn start(&self, sequence: AnimationSequence)
        -> AnimationPlayback;
    pub fn set(
        &self,
        selector: MotionSelector,
        target: StyleTarget,
    );
    pub fn stop(&self, selector: MotionSelector);
}
```

Selector resolution is captured atomically when a command begins. Hosts added
later do not join that invocation, and hosts removed during it are canceled.

```rust
let scope = use_animation_scope();
let effect_scope = scope.clone();

use_effect(
    move || {
        effect_scope.start(
            AnimationSequence::new()
                .animate(
                    MotionSelector::name("row"),
                    StyleTarget::new().opacity(1.0).x(0.0),
                    Transition::tween().duration_secs(0.2),
                )
                .then(
                    MotionSelector::name("badge"),
                    StyleTarget::new().scale(1.0),
                    Transition::spring().stiffness(420.0),
                )
                .at(SequencePosition::WithPrevious(0.06)),
        );
    },
    (),
);

Panel::new()
    .animation_scope(scope.clone())
    .children(rows.into_iter().map(|row| {
        PlayerRow::new(row).motion_name("row")
    }))
```

Sequence positions support after previous, with previous, absolute time, and a
signed offset from a named label. `.at(...)` repositions the most recently
appended `animate` or `then` step and panics if no step precedes it. Targets are
snapshotted when their sequence step becomes eligible. A target unmounted
before its step is skipped; an active target unmounted during its step is
cancelled. Empty selector results are valid and complete immediately.

### Playback handles

Every imperative start returns an `AnimationPlayback` handle with stable
identity for one start generation.

```rust
impl AnimationPlayback {
    pub fn play(&self);
    pub fn pause(&self);
    pub fn stop(&self);
    pub fn cancel(&self);
    pub fn complete(&self);
    pub fn seek(&self, elapsed: Duration);
    pub fn set_speed(&self, speed: f32);
    pub fn set_direction(&self, direction: PlaybackDirection);
    pub fn on_complete(&self, callback: impl FnOnce() + 'static);
    pub fn on_repeat(&self, callback: impl FnMut(RepeatEvent) + 'static);
    pub fn on_stop(&self, callback: impl FnOnce() + 'static);
    pub fn on_cancel(&self, callback: impl FnOnce() + 'static);
}
```

`stop` freezes the current presentation value and ends playback without applying
the target. That value remains in the imperative control slot until a later
`start`, `set`, or `clear`, or until a declarative animate commit supersedes the
slot. `cancel` removes the slot and reveals the next lower animation layer.
`complete` applies the terminal target and completion event. Seeking clamps
finite animation and wraps repeating animation. A speed of zero is equivalent
to pause; a negative speed is invalid because direction is explicit.

Seek follows Motion 13.1.1's immediate-time contract. Direct, repeated, and
backward seeks all sample the requested logical time and synchronously publish
one coalesced update when that slot subscribes to updates. Seek never emits
start, repeat, or completion boundaries, including when the requested time
crosses those boundaries. Repeating the same seek still publishes its one
coalesced update. A seek leaves playback paused; resuming from a terminal seek
may emit completion only after logical playback advances again. These rules are
identical for imperative handles and controlled-clock validation.

The handle also implements Reactant's single-threaded completion future. Its
future resolves to `PlaybackOutcome::Completed`, `Stopped`, or `Cancelled`.
After any terminal operation, later commands on that playback handle are
no-ops. Dropping the handle does not cancel playback.

Lifecycle callbacks are authored on the slot they observe. `MotionTarget`, each
keyed `Animation`, and `StyleTransition` provide `on_start`, `on_update`,
`on_repeat`, `on_complete`, `on_stop`, and `on_cancel` where that event can
occur. The methods follow Reactant's event shape: a brief callback accepts
`Fn(&mut G)`, while an `_event` callback additionally receives the typed slot
event. Adding a callback returns a hidden callback-bearing adapter accepted by
the same target or animation builder; it does not change slot identity.

Host-level `on_animation_start`, `on_animation_update`,
`on_animation_repeat`, `on_animation_complete`, `on_animation_stop`, and
`on_animation_cancel` are conveniences for the host's direct Motion slot only.
They do not observe CSS animations or style transitions. Repeat events include
the slot identity, completed iteration range, direction, and logical elapsed
time. Style-transition events additionally name the property. Update is an
explicit subscription and is coalesced once per rendered frame. Lifecycle
callbacks are ordered by animation layer, host preorder, and slot identity.

## Reduced motion

`MotionConfig` establishes inherited defaults without adding a Unity host.

```rust
MotionConfig::new(app)
    .transition(Transition::spring().stiffness(420.0).damping(32.0))
    .reduced_motion(ReducedMotion::User)
    .time_source(MotionTimeSource::Unscaled)
```

The optional transition is the inherited default for descendants that do not
declare one. A closer `MotionConfig`, a host transition, or a target-local
transition overrides it in that order. Property overrides merge by property
rather than replacing the complete inherited transition.

`ReducedMotion` is `User`, `Always`, or `Never`. Unity 6000.5 does not expose a
portable reduced-motion setting. `User` therefore uses a Reactant-owned
platform bridge: the native macOS player observes the system Reduce Motion
preference, while WebGL observes the browser's `prefers-reduced-motion` media
query. Both bridges publish the initial value before the first Reactant frame
and deliver live changes on the engine thread. Bridge unavailability is a host
contract failure rather than an implicit `false` value.

`use_reduced_motion()` returns the inherited resolved boolean for a component
that wants a custom fallback. `Always` and `Never` bypass platform observation,
which also gives controlled tests a deterministic input.

The automatic reduced-motion policy suppresses transform, layout projection,
drag momentum, and scroll-linked movement. Those properties jump to the target.
Opacity, color, background, and other non-spatial transitions continue. An
application may branch on `use_reduced_motion()` to replace any target or
animation explicitly.

```rust
let reduce_motion = use_reduced_motion();

Panel::new().animate(if reduce_motion {
    StyleTarget::new().opacity(1.0)
} else {
    StyleTarget::new().opacity(1.0).x(0.0)
})
```

CSS-style animations obey the same property suppression. An animation whose
remaining tracks are all infinite and non-spatial continues; one whose only
tracks are suppressed becomes a static target and produces no lifecycle loop.

Changing the resolved policy affects mounted motion at the next panel sample.
Finite spatial tracks apply their targets immediately and become complete;
`transition_end` runs when every finite track in that slot is terminal, and a
presence exit may then release. Mixed slots continue their non-spatial tracks
before completing. Infinite spatial tracks become dormant while their logical
phase continues on the selected clock, emit no repeated lifecycle loop, and
resume at that phase if the policy later permits motion. Policy changes are not
cancellations, so they never emit cancel or stop boundaries.

## Runtime architecture

### Declarative descriptors

Rust lowers each animated host into a validated `MotionDescriptor` beside its
ordinary host properties. A descriptor contains:

- stable host, decoration, motion-value, and animation-slot identities;
- static baselines and typed target layers;
- transitions, keyframes, variants already resolved for this render, and
  orchestration edges;
- gesture, scroll, layout, presence, and callback subscriptions;
- motion-value graph nodes and bindings; and
- clock, reduced-motion, and reconnect policy inherited from `MotionConfig`.

Descriptor identity combines the Reactant runtime, document, host `ObjectId`,
motion slot, and committed generation. Reusing a host and slot updates the
descriptor. Changing a keyed animation entry preserves its timeline identity.
Removing an entry cancels that timeline unless presence retains it.

Descriptors are values in Reactant's work-in-progress tree. Render validation
finishes before any motion command is emitted. A failed or suspended render
retains the previous committed descriptors and Unity timelines unchanged.

### Commit ordering

One Reactant commit is a staged transaction. Unity performs these operations on
its engine thread:

1. validate every host, panel, asset, ref, graph, and renderer binding;
2. capture affected presentation values, velocities, static baselines, scroll
   offsets, and pre-layout geometry before any mutation;
3. stage host creation, movement, static properties, descriptors, graphs,
   measurement participation, removals, and queued actions;
4. apply structural mutations and static properties that have no active
   animation owner;
5. install descriptors using the captured presentation state as their origin;
6. publish static-baseline changes beneath active tracks without writing over
   their live presentation values; and
7. make the transaction visible to the next motion frame.

The old and new resolved pseudo-styles are part of the preflight snapshot, so a
`StyleTransition` begins at the old presentation value even though its target
baseline arrives in the same commit. Reusable CSS keyframes capture their new
underlying value only after all lower layers for that transaction are staged.
Retargeting similarly receives the old sampled value and velocity before any
new layer can overwrite them.

Developer errors discoverable during Rust render panic before a commit is
emitted. A Unity-only validation failure rejects the complete command group,
keeps the prior host tree and `MotionWorld` unchanged, and reports the ordinary
host contract failure through the session. No failed installation cancels or
partially retargets an existing timeline.

An update can therefore animate a newly mounted host on its first visible frame
and project an existing host without an intermediate flash at its final layout.

### Unity motion world

Each UI panel owns a `MotionWorld`. It contains compact arrays for timelines,
tracks, graph nodes, gestures, layout projections, and dirty host outputs.
Stable IDs index generation-checked slots; destroyed slots return to free lists.

Reactant checks in a normalized dump of
`PlayerLoop.GetDefaultPlayerLoop()` produced by both Release profiles under
Unity `6000.5.8f1`. The fixture contains the Unity version, player assembly
version, parent type, and complete ordered sibling list. It, rather than the
illustrative immutable UnityCsReference link, is the normative topology.

Reactant installs two engine-thread callbacks as immediate siblings around the
single `PreLateUpdate.UIElementsUpdatePanels` entry. In the same
`PreLateUpdate.subSystemList`, the required order is exactly Reactant
pre-layout, UI Toolkit panel update, then Reactant post-layout. Existing
unrelated siblings retain their relative order. The pre-layout callback:

1. advances the selected clocks;
2. recognizes pending gesture and scroll changes;
3. evaluates dirty motion-value graph nodes;
4. samples timelines and real layout-property tracks;
5. resolves layer priority and transform composition; and
6. applies all changed layout-affecting values in property batches.

UI Toolkit then performs its normal panel update and layout work. The
post-layout callback runs immediately afterward and before
`PostLateUpdate.UIElementsRepaintPanels`. It captures post-layout geometry,
installs or retargets layout projection, samples presentation-only tracks,
applies changed final values, and queues coalesced sample and lifecycle
messages. A commit completed during `Update` can therefore install an entrance
origin and publish its first sample in the same rendered frame.

PlayerLoop registration is process-wide and reference counted. Registration
detects an existing Reactant entry, removes stale entries after domain reload,
and restores the exact prior loop after the last runtime shuts down. It
validates the fixture before insertion and validates adjacency after insertion,
every scene load, every Reactant commit, and every detected domain reload.
Pre- and post-layout callbacks also exchange a monotonically increasing frame
token; a missing, duplicated, or reversed callback fails the session before
another motion sample is applied.

A third-party PlayerLoop injector must install before Reactant or explicitly
request Reactant re-registration after calling `PlayerLoop.SetPlayerLoop`.
Mutation that breaks the adjacency contract is unsupported and becomes the
same host failure. Reactant never falls back to an unordered
`VisualElement.schedule` loop.

Steady-state sampling allocates no managed memory. Keyframe arrays, graph edges,
callbacks, property writers, and scratch buffers are prepared when descriptors
change. Hosts with unchanged resolved output receive no UI Toolkit assignment.

Layout-affecting values are applied together before UI Toolkit's panel update.
Presentation-only values follow that update. Reactant never alternates a layout
read and write for each animated element. Instrumented integration tests count
the relevant panel layout update and reject a Reactant path that causes a
second synchronous layout pass in one frame.

### Clocks and dropped frames

Unscaled time is the default. `.time_source(MotionTimeSource::Scaled)` opts a
subtree, animation, or imperative start into Unity game time. The nearest
setting wins. Pause and playback speed multiply the selected clock without
changing its source.

`ControlledMotionClock` replaces both sources for tests and captures. It
advances only through explicit commands and samples exact requested durations.
Closed-form generators evaluate directly at the requested logical time.

Production sampling uses `FrameDropPolicy::CatchUp`. If a frame is missed,
Unity samples once at the current logical time. It does not simulate or dispatch
every unseen presentation frame. Keyframe boundaries, repeat boundaries, and
completion crossed during the gap are processed in chronological order before
the resulting sample. Rust receives at most one coalesced update after them.

Springs and unconstrained inertia use the normative closed-form generators. A
bounded inertia track evaluates its exponential until the analytically solved
boundary-crossing time, then evaluates the boundary spring from that crossing
state. A long frame therefore changes only the sampled time, not the result.

### Motion-compatible behavior and native optimization

The Reactant sampler is the semantic reference. It defines easing, spring,
inertia, interruption, keyframe, repeat, callback, and seek behavior. PrimeTween
is not part of the public contract and may not approximate these semantics.

UI Toolkit transitions may execute a track only when an internal eligibility
check proves that the native path supports its property, timing, easing,
interruption, clock, lifecycle events, and reduced-motion behavior exactly.
Initial implementations should limit eligibility to simple finite tweens over
native interpolable properties.

Every native-eligible class has deterministic conformance tests against the
Reactant sampler. A Unity or platform version that fails conformance disables
that optimization and uses the sampler. Backend selection is absent from the
Rust API, snapshots, callbacks, and diagnostics unless developer tracing is
enabled.

### Lifecycle protocol

Unity reports lifecycle messages with runtime, session, descriptor, playback,
generation, and monotonically increasing sequence identities. Messages include
activation, start after delay, repeat, update, completion, cancellation, and a
timeline checkpoint.

Activation acknowledges the presentation value and logical elapsed time at
which Unity installed a track. Reactant uses the acknowledgement to anchor the
timeline to its monotonic runtime clock. Late events for a superseded generation
are acknowledged and ignored.

Lifecycle boundaries travel in `MotionEventBatch` values containing the session
ID, first and last sequence, and ordered events. Rust acknowledges the highest
contiguous sequence in every response and heartbeat. Unity retains
unacknowledged boundaries and retransmits them after a timeout. Rust
deduplicates by a logical ID containing runtime, descriptor, generation, slot,
event kind, and logical boundary index; the transport session is not part of
that ID. A gap requests replay from the first missing sequence before later
boundaries dispatch.

High-frequency values are replaceable samples and do not occupy reliable
sequence positions. One inbound batch may carry both kinds, with boundaries
partitioning samples so completion cannot appear before its last update.
Consecutive repeat boundaries may be encoded as an inclusive logical-iteration
range; Rust expands it in order only for an explicit repeat subscription. This
bounds retained data for a fast infinite animation while preserving semantics.

A callback belongs to one animation slot and generation, never one property.
Slots are the direct Motion layer, each keyed CSS animation entry, each active
style-transition property, and each imperative start. Start fires once when the
slot's first track leaves delay. Update carries the resolved style snapshot for
that slot and is coalesced once per rendered frame. Complete fires after all
finite tracks finish and after `transition_end` has been applied. Stop and
cancel are distinct terminal outcomes. An immediate target or a negative delay
past the end emits start, final update when subscribed, and complete in the same
batch. A disabled initial target emits no lifecycle events.

Every normal session response and transport heartbeat includes one compact
scaled-clock checkpoint when scaled timelines are active. This is timeline
metadata rather than a presentation-value subscription. It bounds reconnect
rollback without creating per-frame Rust traffic.

Callbacks enter the same Reactant batching boundary as other Unity events.
State updates from all callbacks in one payload reconcile once after dispatch.
A resulting retarget is a later commit and cannot rewrite the event that caused
it.

### Reconnect and reconstruction

Reactant retains logical motion descriptors, playback state, and timeline
anchors across Unity session replacement. The replacement snapshot includes
the sampled logical state and last logical boundary index required to construct
the new `MotionWorld`. Reactant synthesizes boundaries crossed after the last
checkpoint, then applies the same logical-ID deduplication used for retransmit.

Reconnect behavior is:

- completed entrance animations remain completed;
- active unscaled finite and repeating timelines resume at the elapsed phase
  calculated from their acknowledged logical anchor;
- active exits resume and still gate unmount;
- paused timelines resume paused at their saved elapsed value;
- scaled timelines resume from their last acknowledged checkpoint because game
  time cannot be inferred while the Unity session is absent;
- controlled timelines use the exact controlled clock value; and
- pointer gestures, drag capture, and uncommitted momentum cancel because the
  originating input session no longer exists.

The first reconstructed frame applies the resumed presentation value before it
is shown. Reconnect never replays `initial`. Motion values reconstruct from
their latest acknowledged value and graph definition, then declarative targets
may continue driving them.

A completion that logically occurs while an unscaled session is disconnected
is included as completed in the replacement snapshot and dispatched once when
the new session becomes active. Generation and sequence identities prevent a
late old-session completion from dispatching it twice.

### Assets and advanced paint

Reactant Animations has no hard dependency on the Reactant Asset Generator or
any numbered task in its implementation plan. Animation descriptors consume
ordinary prepared textures and typed renderer values. A generated texture is
one optional source of such an immutable texture and follows exactly the same
runtime path as any other prepared texture.

Reactant Animations may:

- animate the transform, opacity, tint, filter, clip, or layout of an ordinary
  texture host;
- use prepared texture variants as discrete keyframes;
- place a prepared texture in a decoration layer; or
- drive a separately implemented UI Toolkit custom renderer or shader through
  typed style and motion-value bindings.

When both projects exist, Asset Generator Task 16 makes its textures ordinary
Reactant prepared assets and Task 17 makes them available in Unity. No animation
task waits for either integration because styles, authored textures, custom
geometry, and shaders cover the same animation interfaces. Asset Generator
Tasks 01–15, 18, and 19 likewise provide no prerequisite for this design.

The animation design does not add runtime inputs to asset generation and does
not accept arbitrary shader-property strings. A custom renderer exposes typed
properties through the same property catalog before those values can
participate in `StyleTarget`.

The properties required by the settings mockup have concrete runtime paths:

- rectangular inset clips use the host's existing overflow clip and sampled
  inset values without inserting a wrapper;
- static polygon clips use authored vector chrome or custom geometry;
- animated polygon clips update cached custom geometry for a capable host or
  decoration rather than claiming to clip arbitrary descendants;
- skew, `rotate_x`, and `rotate_y` deform cached custom quad geometry for the
  capable host paint or decoration while leaving descendant layout and input
  geometry unchanged;
- blur, brightness, saturation, contrast, hue rotation, and supported filter
  lists use pinned UI Toolkit filter writers; and
- gradients, glows, shadows, and masks use ordinary styles when supported and
  prepared textures or cached custom geometry otherwise.

Reactant does not promise offscreen capture or transformation of an arbitrary
live UI Toolkit subtree. UI Toolkit exposes no supported subtree-capture
boundary that can suppress the original paint, preserve interaction, and
re-composite descendants without changing physical hierarchy. The public
`skew_x`, `skew_y`, `rotate_x`, `rotate_y`, and polygon-clip channels therefore
have property-specific paint semantics rather than CSS pixel-subtree semantics.
The mockup's modal uses deforming panel chrome and decorations around unchanged
live content, which provides the requested visible skew motion without a hidden
second panel or render texture.

Private paint resources never become Reactant hosts, physical parents, logical
children, event targets, focus targets, accessibility nodes, or layout
participants. The original host retains layout, picking, focus, geometry, and
event ownership. This invariant also applies to layout projection and
`PopLayout` overlays.

Every `(host kind, property, value shape)` combination declares one fixed
renderer capability in the generated property catalog. Missing assets,
incompatible prepared-asset kinds, unsupported value shapes, or an unavailable
renderer binding reject the descriptor transaction as host contract failures.
A supported approximation must visibly respond to the requested channel;
Reactant never accepts a track and then silently omits its paint.
## Mockup translation coverage

The acceptance source is `~/Documents/mockups` at Git commit
`2451ea9cc6f76b356b1102ee37b82c478853122a`. The ledger below is a manually
reviewed requirements checklist. It does not create a source analyzer,
manifest, generated fixture, mirrored gallery, or automated coverage check.

Implementation reviews the pinned source directly and exercises the complex
animation families through focused Rust tests or the existing Reactant sample.
Simple declarations do not each require a separate test. The settings screen
itself is not ported.

The acceptance criterion is API and behavior coverage. Pixel identity and
matching the browser's exact intermediate trajectory are not required. Values,
times, easing, repetition, presence policy, and interruption semantics are
preserved unless an entry explicitly names a paint approximation.

### Coverage ledger

- `BackgroundMusic.tsx:139`: audio-synchronized control heartbeat. Read the
  stable audio playback handle through `MotionTimeSource::Audio`, reproduce
  `heartbeatStrength` with serializable modulo, minimum, clamp, and exponential
  expression nodes, and derive shared scale, brightness, and glow motion
  values. Distribute those values through ordinary Reactant context. Paused,
  stalled, ended, and reduced-motion states resolve to zero pulse strength.

- `SettingsTabs.tsx:90`: tab host. Target `y = active ? 0 : 3`, hover
  `y = active ? 0 : -1`, tap `scale = 0.955`, spring stiffness `520`, damping
  `32`, mass `0.7`. Use `Button` target, hover, and tap builders. Reduced motion
  retains color/opacity feedback and removes spatial movement.

- `SettingsControls.tsx:245`: dropdown-button transform `90ms` cubic Bézier
  `(.2,.8,.2,1)` and filter `140ms ease`. Use typed pseudo styles and
  `StyleTransition`.

- `SettingsControls.tsx:268`: dropdown menu presence. Enter from opacity `0`,
  `y = -12`, `scale_y = .76` to `1, 0, 1` in `.2s` with
  `(.2,.8,.25,1)`; exit to `0, -7, .42` in `.26s` with
  `(.4,0,.75,.3)`. Use `AnimatePresence` and a Reactant host façade with
  inherent Motion builders.

- `SettingsControls.tsx:317`: dropdown option presence. Enter from opacity `0`
  and `x = -17` in `.18s ease-out`, delayed `index * .028s`; exit to opacity
  `0`, `x = 10`. Use keyed Motion children and per-item delay.

- `SettingsControls.tsx:377`: option host interaction. Translate its target and
  gesture declarations directly onto the option host; use the transition entry
  below for CSS-owned properties.

- `SettingsControls.tsx:417`: option transform `90ms`
  `cubic-bezier(.2,.8,.2,1)`, box shadow and filter `140ms ease`. Use typed
  pseudo styles and `StyleTransition`.

- `SettingsControls.tsx:423`: selected-option flash. Opacity `.9 -> 0`, scale
  `.96 -> 1.035`, `.38s ease-out`, with `.01s` reduced-motion duration. Use a
  keyed child under `AnimatePresence`.

- `SettingsControls.tsx:478`: toggle label transform `140ms ease`. Use typed
  pseudo styles and `StyleTransition`.

- `SettingsControls.tsx:588`: checkbox transform `90ms`
  `cubic-bezier(.2,.8,.2,1)`, filter `90ms ease`, border `140ms ease`, and box
  shadow `140ms ease`. Use a multi-property `StyleTransition`.

- `SettingsControls.tsx:664`: control transform `90ms`
  `cubic-bezier(.2,.8,.2,1)`. Use `StyleTransition`.

- `SoundSettings.tsx:179`: slider transform `90ms`
  `cubic-bezier(.2,.8,.2,1)`, filter `90ms ease`, and box shadow `140ms ease`.
  Use a multi-property `StyleTransition`.

- `SoundSettings.tsx:240`: transform `90ms`
  `cubic-bezier(.2,.8,.2,1)` and filter `140ms ease`. Use typed pseudo styles
  and `StyleTransition`.

- `InputSettings.tsx:237`: binding blink opacity `[1, 1, .08, .08, 1]`, `1.05s`
  linear, infinite. Use a Motion keyframe target with `Repeat::Forever`.
  Reduced motion leaves the indicator at its readable static value.

- `InputSettings.tsx:334`: transform `90ms`
  `cubic-bezier(.2,.8,.2,1)` and filter `140ms ease`. Use
  `StyleTransition`.

- `ActionButton.tsx:87`: transform `90ms`
  `cubic-bezier(.2,.8,.2,1)`, filter `140ms ease`, and background `140ms ease`.
  The tap gesture layer owns the pressed transform. Typed hover and focus
  pseudo-styles own filter and background, so no property is driven by two
  layers for the same interaction.

- `ControlInteraction.tsx:41`: `control-shine-sweep`, `720ms ease-out`, one
  iteration, `Both` fill. Use a decoration CSS `Animation`; reduced motion
  installs no animation.

- `ArcadeAttractMode.tsx:125`: perspective grid breathing. Preserve the source
  keyframes, duration, alternate direction, and infinite iterations in a CSS
  `Animation` on grid chrome.

- `ArcadeAttractMode.tsx:169`: 48 particle loops. Preserve each seeded size,
  color, position, drift, duration, and negative phase delay. Use keyed
  decorations and `AnimationIterations::Forever`; reduced motion is static.

- `ArcadeFramePulse.tsx:111`: two border comets. Preserve the `6.5s` linear
  infinite nine-frame left/top/rotation path and settings cutout mask. Use two
  decorations sharing one `Keyframes` value.

- `ArcadeMenuTransition.tsx:88`: routed screen presence. Preserve source
  variants, `.3s` duration, `.17s` delay, and `(.16,1,.3,1)` easing. Use `Sync`
  normally and `Wait` for the source's conservative backend branch.

- `ArcadeMenuTransition.tsx:138`: contained beam. Opacity
  `[0,.72,0]`, scale-x `[.15,1,.72]`, `.3s`, times `[0,.48,1]`. Use a
  decoration Motion target.

- `ArcadeMenuTransition.tsx:177`: reveal scan. Clip inset
  `[49.7%,46%,0%]`, opacity `[0,.48,0]`, times `[0,.44,1]`, and
  `(.65,0,.35,1)`. Use rectangular clip and opacity channels.

- `ArcadeMenuTransition.tsx:208`: transition beam. Preserve its literal source
  target, keyframe, duration, time, and easing values in a decoration case.

- `ArcadeTabTransition.tsx:77`: directional panel. Use typed direction custom
  data, named enter/center/exit variants, `AnimatePresence::custom`, and
  `PopLayout`; preserve the variant-local transition values at lines `16-25`.

- `ArcadeTabTransition.tsx:108`: directional light sweep. X starts at `-90` or
  `940` and crosses to the other value; opacity `[0,.68,.68,0]`, `.34s`, times
  `[0,.22,.72,1]`, easing `(.4,0,.2,1)`. Skew applies to custom sweep
  geometry, not the live panel subtree.

- `ArcadeTabTransition.tsx:140`: scan line. Y `-12 -> 1000`, opacity
  `[0,.38,.22,0]`, `.42s` linear, times `[0,.1,.72,1]`. Use a decoration
  Motion target.

- `ArcadeModal.tsx:82`: backdrop presence. Opacity `0 -> 1 -> 0`, `.2s`, or
  `.01s` under reduced motion. Use `AnimatePresence`.

- `ArcadeModal.tsx:104`: modal panel. Preserve all source opacity, scale-x,
  scale-y, x, skew-x, and filter frames, `.42s` entry or `.3s` exit, and
  `ease-out`. The three-frame exit uses even per-property spacing when the
  four-value times array is incompatible. Skew deforms panel chrome while live
  content remains undeformed.

- `ArcadeModal.tsx:168`: modal shine. X `-115% -> 115%`, `1.8s` linear,
  infinite Motion repetition with `1.2s` repeat delay. Use a decoration target;
  reduced motion omits it.

- `ArcadeCheckboxEffect.tsx:33`: checked burst root. Key by activation; retain
  with presence and exit opacity in `.04s`.

- `ArcadeCheckboxEffect.tsx:40`: checkbox ring. Preserve its source opacity,
  scale, rotation, duration, and easing in a decoration target.

- `ArcadeCheckboxEffect.tsx:53`: checkbox flash. Preserve its source opacity,
  scale, duration, and easing in a decoration target.

- `ArcadeCheckboxEffect.tsx:69`: checkbox beam. Preserve its source transform,
  opacity, duration, and easing in a decoration target.

- `ArcadeCheckboxEffect.tsx:87`: checkbox sparks. Preserve per-spark opacity,
  x, y, rotation, scale-x, duration, delay, and easing on keyed decorations.

- `ArcadeButtonEffect.tsx:32`: button burst root. Key by activation and preserve
  the root presentation values.

- `ArcadeButtonEffect.tsx:46`: button ring. Preserve source opacity, scale,
  rotation, duration, and easing on a decoration.

- `ArcadeButtonEffect.tsx:61`: button beam. Preserve source scale-x, opacity,
  duration, and easing on a decoration.

- `ArcadeButtonEffect.tsx:79`: button particles. Preserve seeded particle
  opacity, x, y, rotation, scale-x, duration multiplier, `index * .008s` delay,
  and `(.2,.82,.32,1)` easing.

- `ArcadeSliderEffect.tsx:24`: slider burst root. Key by activation and keep
  opacity at `1`.

- `ArcadeSliderEffect.tsx:38`: slider ring. Opacity `[.9,.65,0]`, scale
  `[.72,1.3,1.6]`, rotate `-16 -> 12`, base duration `.66s`, easing
  `(.16,.8,.35,1)`.

- `ArcadeSliderEffect.tsx:52`: slider particles. Preserve opacity, x, y,
  rotation, scale-x, duration multiplier, `index * .01s` delay, and
  `(.2,.85,.35,1)` easing.

- `ArcadeExitSequence.tsx:29`: exit flash. Opacity `[0,.7,.25,0]`, `.36s`,
  times `[0,.25,.65,1]`, ease-out.

- `ArcadeExitSequence.tsx:42`: expanding beam. Opacity `[0,.9,0]`, scale-y
  `[.2,1,.08]`, `.43s`, times `[0,.34,1]`, easing `(.2,.8,.2,1)`.

- `ArcadeExitSequence.tsx:57`: top line. Top `[7%,50%,50%]`, opacity
  `[0,.78,0]`, `.5s`, times `[0,.72,1]`, easing `(.7,0,.3,1)`.

- `ArcadeExitSequence.tsx:72`: bottom line. Bottom `[7%,50%,50%]`, opacity
  `[0,.78,0]`, `.5s`, times `[0,.72,1]`, easing `(.7,0,.3,1)`.

- `ArcadeExitSequence.tsx:87`: central collapse. Opacity `[0,0,1,.92,0]`,
  scale-x `[.08,.08,1,.32,.01]`, scale-y `[.5,.5,1.9,.5,.1]`, shared exit
  duration, times `[0,.52,.72,.87,1]`, ease-out.

- `MainMenu.tsx:101`: main content exit. Preserve the five-frame clip, filter,
  opacity, scale, and x targets, times `[0,.14,.38,.73,1]`, easing
  `(.65,0,.35,1)`, and shared exit duration. Reduced motion fades in `.08s`.

- `ScreenFrame.tsx:44`: frame exit. Preserve the five-frame clip, filter,
  opacity, scale, and x targets with the same clock as the main content. Its
  independent x values remain literal. Reduced motion fades in `.08s`.

Every source `useReducedMotion` branch is part of the checklist. Inherited
`ReducedMotion::User` supplies the normal mapping; a component whose source has
a custom static value uses `use_reduced_motion` and builds that value directly.
Manual review exercises each relevant pattern with motion enabled and reduced
motion forced.

Static gradients, glows, masks, and shadows may use ordinary styles, prepared
textures, custom UI Toolkit geometry, or a shader. These paint choices cannot
alter timing, presence, gesture, or reduced-motion behavior.

## Performance requirements

Animation performance is a release requirement rather than an optional
optimization. It is exercised manually before release and by an explicitly
triggered release or regression job when new evidence is needed. It does not
run in ordinary CI. The reference machine is a `Mac17,6` with an Apple M5 Max
and 64 GB memory running macOS `26.5.2` and Unity `6000.5.8f1`. The two required
profiles are:

- `macos-arm64-metal`: a non-development Release player, Apple silicon, Metal,
  VSync count `0`, target frame rate `64`, fixed `1280 x 720` framebuffer, and
  60 Hz display mode.
- `webgl-chrome-desktop`: a Release WebGL 2 build in Chrome
  `152.0.7977.65`, fixed `1280 x 720` backing framebuffer, device-pixel ratio
  normalized by the harness, WebAssembly threads disabled, and 60 Hz display
  mode.

Both use the production quality level and color-space settings. The reviewer
records the actual browser, operating system, Unity version, build flags, and
hardware with the profiler capture. A later run may use newer compatible tools
as long as its environment is recorded rather than silently compared with an
older result.

After warm-up, the motion frame loop must allocate no managed memory. Descriptor
installation may allocate when Reactant commits a changed tree; steady playback,
gesture sampling, scroll, and layout projection may not.

The complete `MotionWorld` update, including layout work it requests and command
submission, must remain below 4 milliseconds CPU time at the 95th percentile in
both workloads:

- `transform-200`: a fixed `20 x 10` grid of 200 hosts. Sixty run tweened x,
  y, scale, and opacity tracks; 60 run springs; 40 consume three-node
  motion-value graphs; and 40 alternate hover, press, and drag gestures from a
  deterministic input script.
- `mixed-200`: 150 hosts run the same transform/opacity mix. Ten hosts each run
  layout projection, color/filter, rectangular clip, decoration, and custom
  geometry animation, for 50 additional hosts.

The manual performance screen fixes host sizes, start phases, targets,
durations, spring parameters, and input order so separate runs exercise the
same work. Gesture events occur at controlled frame indices rather than
wall-clock callbacks.

The 30-second sample begins after five seconds of warm-up. CPU p95 uses all
sampled frames. The delivered average must be at least 59 frames per second;
99 percent of presentation intervals must be no longer than `18.337ms`, and no
interval may exceed `33.34ms`. The benchmark scene contains no unrelated game
simulation. Any managed allocation during the sample fails.

For each rendered frame, the CPU sample is the sum of profiler-recorder values
for Reactant pre-layout, the complete `UIElementsUpdatePanels` invocation, and
Reactant post-layout. Charging the complete panel update is intentionally
conservative in the dedicated scene. The p95 is the nearest-rank value at
one-based rank `ceil(0.95 * sample_count)` after sorting; there is no
interpolation or outlier removal.

Presentation intervals come from platform presentation clocks, not Unity game
time. The native harness records the macOS display-link host timestamp once per
presented frame. The WebGL template records the `requestAnimationFrame`
timestamp that invokes the Unity frame. The controller rejects duplicate,
non-monotonic, missing, background-tab, display-mode-change, or disjoint warm-up
and measurement samples. Average fps is `(sample_count - 1)` divided by the
elapsed time between the first and last accepted presentation timestamps.

The same screen also runs a 30-second mixed scenario cycling route, tab, modal,
control, burst, ambient, and audio-time patterns. It must meet the same
frame-pacing and allocation gates. This is the end-to-end GPU adequacy check;
Reactant does not claim a portable animation-owned GPU-time measurement on
desktop WebGL.

Profiler counters report active timelines, active layout tracks, graph nodes
evaluated, properties applied, native-optimized tracks, managed allocations,
motion CPU time, lifecycle messages, and lifecycle payload bytes. Per-frame
samples are absent without an explicit subscription. With subscriptions, there
is at most one coalesced sample per `(subscription, rendered frame)` plus
non-droppable ordered boundary events. Tests calculate this exact structural
upper bound from the active subscription and boundary counts.

### Running the release workloads

Open **Motion Performance** from the sample navigation and select
`TRANSFORM-200`, `MIXED-200`, or `INTERACTION`. `STEP` advances the fixed input
phase, `SUBSCRIBE` adds exactly one explicit presentation subscription, and
`RESET` restores `transform-200` with no subscription. The screen shows the
authored host, timeline, layout, graph, and subscription counts. CPU, frame,
allocation, property, native-optimization, and lifecycle values come from the
host counter rather than Rust frame callbacks.

Native harnesses read `BattlementRunner.MotionPerformance`. The returned
`BattlementMotionPerformanceSnapshot` is updated around the complete Motion
pre-layout, UI Toolkit panel, and post-layout span. Lifecycle payload bytes are
the actual serialized message size. Native players report the thread's exact
allocated bytes; WebGL reports positive managed-heap growth because its IL2CPP
profile does not expose the thread allocation counter. Leaving the screen removes its descriptors;
the next snapshot therefore returns active-work counters to the session
baseline. The checked-in structural smoke renders the full `transform-200`
tree under virtual time, while the five-second warm-up and complete 30-second
native and WebGL runs remain explicitly invoked release validation.

For a native retained profile, launch the Release executable with matching
`--reactant-performance=<scenario>`,
`--battlement-motion-scenario=<scenario>`, and
`--battlement-motion-profile=<absolute-json-path>` arguments. Supported scenario
values are `transform-200`, `mixed-200`, and `mixed-interaction`. WebGL enables
the same recorder with the `battlement-motion-profile=<scenario>` query
parameter; after the run, the JSON result is available from the document's
`data-battlement-motion-profile` attribute. Both paths wait for the first active
timeline or property, exclude the five-second warm-up, retain the complete next
30 seconds, and evaluate the documented gates without a Development player.

## Validation

### Rust tests

Black-box Reactant tests render components through the fake Battlement UI host
and inspect committed motion descriptors and emitted lifecycle behavior. They
cover:

- direct builder flattening without an extra host or logical position;
- target, transition, keyframe, variant, and property validation;
- equivalent lowering across cross-category host-method orders that preserve
  repeatable-layer order;
- generated catalog exhaustiveness and discrete-property behavior;
- typed pseudo-state precedence and CSS iteration-count semantics;
- variant lists, propagation, opt-out, custom data, stagger, and orchestration;
- Motion and CSS property ownership conflicts;
- inherited `MotionConfig` transition merging;
- presence modes, retained hook state, automatic exit, and manual holds;
- CSS animation restart, pause, fill, and composition behavior;
- callback batching, repeats, stale generations, cancellation, and unmount;
- motion-value graph identity, cycles, subscriptions, and coalescing;
- control queue replacement and scope selector snapshot semantics;
- layout-group identity, portal eligibility, and cross-panel rejection;
- reduced-motion target rewriting;
- full snapshot and reconnect reconstruction;
- atomic transaction rejection without partial Unity mutation; and
- audio-time discontinuities and derived expression behavior.

Tests assert behavior at public boundaries rather than private storage layout.
Developer-error cases assert their diagnostic's relevant host, property, or
variant identity.

### Unity sampler tests

Unity tests run the `MotionWorld` under a controlled clock. They cover:

- tween segment easing and exact keyframe boundaries;
- pre-layout capture and post-layout sampling around
  `PreLateUpdate.UIElementsUpdatePanels` without a second panel update;
- presentation, static baseline, and velocity capture before mutation;
- active-track baseline changes and interruption from visible presentation;
- per-property fallback for mismatched transition-level times;
- spring convergence, velocity handoff, interruption, and long-frame stability;
- inertia constraints and drag momentum;
- finite, reverse, mirror, alternate, delayed, and negative-delay repeats;
- core presentation color, filter, discrete, and structured interpolation used
  by Motion;
- layer priority and transform composition;
- graph evaluation and dirty propagation;
- pan, drag, hover, tap, and focus across mouse, pen, keyboard, and gamepad;
- scroll offsets, viewport entry, layout projection, and scale correction;
- catch-up after dropped frames and lifecycle ordering;
- native macOS and WebGL reduced-motion bridges, including live changes;
- renderer-capability rejection and property-specific paint behavior;
- allocation-free steady-state sampling; and
- native UI Toolkit transition equivalence for every eligible optimization.

Protocol integration tests exercise activation acknowledgements, coalesced
updates, repeat ranges, out-of-order old-session events, reconnect during
entrance and exit, scaled-time reconstruction, controlled time, and completion
while disconnected.

The full native and WebGL performance runs are manual or explicitly triggered.
Ordinary CI may contain cheap controlled-clock and allocation assertions, but
its cached critical path may grow by no more than 30 seconds for this project.
Record the pre-Task-01 release commit as the baseline and compare it with the
final staged implementation on the same machine and cache configuration. Warm
each tree once, then compare the median wall time of three unchanged-input runs.

## Manual QA

Use the Reactant animation sample made only from public Reactant builders. Do
not use direct C# animation calls.

1. Review every item in the mockup coverage ledger against the pinned source.
   Exercise each animation family with motion enabled and reduced motion
   forced. Compare values, keyframe order, timing, easing, repeats, delays,
   fill, and final state.
2. Rapidly interrupt the route, tab, modal, layout, spring, and gesture cases.
   Confirm each new track starts from the visible presentation and compatible
   springs retain velocity.
3. Exercise hover, press, focus, disabled, pan, constrained drag, external drag
   controls, snap-to-cursor, snap-to-origin, propagation, and reorder using
   mouse, keyboard, and gamepad where the interaction applies.
4. Trigger every keyed checkbox, button, and slider burst repeatedly. Confirm
   old generations cancel, exits finish once, and no retained hosts or callbacks
   remain.
5. Exercise the modal case. Confirm backdrop and panel exit before unmount, the
   three-frame exit uses even fallback spacing, mixed filter functions do not
   snap, decoration chrome skews while live content remains interactive, and
   the repeating shine stops after removal.
6. Leave ambient cases running. Confirm particles retain distinct negative
   phases, the grid alternates, border comets follow all corners, and loops do
   not restart in sync.
7. Toggle macOS Reduce Motion while the native player is mounted, then toggle
   the browser's `prefers-reduced-motion` emulation while WebGL is mounted.
   Confirm live spatial suppression and retained opacity/color feedback.
8. Disconnect and reconstruct during entrance, loop, pause, drag, and exit.
   Confirm phase restoration, no replayed completed entrance, one final unmount,
   and canceled pointer-owned gestures.
9. Exercise seek, reverse, pause, complete, stop, and cancel. Confirm values and
   slot-local lifecycle callbacks match their documented distinctions.
10. Run `transform-200`, `mixed-200`, and the mixed scenario on both reference
    profiles. Confirm CPU p95 below `4ms`, at least `59` average fps, interval
    gates, zero steady-state managed allocations, and the calculated lifecycle
    traffic bound. Record the environment and retain the profiler captures.

Linear background gradients preserve every authored stop. The native painter
divides gradients longer than Unity’s eight-key limit into adjacent clipped
sections without discarding colors.
