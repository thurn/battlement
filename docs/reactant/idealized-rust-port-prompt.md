# Blind idealized Rust port

Use this prompt to establish the application code we would want to write before
judging what Reactant currently permits. The output is an API design sketch,
not an implementation against today's framework.

## Coordinator instructions

Spawn a fresh subagent without inherited conversation history. Send only the
prompt and authoring guide below, the selected component name, an output path,
and snapshots of the relevant TypeScript source and helper contracts. Record
the source revision; use the implementation plan's pinned source unless the
user explicitly requests another revision. Do not send the existing Rust port,
other Reactant project files, the main implementation plan, or earlier audits.

Preserve the subagent's draft before comparing it with the real implementation.
The reviewer then investigates each difference against Reactant and Unity,
including major API changes that could support the idealized code. Keep that
feasibility review separate from the blind generation. Do not feed current API
limitations back into the draft until the independent artifact is recorded.

## Subagent prompt

Read only the supplied TypeScript snapshots and this authoring guide. Write the
ideal Rust port of the selected component. **Do not read or search other
Reactant project files**, including existing Rust components, framework code,
tests, documentation, plans, generated output, or Git history. Do not inspect
other worktrees or earlier conversations. Request missing source context from
the coordinator. Do not compile against the current project or browse for its
APIs; that would bias the experiment toward the existing implementation.

Preserve actual behavior, accepted and ignored props, defaults, controlled
state, component boundaries, label/help interactions, inline hierarchy,
conditional children, and visual styling. Use the fixed conventions below;
freely propose other framework APIs. Strongly question why Reactant could not
support the simpler authoring model, including through major API changes.
Do not hide complexity in newly invented sample-specific helpers. Existing
TypeScript helpers may have corresponding Rust components or hooks.

**Use typed Rust runtime styles. Never encode CSS declarations or style values
in inline strings.** Static paint may use the existing generated-asset
declaration syntax described below. Use the supplied asset and motion entry
points; propose typed APIs for other missing capabilities and list their
contracts. Keep ordinary text and identifiers as strings.

Produce the complete component without ellipses or omitted styling, followed
by the proposed framework contracts and unresolved feasibility questions.
Honor Rust ownership and typing. Do not manufacture reasons the design cannot
work: missing APIs are not language constraints, and unverified feasibility
must remain explicitly unverified. Write only the requested design artifacts;
do not implement framework changes or edit the repository.

## Fixed authoring conventions

Reactant renders through Unity UI Toolkit, not a browser DOM. Proposed controls,
label relationships, and visual effects must have a plausible Unity-backed
implementation; application code does not operate on HTML elements.

### Components and builders

Use `#[builder]` on a props struct and implement `Component` explicitly:

```rust
use std::rc::Rc;

use battlement::Style;
use battlement_reactant::prelude::*;

#[builder]
pub struct Example<R> {
  #[builder(required, into)]
  content: Rc<R>,
  #[builder(required)]
  on_press: Rc<dyn Fn()>,
  height: Option<f32>,
}

impl<R: Render> Component for Example<R> {
  fn render(&self) -> impl Render {
    let on_press = Rc::clone(&self.on_press);
    Button::new("")
      .on_click(move || on_press())
      .style(Style::new().height(self.height))
      .child(self.content.clone())
  }
}
```

- `Example::new()` starts the builder. Set required props in any order; the
  completed value is the component. There is no separate `.build()` call.
- Setters consume and return the builder value. Chain them directly; a
  consuming setter does not require a mutable local binding.
- Fields use `snake_case` and may remain private. A required field uses
  `#[builder(required)]`. Other fields use `Default` unless annotated with
  `#[builder(default = expression)]`, such as `#[builder(default = 1.0)]`.
- `#[builder(required, into)]` on `Rc<R>` accepts a renderable value or an
  existing `Rc<R>`. Preserve generic renderable props instead of converting
  composed labels into strings.
- Ordinary `Option<T>` property setters accept either `T` or `Option<T>`.
  Forward an optional height as `.height(self.height)` without an `if let`.
  String properties accept owned strings and convenient borrowed text.
- Callback props store `Rc<dyn Fn(...)>`, but their generated setters accept
  closures and perform the wrapping. A parent can write
  `.on_press(move || actions::save())`; it need not construct an `Rc` there.
  To forward an existing callback, clone it before capture and use a move
  closure, as above. A Boolean callback similarly uses
  `.on_change(move |checked| on_change(checked))`. Do not capture borrowed
  `self` in a stored callback.
- Optional callback props default to `None`. Their current setters accept a
  closure, and `.clear_on_press()` clears an optional `on_press`. Do not assume
  the ordinary `Option<T>` setter rule also accepts an optional callback.
  If direct optional forwarding needs a better API, propose it explicitly
  while retaining ordinary closure syntax.
- Implement `Component::render(&self) -> impl Render`; do not invent dispatch
  from an inherent `render` method or a different builder macro.

### Child composition

Elements use fluent constructors and properties. `.child(...)` accepts a
renderable child, including tuples, `Rc<R>`, and `Option<R>`. Tuples and options
do not create layout hosts. Optional children can use
`.child(condition.then(|| SomeComponent::new()))`. Write nested element trees
inline where they are used. A component field named `children` produces a
`.children(...)` setter; its name follows the declared prop.

Keep hook calls unconditional and stable across renders. A normal Rust `if`
expression must have compatible branch types; use a typed sum such as
`Either<L, R>` or conditional children when the branches render different types.
Do not invent additional macros or erase meaningful component boundaries to
avoid normal Rust typing. The established `#[builder]` and
`asset_generator::generate!` forms in this guide are available.

### Typed styles

Use fluent `Style` builders, numeric values, enums, and typed values:

```rust
Style::new()
  .position(Position::Relative)
  .align_items(Align::Center)
  .width(77)
  .height(optional_height)
  .border_width(4)
  .border_color(Color::rgb(0.3, 0.6, 1.0))
  .translate(Translate::two_dimensional(
    Length::Px(0.0),
    Length::Px(offset_y),
  ))
```

Do not write `.position("relative")`, `.border("4px solid #4ba3ff")`,
`.transform(format!("scale({scale})"))`, CSS blobs, or a string-based CSS parser
as an escape hatch. Runtime gradients, shadows, filters, transforms, easing,
durations, and transitions must use typed values too. Static paint can instead
use generated assets as described below. Propose readable typed APIs where the
guide does not specify them; their current availability is unknown.

Translate CSS custom-property references and fallbacks into typed inherited
values, context, or composable motion inputs. Preserve their dynamic behavior
and operation order instead of copying `var(...)` strings or freezing them to
constants. Display text, semantic identifiers, asset identifiers, and names may
remain strings; styling syntax may not.

### Generated PNG assets

Reactant's asset generator replaces static paint that ordinary UI Toolkit
styles cannot express, such as complex gradients, clipping, masks, shadows,
filters, and advanced text treatments. Declare the paint at module scope with
the existing `battlement_reactant::asset_generator` API:

```rust
use battlement_reactant::asset_generator;

asset_generator::generate! {
  @background PANEL_PAINT {
    @canvas 160px 80px;
    background: linear-gradient(180deg, #06142b, #02091a);
  }
}
```

The macro's supported CSS-like declaration grammar is allowed. It is a static
asset description, not a Rust string containing runtime styles. It emits a
typed handle: `PANEL_PAINT.image()` supplies image content, and
`PANEL_PAINT.background_style()` supplies native background properties that can
be chained with ordinary typed layout styles. Neither assigns layout dimensions
automatically. `@nine-slice` describes resizable paint with fixed edges, and
`@text-image` describes static text with advanced paint.

Before a build or run, the `cargo battlement reactant assets generate` command
selects a project with `--project` and renders its declarations to PNGs for
Unity to import as textures.
The blind subagent declares any necessary assets but does not run generation.
Each runtime variant needs a separate named static declaration; the generator
does not accept runtime parameters. Select the appropriate typed handle from
state, and use motion for continuous changes. Preserve source timing and
transitions when switching visual states.

Rasterize decorative paint, not the whole interactive component. Keep layout,
hit testing, accessibility, focus, callbacks, and live state in Reactant. Do not
freeze dynamic text or animation into an image, or generate assets for simple
paint already expressible through ordinary typed styles.

### Motion entry point

Reactant hosts expose Motion builders directly. Use `MotionStyle` for typed
targets and `Transition` for timing; an animation does not require another
layout element or a `Motion::new(...)` wrapper:

```rust
View::new()
  .initial(MotionStyle::new().opacity(0.0))
  .animate(MotionStyle::new().opacity(1.0).scale(1.0))
  .while_hover(MotionStyle::new().scale(1.045))
  .while_tap(MotionStyle::new().scale(0.88))
  .transition(Transition::tween().duration_secs(0.09))
```

These types are available through the Reactant prelude. Targets can be computed
from props or state. `motion_config::use_reduced_motion()` provides the user's
preference when the source branches on it. The numbers above only illustrate
syntax; preserve the source's actual timing, easing, state precedence, and
reduced-motion behavior. Static generated textures can participate in motion
without regenerating PNGs every frame. Propose any additional typed motion
composition needed by the source instead of using CSS animation strings or
assuming that existing API gaps are unavoidable.
