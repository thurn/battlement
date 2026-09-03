## Construction

`#[builder]` adds a zero-argument constructor and consuming setters to the
annotated struct. Its ordinary type names the completed value: no separate
builder or `.build()` is needed. Reactant exports the attribute in its prelude.
Outside Reactant, point the attribute at this crate's support module:

```rust
use battlement_builder::{builder, support};

#[builder(support = support)]
pub struct Size {
    /// Requested width in pixels.
    #[builder(required)]
    width: f32,
    /// Requested height in pixels.
    #[builder(required)]
    height: f32,
    #[builder(default = 1.0)]
    scale: f32,
    description: Option<String>,
}

let size: Size = Size::new()
    .description("Preview")
    .height(600.0)
    .scale(2.0)
    .width(800.0);
```

Required setters accept any order and cannot repeat. Optional setters work
before, between, and after required setters; repeated values replace earlier
ones. Field rustdoc is also the setter's rustdoc.

Unmarked fields use `Default`. Non-defaultable fields must be required or have a
`#[builder(default = expression)]`. Defaults run once, in declaration order,
when `new()` runs. Required fields cannot also have defaults. The attribute does
not implement `Default` for the struct or perform validation.

## Conversions

- `String` accepts `impl Into<String>`.
- `Option<T>` accepts `T`, `Option<T>`, and `None` through the same setter.
- `Option<String>` additionally accepts `&str` and `&String`, but not
  `Option<&str>`. Nested options may need type annotations.
- `#[builder(into)]` opts other fields into `Into<DeclaredType>`. For options,
  this converts to the entire option, not its payload.
- Ordinary fields otherwise accept exactly their declared type.

Options may be combined (`#[builder(required, into)]`) or written as separate
attributes. Duplicate and unknown options are errors. Type conveniences
recognize bare and canonical standard-library paths, not arbitrary aliases.

## Callbacks

`Rc<dyn Fn(...) -> R>` accepts a closure with that signature, preserving closure
parameter inference. Optional callbacks accept closures and have a separate
`clear_<field>()` method; unlike ordinary options, their setters do not accept
`Some` or `None`.

```rust
use std::rc::Rc;
use battlement_builder::{builder, support};

#[builder(support = support)]
struct Control {
    #[builder(required)]
    changed: Rc<dyn Fn(bool)>,
    focused: Option<Rc<dyn Fn()>>,
}

let control = Control::new()
    .focused(|| println!("Focused"))
    .clear_focused()
    .changed(|checked| println!("{}", !checked));
```

The macro creates the `Rc` and optional `Some` wrapper. It preserves declared
callback lifetimes, return values, higher-ranked arguments, and trait bounds.
Forward an existing `Rc` callback using a closure adapter such as
`move |value| callback(value)`. `FnMut`, `FnOnce`, `Box`, and `Arc` are ordinary
exact-type fields, not automatically wrapped callbacks. Callback fields cannot
also specify `into`.

Reactant additionally recognizes `EventCallback<A>` and
`battlement_reactant::callback::Callback<A>`. These setters use Reactant's
existing event conversion, accepting ordinary or model-aware closures. Stored
event callbacks can be forwarded directly through another setter or to a host
event. Optional event callbacks also have clearing methods. Model-aware closure
arguments may require an explicit model type.

## Generic components and supported declarations

Original type, lifetime, const parameters, and bounds remain part of the type.
Each required prop adds one defaulted generic slot. Generic optional fields
place their `Default` bounds on the constructor, not the setters. Generic
`Rc<Child>` props can use `required, into`; when forwarding an existing `Rc`,
type annotations can distinguish sharing it from wrapping it in another `Rc`.

Place `#[builder]` before derives. Named-field, empty, and unit structs are
supported. Item-level conditional compilation applies to the complete
expansion. Tuple structs, enums, unions, conditionally compiled fields, and
self-dependent or recursive declarations are unsupported. Fully qualify a
required associated type as `<T as Trait>::Associated` when `T` has multiple
trait bounds; an unambiguous single bound can be qualified automatically.

Required setters move fields, so an outer custom `Drop` implementation is
unsupported; fields may themselves have destructors. Private phantom fields
retain otherwise-unused original generic parameters. Ordinary derives see the
generated representation; direct literals, exhaustive destructuring, layout,
and serialized representation are not stable construction interfaces.

`#[builder(support = ::module)]` selects a support module explicitly, including
when a dependency is renamed. It must export `Missing` and `IntoOption`, plus
`IntoEventCallback` when event props are present. Reactant's default is
`::battlement_reactant::prelude::__builder_support`.
