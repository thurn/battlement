# Reactant Components and Rendering

This appendix defines how application code describes a Reactant tree. It is part
of the [Battlement Reactant technical design](reactant-technical-design.md) and
assumes the protocol hosts from the
[Battlement UI technical design](../battlement-ui-technical-design.md) and the
Reactant authoring types from [Host façades](host-facades.md).

## Related information

- [React: describing the UI](https://react.dev/learn/describing-the-ui) explains
  component composition and purity in React.
- [React: rendering lists](https://react.dev/learn/rendering-lists) explains the
  purpose of keys and stable list identity.
- [React: error boundaries][react-error-boundaries] explains the
  nearest-boundary fallback behavior adapted here to explicit Rust `Result`
  values.
- [Hooks and effects](hooks-and-effects.md) defines state read during rendering.
- [Reconciliation, events, and portals](reconciliation-events-and-portals.md)
  defines how rendered values retain identity and become Unity commands.
- [Host façades](host-facades.md) defines order-independent host builders and
  private lowering to `Ui`-prefixed protocol types.

[react-error-boundaries]: https://react.dev/reference/react/Component#catching-rendering-errors-with-an-error-boundary

Host preparation finishes before descendant rendering begins. Nested host
compositions use ordinary runtime thread stacks; applications do not need to
flatten component boundaries or configure larger stacks.

## Component structs

`Render` is a sealed trait for values Reactant can lower into an internal node
sequence. Its lowering method is private, so applications compose supported
values rather than implementing host protocol behavior.

```rust
pub trait Render: private::Sealed + 'static {}
```

A component is an owned struct implementing `Component`. Its fields are props.
Reactant does not support function components.

Ordinary authoring may import `battlement_reactant::prelude::*`. The focused
prelude contains component and render traits, hooks, structural render values,
boundaries, refs, adapters, and common Reactant host façades. Runtime
administration, protocol messages, command composition, and executor types
remain explicit imports.

This focused module is Reactant's sole exception to the repository rule against
public re-exports. The exception applies only to authoring essentials in this
module; it does not permit additional crate-root or convenience re-exports.

```rust
pub trait Component: 'static {
    fn render(&self) -> impl Render;
}
```

The output is opaque, so a component does not declare an associated output type
or erase its own render value. Reactant supplies an internal blanket adapter
for `T: Component`. The object-safe adapter calls `render` and immediately
lowers the owned output while its concrete type is available.

```rust
trait ErasedComponent {
    fn render_into(&self, output: &mut RenderOutput);
}
```

Owned component render values move into `Box<dyn ErasedComponent>` when the
parent output is lowered. The adapter, boxes, and `RenderOutput` are private.

```rust
pub struct PlayerName {
    name: String,
}

impl Component for PlayerName {
    fn render(&self) -> impl Render {
        Label::new(self.name.clone())
    }
}
```

All mounted values are owned and `'static`. A component may borrow local data
while constructing owned props, but it cannot retain that borrow in the virtual
tree.

```rust
PlayerName::new(player.name().to_owned())
```

Reactant does not require every component to implement `Clone` or `PartialEq`.
Those bounds belong only to APIs that copy or compare a value, such as state,
dependencies, context values, and root view factories that choose to clone data.

## Memoized component boundaries

`memo` opts one component into prop comparison and subtree bailout.

```rust
pub struct Memo<C> { /* private fields */ }

pub fn memo<C>(component: C) -> Memo<C>
where C: Component + PartialEq;
```

```rust
memo(PlayerList::new(game.players().clone()))
```

On update, Reactant compares the new component value with the value stored by
the matching committed `Memo<C>`. When they are equal and no work inside the
boundary is dirty, Reactant reuses the complete committed subtree without
calling `C::render`. The root factory and ancestors above the boundary have
already run to construct the new component value. `memo` therefore does not
prevent arbitrary `G` changes from being mapped into props.

`Memo<C>` is a render value and accepts the ordinary Reactant adapters,
including `.key(value)`.

```rust
self.players.iter().map(|player| {
    memo(PlayerRow::new(player.clone())).key(player.id)
})
```

`PartialEq` is the memo contract. Every component field that can change its
rendered output or handlers must participate in equality. Omitting a
render-relevant field can leave stale UI and callbacks. V1 does not accept a
custom comparison function or expose an imperative subtree-skipping API.

Memoization is only a performance hint. A memoized component renders normally
on mount, after unequal props, and whenever state, reducer, context, resource,
external-store, or geometry work dirties it or a descendant. The exact dirty
propagation and transactional rules are defined in
[Reconciliation, events, and
portals](reconciliation-events-and-portals.md#memoized-component-bailout).

## Pure rendering

Reactant may render a component more than once and discard a render that
suspends or panics. `render` therefore calculates a tree without mutating
external state, starting tasks directly, or sending commands.

```rust
fn render(&self) -> impl Render {
    let (open, set_open) = use_state(false);
    MenuButton::new(open).on_toggle(set_open)
}
```

Hook registration is allowed because it records work in the work-in-progress
component. Event handlers and effects contain actual side effects.

This is incorrect because retries could send the message more than once:

```rust
fn render(&self) -> impl Render {
    self.analytics.record_view();
    Label::new("Inventory")
}
```

Use an effect for behavior caused by the component becoming committed. Clone
owned props before moving them into the `'static` setup closure.

```rust
let analytics = self.analytics.clone();
use_effect(move || analytics.view("inventory"), ());
```

## Render values

The sealed `Render` implementations cover the complete V1 composition surface.

The following values implement `Render`:

- every `Component`;
- `Memo<C>` for a memoized component boundary;
- every Reactant host façade;
- `()` as one intentionally empty logical position;
- `Option<R>`;
- `Result<R, E>` for an explicit
  [recoverable render error](#recoverable-render-errors);
- tuples containing one through twelve render values;
- arrays and `Vec<R>`;
- `Rc<R>`;
- `Fragment<R>`;
- portals, Suspense boundaries, and
  [error boundaries](#error-boundaries); and
- Reactant conditional and resource-read values.

Strings, string slices, characters, numbers, and booleans do not implement
`Render`. Unity UI Toolkit has no context-free text-node host, so applications
render text explicitly through `Label`, `TextElement`, or a component that
selects the intended control.

Arbitrary iterators do not implement `Render`. A blanket iterator
implementation would overlap the blanket component implementation whenever a
downstream type implemented both `Iterator` and `Component`; stable Rust cannot
express the required negative bound. Container `.children(iterator)` consumes
and collects a homogeneous iterator immediately, preserving the intended inline
syntax without an incoherent trait surface.

`()` is the shortest intentionally empty render. `Option` is the normal
conditional output. `None` removes the previously committed output of that
position.

```rust
fn render(&self) -> impl Render {
    self.visible.then(|| Panel::new().child(self.content.clone()))
}
```

A component that normally renders returns its value directly.

```rust
fn render(&self) -> impl Render {
    Label::new(self.text.clone())
}
```

Empty values retain their position in the logical sibling sequence. They
produce no Unity host, but inserting content into an earlier `None` does not
change the position of a later unkeyed sibling. This matches how React counts
empty children. Arrays and vectors still contribute one logical position per
entry, so dynamic collections require keys when entries can reorder.

## Recoverable render errors

An **explicit recoverable render error** is an owned `Err` value that abandons
normal output so an ancestor may render a fallback. `Result<R, E>` implements
`Render` when `R: Render` and `E: std::error::Error + 'static`. `Ok(render)`
contributes the same logical position and output as `render`. `Err(error)`
contributes no partial output and propagates an owned, type-erased error through
render traversal.

This is additive to the `Component` API. A component still returns
`impl Render`, so infallible components and every existing root factory keep
their signatures. A fallible component may return one concrete `Result` render
type and use `?` while constructing it.

```rust
impl Component for Profile {
    fn render(&self) -> impl Render {
        let profile = self.profile.clone().ok_or(ProfileError::Missing)?;
        Ok::<_, ProfileError>(ProfilePanel::new(profile))
    }
}
```

Normal Rust return-type rules still apply: all successful paths must produce
one concrete `R`, using `Either` or `Node` when heterogeneous branches need
erasure. A function returning `impl Render` must identify its concrete error
type on the final `Ok`, as above, because the opaque return type does not give
Rust another place to infer `E`. A helper returning a named `Result<R, E>` may
instead use ordinary unannotated `Ok` expressions. Errors must be owned because
every render value is `'static`.

Reactant erases an `E` only after receiving `Err`. The public borrowed view is:

```rust
pub struct RenderError { /* private fields */ }

impl RenderError {
    pub fn new<E>(error: E) -> Self
    where E: std::error::Error + 'static;
    pub fn from_boxed(error: Box<dyn std::error::Error + 'static>) -> Self;
    pub fn from_boxed_send_sync(
        error: Box<dyn std::error::Error + Send + Sync + 'static>,
    ) -> Self;
    pub fn message(message: impl Into<String>) -> Self;
    pub fn downcast_ref<E>(&self) -> Option<&E>
    where E: std::error::Error + 'static;
}
```

`RenderError` implements `Display`, `Debug`, and `std::error::Error`, preserving
the owned error's display text and source chain. Its `source` delegates to that
error's `source`, avoiding a duplicate wrapper entry in the chain.
`downcast_ref` inspects the concrete error itself and lets an application
fallback distinguish a domain error without requiring one global error enum.

Private storage accepts either a boxed error from the public constructors or a
shared `Arc<dyn std::error::Error + Send + Sync>` from the resource cache.
Display, source inspection, and `downcast_ref::<E>()` delegate to the concrete
error behind either owner. Shared ownership therefore never changes the type an
application observes.

`new` covers concrete standard errors. `from_boxed` and
`from_boxed_send_sync` cover the two common erased forms without relying on
trait-object conversion bounds, and `message` creates an internal standard
error from text. These constructors make `RenderError` itself the `E` when a
source error does not directly satisfy the `Result` implementation's bound.
Neither `E` nor `RenderError` otherwise requires `Send` or `Sync` because render
traversal and fallback selection stay on the engine thread; resource values
retain their independent cross-thread bounds.

Erasure normalizes an existing `RenderError` instead of boxing it inside another
`RenderError`. `RenderError::new(existing)` has the same pass-through behavior.
This preserves the original domain error for `downcast_ref`, display, and source
inspection regardless of whether component code wrapped the error before
returning `Err`.

The fallback receives `&RenderError` only for the duration of rendering and
must copy any information retained in owned props or handlers.

An application-state branch that is clearer as ordinary data may still render
an enum directly. `Err` is intended for a descendant that cannot produce its
normal UI and wants an ancestor to choose the fallback.

## Error boundaries

An **error boundary** is an `ErrorBoundary` render value containing one primary
child and a fallback factory. Its builder shape mirrors `Suspense`: `new`
receives the fallback and `child` supplies the primary subtree.

```rust
ErrorBoundary::new(|error: &RenderError| {
    ErrorPanel::new(error.to_string())
})
.reset_on(self.retry_revision)
.on_error(|game: &mut Game, error| game.report_error(error))
.child(Profile::new())
```

The public shape is:

```rust
pub struct ErrorBoundary<
    F,
    C = Missing,
    D = NoReset,
    O = NoErrorHandler,
> { /* private fields */ }

#[doc(hidden)]
pub struct NoReset;

#[doc(hidden)]
pub struct NoErrorHandler;

impl<F> ErrorBoundary<F, Missing> {
    pub fn new(fallback: F) -> Self;
}

impl<F, C, D, O> ErrorBoundary<F, C, D, O> {
    pub fn reset_on<N: Dependencies>(self, value: N)
        -> ErrorBoundary<F, C, N, O>;
    pub fn on_error<G, N>(self, callback: N)
        -> ErrorBoundary<F, C, D, N>
    where G: 'static, N: Fn(&mut G, &RenderError) + 'static;
}

impl<F, D, O> ErrorBoundary<F, Missing, D, O> {
    pub fn child<R: Render>(self, child: R)
        -> ErrorBoundary<F, R, D, O>;
}
```

The complete boundary implements `Render` when `C: Render`,
`F: Fn(&RenderError) -> R + 'static`, and `R: Render`. The incomplete boundary
does not implement `Render`; `Missing` is the same public incomplete-builder
marker defined under [Required props](#required-props). `child` exists only on
the incomplete specialization, so supplying it twice does not silently replace
the first child. The fallback is `Fn` because Reactant may render it more than
once across resets.
Reactant invokes the fallback without a component hook context. The fallback
must remain pure and calling a hook directly inside it panics; a stateful
fallback returns a component that owns its hooks.

The boundary has one fixed semantic node type independent of `F` and `C`.
Position and an optional key determine its identity. A render replaces its
stored primary value and fallback factory with the current values without
remounting the boundary merely because either generic type changed. The output
created by the selected primary or fallback still follows ordinary nested
component type and key reconciliation. A changed boundary key remounts the
whole boundary and its selected subtree.

An unlatched boundary attempts its primary child. Traversal is depth-first in
logical left-to-right sibling order: tuple field order, array and vector
iteration order, host child order, and a portal's logical source position. The
first explicit render error stops traversal of every later value in that
primary attempt, so later components are not rendered and later resource reads
do not start. The nearest enclosing boundary consumes the error and renders its
fallback. Errors from that fallback are outside the boundary's catching scope
and continue to the next enclosing error boundary. An error that reaches a root
makes the active render-producing runtime entry return `Err(RenderError)`
without committing or poisoning the runtime.

Catching scope follows the rendered tree, not lexical Rust scope. An error
returned before an `ErrorBoundary` value is constructed is above that boundary
and cannot be caught by it. Applications place fallible work in a descendant
component or another child render value when the surrounding boundary should
handle its error.

After a fallback commits, the boundary latches that error and keeps rendering
the fallback across unrelated parent, context, state, store, resource, and
geometry updates. This matches the durable fallback an experienced React user
expects and prevents hot retries of a failing primary.

Changing the comparable value supplied to `reset_on` clears the latch and
retries the primary. Omitting `reset_on` means only a changed boundary key or
unmount/remount resets it. A key change remounts the boundary and fallback;
`reset_on` preserves boundary identity while a successful retry unmounts the
fallback and mounts a fresh primary. If the retry fails, the newly caught error
replaces the latch and the existing fallback subtree reconciles normally.
If reconciliation retains the boundary but the concrete `reset_on` dependency
type changes, Reactant treats the dependency as changed. It preserves boundary
identity, clears the latch, and retries the primary because values with
different Rust types are not comparable.

`on_error` is optional. After a newly caught fallback commits, Reactant queues
the callback once as post-commit work. It receives the runtime's `&mut G`, and
Reactant validates its recorded model type before commit just as it validates
event handlers. It never runs for an abandoned fallback or again for ordinary
renders of the same latch. After it runs, Reactant invokes every root factory
because the callback may have changed arbitrary application state. Its panic
follows the passive-effect poisoning rule. Reactant performs no implicit
logging.

When one commit catches errors in several boundaries, their newly queued
`on_error` callbacks run in catch order: depth-first through the logical tree
and left-to-right among siblings. This order is observable because every
callback may mutate the shared model.

Pending tokens observed only inside an errored primary are discarded and never
registered as committed consumers or boundary waiters. Their resource tasks
remain cached. Completion does not clear the latch. A failed resource also
remains cached, so retry UI normally invalidates it and changes the boundary's
`reset_on` value in the same application action.

When an initially rendered primary fails, its tentative component instances and
hook state are discarded and only a successfully rendered fallback mounts. If
a committed primary later fails and the fallback commits, the primary unmounts,
its hosts are removed, and its passive cleanups are queued under the ordinary
child-before-parent rules. A later successful retry unmounts the fallback and
mounts a fresh primary subtree. Ancestors outside the boundary retain their
identity and state throughout.

Error propagation is a private structural render outcome, not a panic. It uses
the same work-in-progress transaction that makes suspension safe. Rendering a
fallback cannot expose commands or component state until the complete render,
validation, and reconciliation plan commits successfully.

Every render subtree produces an internal outcome containing tentative nodes
and zero or more pending tokens, or one explicit error. Structural children
combine successful outcomes from left to right. Encountering an error discards
the nodes and pending tokens accumulated within that currently attempted
subtree and immediately returns the error. An `ErrorBoundary` replaces an error
from its primary with the independently rendered fallback outcome; therefore,
pending tokens collected by ancestors or earlier siblings outside that primary
remain intact. An error from the fallback propagates normally. `Suspense`
applies the corresponding rules to its own primary and fallback, so each
boundary consumes only outcomes produced inside its primary scope.

Error boundaries and Suspense are orthogonal:

- pending resource tokens pass through error boundaries to the nearest
  `Suspense` boundary;
- explicit errors pass through `Suspense` to the nearest `ErrorBoundary`;
- an explicit error discards pending tokens collected inside the same failed
  primary, while tokens outside that boundary scope remain eligible for their
  own `Suspense` boundary and resource tasks already started remain cached;
- a suspending error fallback may be handled by an enclosing `Suspense`, and an
  error from a Suspense fallback may be handled by an enclosing
  `ErrorBoundary`; and
- panics bypass both boundary kinds.

Error boundaries catch explicit `Err` render values and cached errors from a
fallible resource rendered through `.then`. They never catch component or hook
panics, inconsistent hook order, invalid keys, validation failures,
resource-task panics, event-handler failures, reducer failures, effect failures,
or other Reactant invariants. Converting those developer failures into
recoverable UI would hide a possibly inconsistent runtime state.
Applications copy any error details retained by fallback component props.
`on_error` is the committed reporting path and receives the same borrowed
`RenderError` view used by the fallback.

## Expression-oriented composition

Reactant code should normally be one expression. `.child` appends any render
value; `.children` consumes one homogeneous `IntoIterator`. Neither has a final
build step.

```rust
Column::new()
    .child(Heading::new("Players"))
    .children(self.players.iter().map(PlayerRow::new))
```

Repeated `.child` calls support heterogeneous siblings without an erased value.

```rust
Column::new()
    .child(Header::new(self.title.clone()))
    .child(Body::new(self.content.clone()))
    .child(Footer::new())
```

Container façades retain generic child state so heterogeneous siblings remain
statically typed without an erased public value.

```rust
impl<C> View<C> {
    pub fn child<R: Render + 'static>(self, child: R)
        -> View<(C, R)>;
    pub fn children<I>(self, children: I)
        -> View<(C, Vec<I::Item>)>
    where I: IntoIterator, I::Item: Render + 'static;
}
```

Child-state parameters are public only because they occur in builder return
types and are hidden from generated API documentation. Applications rely on
inference and return `impl Render`. Each specialization retains the complete
valid `View` builder surface. Tuple render values remain useful inside
`Fragment` and component props; `.children((a, b))` is deliberately absent
because Rust cannot overload it alongside arbitrary iterators.

`Fragment::new` groups siblings without introducing a Unity element.

```rust
Fragment::new((
    Label::new("Attack"),
    DamageBadge::new(self.damage),
))
```

Boolean conditions use `then` or `then_some`. Multi-way selection uses an enum
whose variants return one common render wrapper such as `Either`.

```rust
match self.phase {
    Phase::Lobby => Either::left(Lobby::new()),
    Phase::Battle => Either::right(Battle::new()),
}
```

`Either<L, R>` is a public two-variant render value. `left` and `right` are
constructors; changing variants changes the child type at that position and
therefore follows normal remount rules.

```rust
pub enum Either<L, R> {
    Left(L),
    Right(R),
}
```

Three or more heterogeneous branches use the explicit erased `Node` value.

```rust
match self.tab {
    Tab::Cards => Node::new(Cards::new()),
    Tab::Map => Node::new(Map::new()),
    Tab::Log => Node::new(Log::new()),
}
```

## Builders are components

Component constructors return the value that will render. Optional prop methods
consume and return `Self`. There is no `.build()`.

```rust
Button::new("End turn")
    .enabled(self.can_end_turn)
    .class("primary")
```

Required props use either hand-written typestate or the narrow helper described
below. A component with no required props returns `Self` from `new`.

```rust
impl Badge {
    pub fn new() -> Self {
        Self { color: Color::WHITE }
    }
}
```

Setter names match fields unless a domain-specific verb is clearer. A single
child prop uses `child`; a collection uses `children`; textual labels use the
primitive's established `label` or constructor API.

```rust
Dialog::new()
    .title("Discard changes?")
    .child(ConfirmationButtons::new())
```

## Prop reuse with struct update

Reactant adds no prop-spread syntax or macro. Because component fields are
props, an application component reuses a complete value of the same concrete
type with Rust's struct update operator.

```rust
let defaults = PlayerRow::new(player);
PlayerRow {
    selected: true,
    ..defaults
}
```

The operation moves every omitted field exactly as ordinary Rust does. Clone
`defaults` first when the original value must remain available. All fields
participating in an update must be visible at the call site; components with
private fields remain reusable through their consuming builder methods.

Struct update cannot change a value's generic type. Required-prop typestate
therefore uses the setters below, and an incomplete typestate still cannot
render. Prop reuse never bypasses required-property checking.

## Required props

Required props must be present in the Rust type before the value implements
`Component`. Missing props are compile errors, not render-time panics.

Each missing required value is represented by `Missing`; supplying it changes
that generic parameter to the real type. Hand-written typestate may use any
storage layout. The macro-assisted form uses the fixed layout below so stable
`macro_rules!` can reconstruct a type-changing value without inspecting the
component declaration.

```rust
pub struct Missing;
```

```rust
pub struct Card<Title = Missing, Art = Missing> {
    required: (Title, Art),
    optional: CardOptions,
}
```

```rust
struct CardOptions {
    compact: bool,
}

impl Default for CardOptions {
    fn default() -> Self {
        Self { compact: false }
    }
}
```

Only the complete specialization implements `Component`.

```rust
impl Component for Card<String, TextureAddress> {
    fn render(&self) -> impl Render {
        render_card(
            &self.required.0,
            &self.required.1,
            &self.optional,
        )
    }
}
```

Required setters preserve the other typestate parameters.

```rust
impl<A> Card<Missing, A> {
    pub fn title(self, value: String) -> Card<String, A> {
        Card {
            required: (value, self.required.1),
            optional: self.optional,
        }
    }
}
```

Optional setters are generic over all states, so they may appear anywhere in
the chain.

```rust
impl<T, A> Card<T, A> {
    pub fn compact(mut self, value: bool) -> Self {
        self.optional.compact = value;
        self
    }
}
```

The sole permitted component macro removes the repetitive missing-state impls:

```rust
required_props!(Card, title: String, art: TextureAddress);
```

The component still declares its generic fields, constructor, optional setters,
and `Component` implementation. It must have only a `required` tuple and one
`optional` field. Tuple entries and generic parameters appear in the same order
as the macro arguments. The expansion emits the setter impls shown above and
moves `optional` explicitly; it never uses type-changing functional record
update.

```rust
impl Card<Missing, Missing> {
    pub fn new() -> Self {
        Self {
            required: (Missing, Missing),
            optional: CardOptions::default(),
        }
    }
}
```

The macro does not declare the component, inspect optional props, generate
rendering, or generate a general-purpose builder.

The expanded API supports required setters in any order and optional setters at
any point.

```rust
Card::new()
    .compact(true)
    .art(texture)
    .title("Citadel".to_owned())
```

The macro accepts one through four required props and emits only setter impls.
Components that need other storage or more required props use hand-written
typestate so diagnostics and generic types remain readable.

The macro is exported as `reactant::required_props!` and accepts one component
identifier followed by `setter_identifier: RustType` pairs. Each `RustType`
uses the normal `$ty` grammar, including paths and generic arguments. Expansion
uses `$crate` paths and hygienic private names. The component's own visibility
controls the generated methods. Duplicate setters, mismatched tuple arity, or a
component whose generic parameters are not in the declared order fail as normal
Rust compile errors; rustdoc compile-fail cases pin their diagnostics to the
macro invocation and missing setter chain.

`required_props!` is Reactant's only framework-defined macro. The implementation
may use standard Rust, derive, and test macros. Reactant exposes no component,
event, tuple, hook, or general-purpose builder code-generation macro.

## Props that render components

A prop may own another component or any other render value. A generic prop keeps
the concrete type and avoids allocation. Rendering through `&self` clones that
prop into the new owned render tree, so only this form requires `Clone`.

```rust
pub struct Frame<C> {
    child: C,
}

impl<C: Render + Clone + 'static> Component for Frame<C> {
    fn render(&self) -> impl Render {
        Panel::new().child(self.child.clone())
    }
}
```

For large or non-cloneable component descriptions, store `Node`, `Rc<C>`, or
another intentionally shared render value as the prop.

Heterogeneous stored children use the public erased `Node` value. Erasure is
opt-in where storage requires it, not the normal component return type.

```rust
pub struct Toolbar {
    items: Vec<Node>,
}
```

`Node::new(render)` accepts an owned `'static` render value. Cloning `Node`
clones its shared immutable description. A cloned value in a later render
reuses a prior sibling only when normal position or key-and-type identity
matches. Two equal keys in the same current sibling list still panic.

```rust
impl Node {
    pub fn new<R: Render + 'static>(render: R) -> Self;
}
```

`Node` stores `Rc<dyn ErasedRender>` plus the erased render descriptor used by
reconciliation. It does not require `R: Clone`; cloning the node clones the
`Rc`, and rendering calls the immutable erased render operation. Erasure
therefore preserves the concrete component or host type used for nested
identity rather than treating every stored child as the same type.

## Closure props

A component may accept a closure that produces a component. The closure and its
captured data must be owned and `'static`.

```rust
List::new(self.players.clone())
    .row(|player| PlayerRow::new(player))
```

The list invokes the closure only while rendering. Returned components are
ordinary child values and follow normal key rules.

```rust
.row(|player| PlayerRow::new(player.clone()).key(player.id))
```

Callbacks used as event props are different: Reactant stores them on committed
host nodes and invokes them during event dispatch. They are described in
[Event handlers](reconciliation-events-and-portals.md#event-handlers).

## Reactant host façades

Every supported native host has one opaque Reactant façade. A façade becomes a
host node only when Reactant renders it; constructing one does not allocate an
`ObjectId` or send a command.

```rust
View::new()
    .class("health-bar")
    .child(Label::new(self.health.to_string()))
```

Ordinary properties, children, events, motion, keys, refs, and portal targets
are inherent façade methods. Every valid category remains available after every
other call, including when generic child or motion types change.

Façade structs and ordinary property builders retain the corresponding core
host rustdoc, including property semantics and useful Unity links. Examples are
adapted to the Reactant prelude; Reactant-only methods document their own
logical behavior.

```rust
Button::new("Inspect")
    .element_ref(self.button_ref.clone())
    .on_click(inspect_card)
    .key(self.card_id)
    .enabled(self.can_inspect)
```

Repeated singleton methods use the final value. Repeating `.key`,
`.element_ref`, or `.portal_target` replaces the earlier assignment instead of
creating nested adapters. Repeated children and classes continue to append.

Reactant owns every native subscription for a façade it renders. Façades do
not expose the core `events` or `event_subscriptions` fields or builders.
Applications use Reactant's `on_*` methods. Geometry hooks update the separate
host observation registry; geometry is not a native element field.

Lowering privately extracts the corresponding `Ui`-prefixed host, derives its
native subscriptions from the desired handlers, and attaches the final key,
ref, portal target, children, and motion state. It emits one logical and native
host without an adapter-created wrapper.

## Keys

`.key(value)` accepts an owned `Eq + Hash + Clone + 'static` value. Reactant
erases the key as `(TypeId, hash, value)` so equal byte representations from
different domain types do not collide.

```rust
self.cards.iter().map(|card| {
    CardRow::new(card.clone()).key(card.id)
})
```

Keys are compared only among siblings in one rendered child sequence. The same
key may appear under another parent. Duplicate values of the same key type under
one parent panic before commit.

Keys come from application domain identity. Reactant does not generate a key
from component-local state because that would make identity depend on the
instance the key is supposed to identify.

## Manual QA

1. Render a component containing a host façade, tuple, optional child,
   fragment, vector, and iterator. Confirm the fake Unity hierarchy contains
   only the expected host elements and no fragment wrappers.
2. Toggle each conditional form and confirm removed hosts disappear while
   unaffected unkeyed siblings retain their IDs even when an earlier empty
   position gains or loses a host.
3. Render one ordinary component built with setters and another made from it
   with struct update. Confirm the changed prop and all reused props in
   `UiWorld`.
4. Build a required-prop component in both setter orders and with an optional
   setter between them. Confirm both render identically through `UiWorld`.
5. Render component and closure props, reorder keyed results, and confirm their
   state and native IDs follow keys rather than positions.
6. Reset a previously set host property by omitting it on the next render.
   Confirm Unity receives a reset and exposes the platform default.
7. Confirm a raw `UiButton` does not implement Reactant `Render` and that the
   Reactant `Button` API exposes no native subscription builder.
8. Render nested error boundaries around a component that alternates between
   `Err` and `Ok`. Confirm the nearest fallback sees the original concrete
   error and remains latched across unrelated renders. Change `reset_on` and
   confirm a successful retry mounts fresh primary state. Repeat with
   `Err(RenderError::from_boxed(error))` and confirm `downcast_ref` still sees
   the boxed domain error rather than a nested `RenderError`.
9. Combine pending resource reads and explicit errors in both boundary orders.
   Confirm each structural outcome reaches its matching nearest boundary and a
   panic bypasses both.
10. Put a pending read before an error in one primary and another pending read
    outside that primary. Complete both tasks. Confirm only the independently
    committed waiter schedules a retry, while an explicit refresh later sees
    the cached value from the discarded primary.
11. Put a resource read after an error and confirm it first starts only after
    the error is removed. Separately make an error fallback suspend and a
    Suspense fallback return `Err`; confirm the appropriate outer boundary
    handles each outcome.
12. Cause a fallback to panic or fail validation after a primary was committed.
    Confirm the committed primary and command journal remain unchanged. Then
    commit a valid fallback and confirm primary cleanup occurs child before
    parent.
13. Place the boundary below `Memo`, update fallback state while the primary
    still errors, and confirm the dirty descendant defeats memo bailout. Change
    the fallback factory and child generic types through erased branches while
    retaining the boundary key; confirm only changed nested output remounts.
14. Attach `on_error`, abandon one invalid fallback, then commit a valid one.
    Confirm the callback runs once only after the valid fallback commits and
    does not repeat until `reset_on` permits and catches another attempt.
15. Fail a `Resource::try_new` read rendered with `.then`. Confirm it reaches
    the nearest boundary without application mapping and remains latched until
    both the resource and boundary are explicitly reset.
16. Retain one erased boundary while changing the concrete `reset_on` type.
    Confirm the boundary keeps its identity, clears the latch, and mounts a
    fresh primary subtree. Catch errors in two sibling boundaries and confirm
    their `on_error` callbacks run in logical left-to-right catch order.
