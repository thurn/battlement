# Reactant Components and Rendering

This appendix defines how application code describes a Reactant tree. It is part
of the [Battlement Reactant technical design](reactant-technical-design.md) and
assumes the host primitives from the
[Battlement UI technical design](../battlement-ui-technical-design.md).

## Related information

- [React: describing the UI](https://react.dev/learn/describing-the-ui) explains
  component composition and purity in React.
- [React: rendering lists](https://react.dev/learn/rendering-lists) explains the
  purpose of keys and stable list identity.
- [Hooks and effects](hooks-and-effects.md) defines state read during rendering.
- [Reconciliation, events, and portals](reconciliation-events-and-portals.md)
  defines how rendered values retain identity and become Unity commands.

## Component structs

`Render` is a sealed trait for values Reactant can lower into an internal node
sequence. Its lowering method is private, so applications compose supported
values rather than implementing host protocol behavior.

```rust
pub trait Render: private::Sealed + 'static {}
```

A component is an owned struct implementing `Component`. Its fields are props.
Reactant does not support function components.

```rust
pub trait Component: 'static {
    fn render(&self) -> Option<impl Render>;
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
    fn render(&self) -> Option<impl Render> {
        Some(Label::new(self.name.clone()))
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

## Pure rendering

Reactant may render a component more than once and discard a render that
suspends or panics. `render` therefore calculates a tree without mutating
external state, starting tasks directly, or sending commands.

```rust
fn render(&self) -> Option<impl Render> {
    let (open, set_open) = use_state(false);
    Some(MenuButton::new(open).on_toggle(set_open))
}
```

Hook registration is allowed because it records work in the work-in-progress
component. Event handlers and effects contain actual side effects.

This is incorrect because retries could send the message more than once:

```rust
fn render(&self) -> Option<impl Render> {
    self.analytics.record_view();
    Some(Label::new("Inventory"))
}
```

Use an effect for behavior caused by the component becoming committed. Clone
owned props before moving them into the `'static` setup closure.

```rust
let analytics = self.analytics.clone();
use_effect((), move || analytics.view("inventory"));
```

## Render values

The sealed `Render` implementations cover the complete V1 composition surface.

The following values implement `Render`:

- every `Component`;
- every `UiElement` variant exported by `battlement-ui`;
- `Option<R>`;
- tuples containing one through twelve render values;
- arrays and `Vec<R>`;
- `Rc<R>`;
- `Fragment<R>`;
- portals and Suspense boundaries; and
- Reactant conditional and resource-read values.

Arbitrary iterators do not implement `Render`. A blanket iterator
implementation would overlap the blanket component implementation whenever a
downstream type implemented both `Iterator` and `Component`; stable Rust cannot
express the required negative bound. Container `.children(iterator)` consumes
and collects a homogeneous iterator immediately, preserving the intended inline
syntax without an incoherent trait surface.

`Option` is the normal conditional output. `None` removes the previously
committed output of that position.

```rust
fn render(&self) -> Option<impl Render> {
    self.visible.then(|| Panel::new().child(self.content.clone()))
}
```

A component that normally renders should return `Some` directly.

```rust
fn render(&self) -> Option<impl Render> {
    Some(Label::new(&self.text))
}
```

`Result` is not a render value. Battlement developer and resource failures
panic; recoverable application states must be represented explicitly as props
or an enum and rendered normally.

## Expression-oriented composition

Reactant code should normally be one expression. Containers accept one child,
many children, and iterator-produced children without a final build step.

```rust
Column::new()
    .child(Heading::new("Players"))
    .children(self.players.iter().map(PlayerRow::new))
```

Tuples support heterogeneous siblings.

```rust
Column::new().children((
    Header::new(&self.title),
    Body::new(self.content.clone()),
    Footer::new(),
))
```

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
#[derive(Default)]
struct CardOptions {
    compact: bool,
}
```

Only the complete specialization implements `Component`.

```rust
impl Component for Card<String, TextureAddress> {
    fn render(&self) -> Option<impl Render> {
        Some(render_card(
            &self.required.0,
            &self.required.1,
            &self.optional,
        ))
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

## Props that render components

A prop may own another component or any other render value. A generic prop keeps
the concrete type and avoids allocation. Rendering through `&self` clones that
prop into the new owned render tree, so only this form requires `Clone`.

```rust
pub struct Frame<C> {
    child: C,
}

impl<C: Render + Clone + 'static> Component for Frame<C> {
    fn render(&self) -> Option<impl Render> {
        Some(Panel::new().child(self.child.clone()))
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

## Direct Battlement primitives

Every supported `UiElement` variant has a direct `Render` implementation. A
primitive becomes a host node only when Reactant renders it; constructing one
does not allocate an `ObjectId` or send a command.

```rust
VisualElement::new()
    .class("health-bar")
    .child(Label::new(self.health.to_string()))
```

Reactant extension methods return small generic adapters around the primitive.
They do not add another Unity element.

```rust
Button::new("Inspect")
    .key(self.card_id)
    .element_ref(self.button_ref.clone())
```

The adapter chain is flattened while producing the virtual tree. Conflicting
extensions, such as two different keys on one value, panic during rendering.

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

`use_id` is not a list key. It identifies accessibility and cross-element
relationships inside one mounted component; using it as a key makes identity
depend on the component instance that the key is supposed to identify.

## Manual QA

1. Render a component containing a primitive, tuple, optional child, fragment,
   vector, and iterator. Confirm the fake Unity hierarchy contains only the
   expected host elements and no fragment wrappers.
2. Toggle each conditional form and confirm removed hosts disappear while
   unaffected siblings retain their IDs.
3. Render one ordinary component built with setters and another made from it
   with struct update. Confirm the changed prop and all reused props in
   `UiWorld`.
4. Build a required-prop component in both setter orders and with an optional
   setter between them. Confirm both render identically through `UiWorld`.
5. Render component and closure props, reorder keyed results, and confirm their
   state and native IDs follow keys rather than positions.
6. Reset a previously set primitive property by omitting it on the next render.
   Confirm Unity receives a reset and exposes the platform default.
