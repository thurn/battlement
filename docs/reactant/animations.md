# Reactant Animations

Reactant Animations is the animation, gesture, and layout-projection system for
Reactant user interfaces. It gives every existing Battlement UI primitive a
Motion-inspired builder API while preserving Reactant's Rust component model and
Unity UI Toolkit host tree.

The authoring goal is mechanical familiarity. A typical component written with
Motion for React should translate into Rust one expression at a time. Reactant
does not introduce animated copies of every host type. `Button`,
`VisualElement`, `Label`, and the other existing primitives receive animation
builders through a sealed extension trait imported by the Reactant prelude.

The execution goal is a stable 60 frames per second on mobile and WebGL. Rust
declares animation state. Unity recognizes gestures, evaluates motion-value
graphs, samples timelines, projects layout, and applies final UI Toolkit values
on every rendered frame. Animation does not require a Rust render or a network
exchange per frame.

## Related information

- [Reactant technical design](reactant-technical-design.md) defines sessions,
  commits, snapshots, and the Rust-to-Unity boundary extended here.
- [Components and rendering](component-authoring.md) defines host primitives,
  sealed render values, keys, and the focused prelude.
- [Hooks and effects](hooks-and-effects.md) defines positional hooks and stable
  hook-owned handles.
- [Reconciliation, events, and portals](reconciliation-events-and-portals.md)
  defines logical identity, host identity, event ordering, and portals.
- [Refs and geometry](refs-geometry-and-floating-ui.md) defines `ElementRef`,
  asynchronous geometry, and panel ownership.
- [Generated UI assets](asset-generator.md) defines the static paint compiler
  used by animation hosts and decoration layers.
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
  panel-attached frame scheduler.

[motion-react]: https://motion.dev/docs/react
[motion-layout]: https://motion.dev/docs/react-layout-animations
[settings-mockup]:
  https://github.com/thurn/mockups/tree/2451ea9cc6f76b356b1102ee37b82c478853122a
[unity-transitions]:
  https://docs.unity3d.com/Manual/UIE-Transitions.html
[unity-scheduler]:
  https://docs.unity3d.com/ScriptReference/UIElements.VisualElement.html

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

The contract covers UI hosts and typed UI values. Browser-only DOM mutation,
arbitrary CSS selectors, SVG path machinery, and runtime CSS string parsing are
not part of Reactant. Equivalent UI effects use typed values, decoration
layers, generated assets, or a purpose-built Unity rendering implementation.

Motion APIs whose purpose is to run JavaScript on every browser frame become
Unity-local graphs rather than Rust frame callbacks. `use_time`, motion-value
expressions, and explicit coalesced subscriptions cover those cases.
`LazyMotion` has no counterpart because Rust and Unity animation code is linked
at build time. Browser view transitions have no counterpart; Reactant presence
and shared layout animate the live UI Toolkit hosts instead.

The pinned settings mockup is the minimum feature bar. Its animation patterns
are listed in
[Mockup translation coverage](#mockup-translation-coverage). The design also
covers common Motion APIs that the mockup does not happen to use, so application
code does not need another animation subsystem as it grows.

## Authoring model

### Existing hosts gain animation builders

`MotionHostExt` is a sealed extension trait implemented for every Reactant host
primitive. It is part of `battlement_reactant::prelude`. Applications use its
methods directly and never name the internal render adapter returned by a
builder.

```rust
Button::new("Settings")
    .animate(MotionStyle::new().y(0.0).scale(1.0))
    .while_hover(MotionStyle::new().y(-1.0))
    .while_tap(MotionStyle::new().scale(0.955))
    .transition(
        Transition::spring()
            .stiffness(520.0)
            .damping(32.0)
            .mass(0.7),
    )
```

Every method returns another value implementing the sealed host-builder and
`Render` traits. Lowering flattens the adapter into the same host node as the
original primitive. It creates no wrapper `VisualElement`, changes no physical
parent, and consumes no additional logical sibling position.

Unity may attach private paint resources such as decoration meshes or a
projection surface to implement a requested property. Those resources are not
Reactant hosts and do not change logical hierarchy, input, layout, or focus.

There are no `MotionButton`, `MotionVisualElement`, or `Motion::new` APIs.
Reactant defines no animation macro. Ordinary Rust types, closures, enums, and
builders provide the complete authoring surface.

### Motion targets

`MotionStyle` contains optional typed target values. A missing property does
not participate in that target layer. `MotionTarget` combines a style with an
optional transition and orchestration metadata.

```rust
pub struct MotionStyle { /* private fields */ }
pub struct MotionTarget { /* private fields */ }
pub enum InitialTarget<Name = NoVariant> {
    Target(MotionTarget),
    Variant(Name),
    Disabled,
}

impl MotionTarget {
    pub fn new(style: MotionStyle) -> Self;
    pub fn transition(self, value: Transition) -> Self;
    pub fn transition_end(self, value: MotionStyle) -> Self;
}
```

`animate`, `exit`, and gesture target builders accept
`impl Into<MotionTarget>`, and `MotionStyle` converts directly. `initial`
accepts `impl InitialValue`, a sealed input implemented only for `bool`,
`MotionStyle`, and `MotionTarget`. The distinct `initial_variant(name)` builder
selects `InitialTarget::Variant`, avoiding overlapping generic
`From<MotionTarget>` and
`From<Name>` implementations. `false` selects `Disabled`; `true` is a developer
error because it has no Motion meaning. Application variant-name types never
implement `InitialValue`.

Every base host starts with the sealed `NoVariant` name type. Calling
`.variants(...)` changes the adapter's name type while preserving a previously
selected disabled or concrete initial target. This gives `.initial(false)` a
known type even when it appears before `.variants(...)` and prevents an
unconstrained generic at the call site.

```rust
VisualElement::new()
    .initial(MotionStyle::new().opacity(0.0).x(-17.0))
    .animate(MotionStyle::new().opacity(1.0).x(0.0))
    .exit(
        MotionTarget::new(MotionStyle::new().opacity(0.0).x(10.0))
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

An unspecified transition uses these property defaults:

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
fallback are normative and are ported directly into both samplers. A
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

VisualElement::new()
    .animate(
        MotionStyle::new()
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
Button::new("Delete")
    .style(Style::new().background_color(Color::hex("#15121a")))
    .style_transition(
        StyleTransition::new()
            .property(StyleProperty::BackgroundColor)
            .duration_secs(0.18)
            .ease(Easing::EaseOut),
    )
    .hover_style(Style::new().background_color(Color::hex("#2a121c")))
```

The transition observes changes to the resolved static style from Reactant
renders and supported UI pseudo-states. It never observes sampled Motion output
or `transition_end` assignments.

Multiple properties may be declared with distinct duration, delay, and easing
values. `StyleTransition::all()` expands at validation time to every changed
interpolable property and never includes discrete properties implicitly.

Discrete transitions require `.allow_discrete(true)`. They switch at 50 percent
of the active interval. `Display::None` and hidden visibility switch at the end
when disappearing and at the start when appearing, matching their useful CSS
behavior. Without `allow_discrete`, a discrete static change applies
immediately.

### Reusable CSS-style animations

`Keyframes<MotionStyle>` is an ordinary cloneable Rust value. `Animation`
applies it with CSS-style playback settings. There is no global string registry
and no runtime lookup by animation name.

Keyframes normalize independently per property. Interior frames that omit a
property do not create a sample for it; interpolation spans the nearest earlier
and later frames that declare it. If the first or last declared frame is not at
zero or one, Reactant inserts the resolved underlying property value at that
endpoint. The underlying value is captured from lower-priority layers when the
animation generation is installed. A property omitted from every frame creates
no track. This matches the CSS keyframe rule without turning a missing
`MotionStyle` field into an implicit hold.

```rust
fn grid_breathe() -> Keyframes<MotionStyle> {
    Keyframes::new([
        MotionStyle::new().y(-6.0).scale_y(0.96).opacity(0.58),
        MotionStyle::new().y(12.0).scale_y(1.02).opacity(1.0),
    ])
}

VisualElement::new()
    .animation(
        Animation::new(grid_breathe())
            .duration_secs(5.2)
            .ease(Easing::EaseInOut)
            .repeat(Repeat::Forever)
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
Button::new("Upgrade")
    .before(
        Decoration::new()
            .position(DecorationPosition::Fill)
            .background(premium_shine_gradient())
            .animation(
                Animation::new(shine_sweep())
                    .duration_secs(2.4)
                    .repeat(Repeat::Forever),
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

Decorations support `Style`, `MotionStyle`, `StyleTransition`, `Animation`, and
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
            MotionStyle::new()
                .opacity(0.0)
                .x(*direction as f32 * 58.0)
                .scale(0.99),
        )
    })
    .target(
        TabVariant::Center,
        MotionTarget::new(
            MotionStyle::new().opacity(1.0).x(0.0).scale(1.0),
        )
        .transition(
            Transition::tween()
                .duration_secs(0.36)
                .ease(Easing::CubicBezier([0.16, 1.0, 0.3, 1.0])),
        ),
    )
    .resolver(TabVariant::Exit, |direction| {
        MotionTarget::new(
            MotionStyle::new()
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

VisualElement::new()
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

`MotionStyle` covers every public Battlement UI style property plus Motion
aliases and the additional visual values required by the mockup. The existing
host-style schema generates an exhaustive `MotionProperty` catalog. Every
catalog entry declares its Rust value type, canonical unit, initial value,
interpolation category, percentage reference box, additive rule, wire encoding,
and Unity writer. Generation fails when any public style property lacks an
entry. Each generated builder and serializer uses that same entry, so the Rust
and Unity catalogs cannot drift.

Interpolated values include:

- finite scalars, opacity, flex values, and numeric text metrics;
- lengths, percentages, angles, transform origins, and radii;
- Motion-compatible RGBA colors;
- translate, rotate, scale, skew, and ordered transform lists;
- compatible gradients, shadows, filters, clips, and masks; and
- layout insets, sizes, gaps, padding, margins, and border widths.

`MotionLength` preserves `Px`, `Percent`, and typed `Calc` components. Mixed
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

VisualElement::new()
    .motion_style(
        MotionStyle::new()
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
pub fn use_scroll(value: ScrollOptions) -> ScrollMotionValues;
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
clamp, wrap, color mixing, lengths, and transforms. Reactant does not pretend
that an arbitrary Rust closure can execute in Unity. Application-specific Rust
calculations require an explicit value subscription and therefore update only
at the Rust exchange rate.

Motion's `useMotionTemplate` translates to `use_motion_expression` with typed
`TransformList`, `FilterList`, gradient, or length expression nodes. There is no
formatted runtime CSS string. Motion's `useWillChange` has no value-level API in
Reactant because `MotionWorld` automatically prepares and releases property
writers, meshes, and projection surfaces for every active descriptor. Removing
that source line is the mechanical translation.

The Unity graph is acyclic. Reactant validates dependencies and reports a cycle
before commit. Unity evaluates only dirty nodes, once per frame, in topological
order. A graph may feed any number of hosts without extra Rust work.

`MotionValue::set`, `jump`, `stop`, and `animate` are allowed in event handlers,
effects, and engine-thread application callbacks. They panic during render.
`set` retargets through an attached passive effect when a component needs to
synchronize an external value.

`MotionValue::get` returns the last value observed by Rust and may lag one
exchange. It is forbidden during render because using a sampled client value as
render input would create an implicit subscription and unstable feedback loop.

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

VisualElement::new()
    .element_ref(target)
    .motion_style(MotionStyle::new().opacity_value(opacity))
    .while_in_view(MotionStyle::new().y(0.0).opacity(1.0))
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
            .animate(MotionStyle::new().opacity(1.0))
            .exit(MotionStyle::new().opacity(0.0))
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

VisualElement::new()
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
VisualElement::new()
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
                    VisualElement::new()
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

Every host supports `while_hover`, `while_tap`, `while_focus`, and
`while_drag`, with target or named-variant forms. Corresponding start, end, and
cancel callbacks use ordinary Reactant event handlers.

```rust
Button::new("Apply")
    .while_hover(MotionStyle::new().y(-1.0).filter_brightness(1.08))
    .while_tap(MotionStyle::new().scale(0.96))
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
VisualElement::new()
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

VisualElement::new()
    .element_ref(constraints.clone())
    .child(
        VisualElement::new()
            .drag(DragAxis::Both)
            .drag_constraints(DragConstraints::element(constraints))
            .drag_elastic(DragElastic::sides(0.12, 0.12, 0.08, 0.08))
            .drag_momentum(true)
            .drag_direction_lock(true)
            .while_drag(MotionStyle::new().scale(1.03))
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

`ReorderGroup<T>` and `ReorderItem<T>` are specialized logical components
because reorder needs collection semantics, not animated copies of host types.

```rust
ReorderGroup::new(Axis::Vertical, items.clone())
    .on_reorder(move |game: &mut Game, order| game.set_order(order))
    .children(items.into_iter().map(|item| {
        ReorderItem::new(item.id, PlayerRow::new(item)).key(item.id)
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

`use_animation_controls()` returns a stable `AnimationControls` handle. A host
binds it with `.animation_controls(controls.clone())`.

```rust
let controls = use_animation_controls();
let click_controls = controls.clone();

Button::new("Replay")
    .on_click(move |_game: &mut Game| {
        click_controls.start(
            MotionTarget::new(
                MotionStyle::new()
                    .scale_keyframes(Keyframes::new([1.0, 1.08, 1.0])),
            )
            .transition(Transition::tween().duration_secs(0.28)),
        );
    });

VisualElement::new()
    .animation_controls(controls)
    .animate(MotionStyle::new().scale(1.0))
```

Controls may start a target or named variant, set a presentation value without
animation, and stop all controlled tracks. Starting before the host is attached
queues the latest target for that binding. Starting after detach is a no-op.

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
let scope = use_animation_scope();
let effect_scope = scope.clone();

use_effect(
    move || {
        effect_scope.start(
            AnimationSequence::new()
                .animate(
                    MotionSelector::name("row"),
                    MotionStyle::new().opacity(1.0).x(0.0),
                    Transition::tween().duration_secs(0.2),
                )
                .then(
                    MotionSelector::name("badge"),
                    MotionStyle::new().scale(1.0),
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
signed offset from a named label. Targets are snapshotted when their sequence
step becomes eligible. A target unmounted before its step is skipped; an active
target unmounted during its step is cancelled. Empty selector results are valid
and complete immediately.

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

The handle also implements Reactant's single-threaded completion future. Its
future resolves to `PlaybackOutcome::Completed`, `Stopped`, or `Cancelled`.
After any terminal operation, later commands on that playback handle are
no-ops. Dropping the handle does not cancel playback.

Declarative hosts provide `on_animation_start`, `on_animation_update`,
`on_animation_repeat`, `on_animation_complete`, `on_animation_stop`, and
`on_animation_cancel`. Repeat includes the completed iteration range, direction,
and logical elapsed time. Update is an explicit subscription and is coalesced
once per rendered frame. Lifecycle callbacks are ordered by animation layer,
host preorder, and slot identity.

## Reduced motion

`MotionConfig` establishes inherited defaults without adding a Unity host.

```rust
MotionConfig::new(app)
    .transition(Transition::spring().stiffness(420.0).damping(32.0))
    .reduced_motion(ReducedMotion::User)
    .time_source(MotionTime::Unscaled)
```

The optional transition is the inherited default for descendants that do not
declare one. A closer `MotionConfig`, a host transition, or a target-local
transition overrides it in that order. Property overrides merge by property
rather than replacing the complete inherited transition.

`ReducedMotion` is `User`, `Always`, or `Never`. `User` follows the Unity
platform accessibility setting and updates mounted subtrees when that setting
changes. `use_reduced_motion()` returns the resolved boolean for a component
that wants a custom fallback.

The automatic reduced-motion policy suppresses transform, layout projection,
drag momentum, and scroll-linked movement. Those properties jump to the target.
Opacity, color, background, and other non-spatial transitions continue. An
application may branch on `use_reduced_motion()` to replace any target or
animation explicitly.

```rust
let reduce_motion = use_reduced_motion();

Panel::new().animate(if reduce_motion {
    MotionStyle::new().opacity(1.0)
} else {
    MotionStyle::new().opacity(1.0).x(0.0)
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
adapter slot, and committed generation. Reusing a host and slot updates the
descriptor. Changing a keyed animation entry preserves its timeline identity.
Removing an entry cancels that timeline unless presence retains it.

Descriptors are values in Reactant's work-in-progress tree. Render validation
finishes before any motion command is emitted. A failed or suspended render
retains the previous committed descriptors and Unity timelines unchanged.

### Commit ordering

One Reactant commit orders animation work as follows:

1. create hosts and apply non-animated static properties;
2. move and reparent hosts;
3. install or update motion descriptors and value graphs;
4. mark pre- and post-layout measurement participants;
5. remove hosts whose presence lifecycle is complete; and
6. perform queued host actions.

Unity applies the command group atomically on its engine thread. It captures
required old geometry before the first mutation, runs at most one required
layout pass after the mutations, installs projection, and publishes the first
new presentation sample before painting.

An update can therefore animate a newly mounted host on its first visible frame
and project an existing host without an intermediate flash at its final layout.

### Unity motion world

Each UI panel owns a `MotionWorld`. It contains compact arrays for timelines,
tracks, graph nodes, gestures, layout projections, and dirty host outputs.
Stable IDs index generation-checked slots; destroyed slots return to free lists.

One panel scheduler callback samples every active item immediately before UI
Toolkit rendering. The callback:

1. advances the selected clocks;
2. recognizes pending gesture and scroll changes;
3. evaluates dirty motion-value graph nodes;
4. samples timelines and layout projections;
5. resolves layer priority and transform composition;
6. applies changed final values in property batches; and
7. queues coalesced sample and lifecycle messages.

Steady-state sampling allocates no managed memory. Keyframe arrays, graph edges,
callbacks, property writers, and scratch buffers are prepared when descriptors
change. Hosts with unchanged resolved output receive no UI Toolkit assignment.

Layout-affecting values are applied together before one requested layout pass.
Presentation-only values follow that pass. Reactant never alternates a layout
read and write for each animated element.

### Clocks and dropped frames

Unscaled time is the default. `.time_source(MotionTime::Scaled)` opts a subtree,
animation, or imperative start into Unity game time. The nearest setting wins.
Pause and playback speed multiply the selected clock without changing its
source.

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

The generated-asset system produces immutable textures with no runtime
parameters. Reactant Animations may:

- animate the transform, opacity, tint, filter, clip, or layout of a generated
  texture host;
- use generated variants as discrete keyframes;
- place a generated texture in a decoration layer; or
- drive a separately implemented UI Toolkit custom renderer or shader through
  typed style and motion-value bindings.

The animation design does not add runtime inputs to the asset generator and
does not accept arbitrary shader-property strings. A custom renderer exposes
typed properties through the same property catalog before those values can
participate in `MotionStyle`.

The properties required by the settings mockup have concrete runtime paths:

- inset clips use nested overflow-mask elements whose edges are sampled as
  ordinary layout values;
- static polygon clips use generated or authored vector masks;
- animated polygon clips use cached custom mesh geometry and update only the
  affected vertex buffer;
- decorative skew uses a cached custom quad mesh with sampled vertices;
- subtree skew, blur, brightness, saturation, and blend effects use a pooled
  `ProjectionSurface` that composites the affected logical subtree through a
  prepared material; and
- gradients, glows, shadows, and masks use ordinary styles when supported and
  generated textures or cached custom geometry otherwise.

`ProjectionSurface` is a private Unity paint primitive. It does not become a
Reactant host, logical child, event target, focus target, or layout participant.
The original host retains input and geometry. While active, the surface owns
painting for that host subtree and receives its projection transform, clip, and
opacity. The runtime pools its render targets by panel, dimensions, and format
and releases them when the last requiring track completes.

Reactant selects a projection surface only for a property combination whose
subtree semantics cannot be expressed by UI Toolkit styles, masks, or cached
custom geometry. The mockup's full-screen CRT transition and skewed modal are
required conformance fixtures for this path. Surface capture and compositing are
included in the animation-owned CPU and GPU budgets.

Missing assets, incompatible prepared-asset kinds, or unsupported renderer
bindings remain host contract failures. Reactant does not silently replace an
effect with a lower-fidelity animation.

## Mockup translation coverage

The settings mockup establishes concrete acceptance cases. Translation follows
the source construct instead of forcing every effect through one API:

- A `motion.*` element becomes the same Reactant host with Motion builders.
- A state-dependent CSS `transition` becomes `StyleTransition` plus typed
  static and pseudo-state styles.
- A CSS `@keyframes` and `animation` pair becomes a `Keyframes<MotionStyle>` and
  `Animation` value.
- A purely visual span becomes a decoration when it needs no child content,
  layout, input, accessibility, or ref; otherwise it remains an ordinary host.

Every animation declaration in the mockup maps as follows:

- `SettingsTabs` translates tab y and scale targets, hover, tap, and the exact
  stiffness `520`, damping `32`, and mass `0.7` spring onto each `Button`.
- `SettingsControls` translates dropdown presence, menu y and scale-y entry and
  exit, staggered option x and opacity, selected-option flash, toggle-label
  replacement, and the control press, filter, shadow, and transform style
  transitions.
- `SoundSettings` translates its slider, thumb, checkbox, and button state
  changes with `StyleTransition`. Its release bursts remain keyed Motion hosts;
  it does not require a declarative Motion target on the settings component.
- `InputSettings` uses the same control transitions and release effects, plus
  an infinite linear five-frame opacity `Animation` for the input-binding blink.
- `ActionButton` maps pressed transform, focus, hover, filter, and background
  changes to gesture layers and `StyleTransition`. `ControlInteraction` maps the
  global three-frame `control-shine-sweep` to a finite decoration `Animation`
  with `Both` fill, not to a style transition.
- `ArcadeRouteTransition` becomes application state under `MotionConfig`; its
  system-or-application reduced-motion value is inherited by both routes.
- `ArcadeTabTransition` uses custom directional variants,
  `AnimatePresence::custom`, `PopLayout`, a finite light-sweep child, and the
  scan-line y and opacity keyframes with explicit times.
- `ArcadeMenuTransition` uses `Sync` or `Wait` presence, screen variants, and
  contained scan, clip, opacity, scale-x, and beam keyframes. The WebKit branch
  maps to the same conservative `Wait` mode when the selected Unity backend
  cannot safely overlap the two projection surfaces.
- `ArcadeExitSequence` maps the flash, expanding beam, top line, bottom line,
  and central collapse to five ordinary children with their exact keyframes,
  times, durations, and cubic Bézier easing.
- `MainMenu` and `ScreenFrame` map their coordinated exit surfaces to clip,
  filter, opacity, scale-x, scale-y, and x keyframes. They share the same
  logical clock so the frame and content collapse cannot drift apart.
- `ArcadeModal` uses presence for backdrop and panel, four-frame entry and
  three-frame exit targets, Motion-compatible cross-function filter mixing, and
  an infinite linear shine with repeat delay. Its four transition times fall
  back to even spacing on each three-frame exit property, matching Motion.
- `ArcadeButtonEffect` and `ArcadeSliderEffect` remain keyed finite Motion
  subtrees after their first trigger. A new burst key replaces the old subtree.
  Ring, beam, particle, scale, rotation, translation, and opacity values map
  property for property.
- `ArcadeCheckboxEffect` uses `AnimatePresence` for the checked-state burst and
  finite ring, flash, beam, and spark children, including its short exit.
- `ArcadeAttractMode` maps perspective-grid breathing to alternate reusable
  keyframes and maps all 48 particles to keyed decorations with per-instance
  duration, drift, size, color, and negative phase delay.
- `ArcadeFramePulse` maps both border beams to the same infinite nine-frame
  left, top, and rotation `Animation`; state selects the settings cutout mask.
- Every source `useReducedMotion` branch maps to inherited
  `ReducedMotion::User` behavior or an explicit `use_reduced_motion` branch with
  the same static fallback value.

The directional tab variant translates without stringly typed style data:

```rust
AnimatePresence::new()
    .initial(false)
    .custom(direction)
    .mode(PresenceMode::PopLayout)
    .child(
        ArcadeTabPanel::new(active_tab, direction)
            .key(active_tab)
            .variants(tab_variants())
            .initial_variant(TabVariant::Enter)
            .animate_variant(TabVariant::Center)
            .exit_variant(TabVariant::Exit),
    )
```

The ambient particle loop preserves the mockup's negative phase offset:

```rust
VisualElement::new().before_all(particles.into_iter().map(|particle| {
    Decoration::new()
        .position(DecorationPosition::Point(
            Percent::new(particle.x),
            Percent::new(particle.y),
        ))
        .size(Size::square(Pixels::new(particle.size)))
        .background(Color::hex(particle.color))
        .animation(
            Animation::new(particle_drift(particle.drift_x, particle.drift_y))
                .duration_secs(particle.duration)
                .delay_secs(-particle.phase * particle.duration)
                .ease(Easing::Linear)
                .repeat(Repeat::Forever),
        )
}))
```

The border comet uses interpolated edges and short corner rotations in one
typed timeline:

```rust
Keyframes::new([
    frame(0.00, 0.00, 0.0),
    frame(1.00, 0.00, 0.0),
    frame(1.00, 0.00, 90.0),
    frame(1.00, 1.00, 90.0),
    frame(1.00, 1.00, 180.0),
    frame(0.00, 1.00, 180.0),
    frame(0.00, 1.00, 270.0),
    frame(0.00, 0.00, 270.0),
    frame(0.00, 0.00, 360.0),
])
.times([0.0, 0.24, 0.25, 0.49, 0.5, 0.74, 0.75, 0.99, 1.0])
```

Static gradients, glows, clipped frames, masks, and shadows may come from
ordinary supported styles, generated assets, custom UI Toolkit geometry, or a
shader. The chosen paint technique does not change timeline, presence, gesture,
or reduced-motion behavior.

## Performance requirements

Animation performance is a release requirement rather than an optional
optimization. The designated project benchmark matrix must include a mid-tier
mobile target and the production WebGL browser configuration.

The matrix is project-owned, but every runnable profile is a checked-in manifest
that records its stable profile ID, device model, CPU, GPU, memory, operating
system, Unity version, editor or player build, browser and version when
applicable, resolution, device-pixel ratio, refresh rate, quality settings, and
build flags. Results include the manifest hash and fail if any declared field is
missing or differs from the executing environment. This keeps the requirement
reproducible without baking short-lived hardware names into this design.

After warm-up, the motion frame loop must allocate no managed memory. Descriptor
installation may allocate when Reactant commits a changed tree; steady playback,
gesture sampling, scroll, and layout projection may not.

The animation subsystem must remain below 4 milliseconds of CPU time and below
4 milliseconds of animation-owned GPU time at the 95th percentile under both
of these workloads:

- 200 concurrent transform and opacity animations with mixed tweens, springs,
  motion-value dependencies, and gesture activation; and
- 50 concurrent mixed layout, color, filter, clip, decoration, and paint
  animations in addition to 150 transform and opacity tracks.

Each required profile runs at 60 Hz for 30 seconds after a five-second warm-up.
The p95 is calculated over all measured frames. The delivered average must be
at least 59 frames per second, at least 99 percent of presentation intervals
must be no longer than `1.1` times one 60 Hz interval, and no interval may
exceed two intervals. The dedicated benchmark scene contains no unrelated game
simulation, so these are end-to-end frame-pacing gates rather than
animation-loop estimates.

Profiler markers bound the complete `MotionWorld` update, layout work requested
by that update, `ProjectionSurface` capture, and animation command submission.
CPU time includes waiting for a required surface to become ready. A separate GPU
counter, measured with completion fences and reported beside CPU time, covers
surface rendering and compositing. GPU timing must come from the target player
or browser's supported profiler or timer-query path; a profile without valid GPU
timing fails rather than silently omitting the gate. Any managed allocation
during the measured interval also fails the profile.

The settings screen must also maintain the target display rate with all ambient
effects enabled while repeatedly switching tabs, opening and closing the modal,
changing controls, and returning to the menu. Reduced motion is a behavior, not
a performance fallback, and is tested separately.

Profiler counters report active timelines, active layout tracks, graph nodes
evaluated, properties applied, native-optimized tracks, managed allocations,
motion CPU time, and lifecycle payload size. Counters are diagnostics and do not
change public animation behavior.

## Validation

### Rust tests

Black-box Reactant tests render components through the fake Battlement UI host
and inspect committed motion descriptors and emitted lifecycle behavior. They
cover:

- direct builder flattening without an extra host or logical position;
- target, transition, keyframe, variant, and property validation;
- variant lists, propagation, opt-out, custom data, stagger, and orchestration;
- Motion and CSS property ownership conflicts;
- inherited `MotionConfig` transition merging;
- presence modes, retained hook state, automatic exit, and manual holds;
- CSS animation restart, pause, fill, and composition behavior;
- callback batching, repeats, stale generations, cancellation, and unmount;
- motion-value graph identity, cycles, subscriptions, and coalescing;
- layout-group identity, portal eligibility, and cross-panel rejection;
- reduced-motion target rewriting; and
- full snapshot and reconnect reconstruction.

Tests assert behavior at public boundaries rather than private storage layout.
Developer-error cases assert their diagnostic's relevant host, property, or
variant identity.

### Unity sampler tests

Unity tests run the `MotionWorld` under a controlled clock. They cover:

- tween segment easing and exact keyframe boundaries;
- per-property fallback for mismatched transition-level times;
- spring convergence, velocity handoff, interruption, and long-frame stability;
- inertia constraints and drag momentum;
- finite, reverse, mirror, alternate, delayed, and negative-delay repeats;
- Motion-compatible color, filter, discrete, and structured interpolation;
- layer priority and transform composition;
- graph evaluation and dirty propagation;
- pan, drag, hover, tap, and focus across mouse, touch, keyboard, and gamepad;
- scroll offsets, viewport entry, layout projection, and scale correction;
- catch-up after dropped frames and lifecycle ordering;
- allocation-free steady-state sampling; and
- native UI Toolkit transition equivalence for every eligible optimization.

Protocol integration tests exercise activation acknowledgements, coalesced
updates, repeat ranges, out-of-order old-session events, reconnect during
entrance and exit, scaled-time reconstruction, controlled time, and completion
while disconnected.

Performance tests run the required workload matrix and fail on the CPU budget,
GPU budget, average delivered frame rate, frame-pacing limits, invalid timing
data, or any steady-state managed allocation.

## Manual QA

Use a Unity runtime UI built with the public Reactant builders. Do not
substitute direct C# animation calls during this review.

1. Open the settings screen from the main menu. Confirm the CRT effect, screen
   clip, filters, beam, and content transition overlap in the intended order and
   settle without a flash at either endpoint.
2. Move through every settings tab in both directions. Confirm directional
   variants, `PopLayout`, sweep, scan line, scroll containment, and rapid
   interruption from the current visible presentation.
3. Hover, press, keyboard-focus, and gamepad-submit every control type. Confirm
   gesture priority, spring return, filters, focus visibility, and the finite
   control-shine keyframes with `Both` fill.
4. Toggle settings, change a slider repeatedly, and trigger checkbox and button
   effects. Confirm every keyed burst finishes, exits, and releases its retained
   subtree without leaving hosts or callbacks alive.
5. Open and close the modal by every supported route. Confirm backdrop and panel
   exits finish before unmount, the three-frame exit ignores the incompatible
   four-entry times and uses even spacing, cross-function blur and brightness
   segments interpolate without a midpoint snap, focus follows application
   policy, and the repeating shine stops after removal.
6. Leave the screen idle. Confirm 48 particles retain distinct negative phases,
   the grid alternates, the border comet follows every corner, and repeating
   effects do not drift into synchronized restarts.
7. Interrupt route, tab, modal, drag, and layout animations at several points.
   Confirm each new target begins at the current presentation value and springs
   preserve compatible velocity.
8. Exercise pan, drag constraints, external drag controls, snap-to-cursor,
   snap-to-origin, propagation, and reorder with mouse, touch, and gamepad where
   applicable. Confirm local presentation remains responsive while Rust accepts
   or corrects the semantic result.
9. Enable system reduced motion while the screen is mounted. Confirm spatial,
   layout, scroll-linked, and momentum motion stops while opacity and color
   feedback remains, and custom fallbacks update immediately.
10. Disconnect and reconstruct the Unity session during an entrance, loop,
    paused playback, drag, and exit. Confirm timelines resume at their specified
    phase, completed entrances do not replay, exits still unmount once, and
    pointer-owned gestures cancel.
11. Run controlled time, seek, reverse direction, pause, complete, stop, and
    cancel through the imperative playback API. Confirm presentation values and
    lifecycle callbacks match the documented distinctions.
12. Profile the settings screen and both stress workloads on the designated
    mobile and WebGL targets. Confirm both 4 ms p95 budgets, at least 59 average
    frames per second, the presentation-interval limits, valid GPU timings, zero
    steady-state managed allocations, and bounded lifecycle traffic.
