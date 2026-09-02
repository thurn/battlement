# Reactant Host Façades

Status: approved target contract, not an implementation-status report.
Downstream Reactant designs describe the post-migration state. The core façade
migration and later feature integrations have separate acceptance boundaries
defined below.

Reactant owns the Rust types used to author its UI hosts. `battlement-ui` owns
the serializable types used by documents, commands, fake clients, and Unity.
This separation gives component authors one coherent builder API without
duplicating the wire model or adding native wrapper elements.

A **host façade** is a Reactant value such as `Button` with private fields. It
contains the corresponding Battlement UI value, such as `UiButton`, plus
Reactant-only state. A façade lowers to exactly one `UiElement` host. "Opaque"
in this document means that applications cannot read or construct its fields;
generic parameters may still appear in inferred builder return types.

The migration is intentionally breaking. Old unprefixed `battlement-ui` host
names disappear, raw `Ui` hosts stop implementing Reactant `Render`, and no
public conversions bridge the two authoring models.

## Related information

- [Battlement UI technical design](../battlement-ui-technical-design.md)
  defines the serializable element, document, command, and event contracts.
- [Reactant technical design](reactant-technical-design.md) defines runtime
  ownership, reconciliation, commits, and failure behavior.
- [Components and rendering](component-authoring.md) defines Reactant render
  values, components, children, and keys.
- [Reconciliation, events, and portals](reconciliation-events-and-portals.md)
  defines host identity, handler replacement, and portal behavior.
- [Refs and geometry](refs-geometry-and-floating-ui.md) defines element refs,
  attachment ownership, and queued host actions.
- [Reactant Animations](animations.md) defines motion stored by these façades.
- [Reactant implementation plan](reactant-implementation-plan.md) records the
  runtime work that predates this migration.
- [Animations implementation plan](animations-implementation-plan.md) treats
  the migration as a completed prerequisite.

## Namespace boundary

Concrete native host structs in `battlement-ui` receive a mechanical `Ui`
prefix. Reactant normally uses the unprefixed name for its façade. The root
container is the deliberate exception: `UiVisualElement` maps to the shorter
Reactant name `View`. Reactant exposes no `VisualElement` façade.

| Battlement UI | Reactant |
|---|---|
| `UiVisualElement` | `View` |
| `UiBox` | `Box` |
| `UiLabel` | `Label` |
| `UiTextElement` | `TextElement` |
| `UiTextField` | `TextField` |
| `UiToggle` | `Toggle` |
| `UiRadioButton` | `RadioButton` |
| `UiRadioButtonGroup` | `RadioButtonGroup` |
| `UiToggleButtonGroup` | `ToggleButtonGroup` |
| `UiDropdownField` | `DropdownField` |
| `UiButton` | `Button` |
| `UiRepeatButton` | `RepeatButton` |
| `UiGroupBox` | `GroupBox` |
| `UiPopupWindow` | `PopupWindow` |
| `UiScrollView` | `ScrollView` |
| `UiScroller` | `Scroller` |
| `UiSlider` | `Slider` |
| `UiSliderInt` | `SliderInt` |
| `UiMinMaxSlider` | `MinMaxSlider` |
| `UiProgressBar` | `ProgressBar` |
| `UiTab` | `Tab` |
| `UiTabView` | `TabView` |
| `UiImage` | `Image` |

The only additional renamed public item is the shared host trait:
`VisualElementProperties` becomes `UiVisualElementProperties`. Supporting
values keep their current names, including `Style`, `Prop`, `Length`, image and
icon sources, control options, and event payloads. The public import paths are
`battlement_ui::UiButton` and, through the umbrella crate,
`battlement::UiButton`.

The rename does not change protocol data. `UiElement::Button` contains a
`UiButton`, and its serialized discriminator remains `"Button"`. Unity class
names, command bodies, snapshots, and events remain byte-for-byte compatible.

Protocol-oriented code uses the prefixed types directly.

```rust
let node = UiNode::new(id, UiButton::new("Save"));
let update = UiButton::default().enabled(false);
```

Reactant component code imports its focused prelude and sees only façades.

```rust
use battlement_reactant::prelude::*;

View::new().child(Button::new("Save").on_click(save_game))
```

The explicit `Box` import from the Reactant prelude shadows Rust's standard
prelude name in component modules. Code that also allocates a Rust box uses
`std::boxed::Box` explicitly.

## Façade contract

Every façade owns all **authored state**, meaning the values supplied by the
current render for one logical host. Its fields are private, and its builders
are the only public mutation surface.

A façade contains the following categories of state:

- The corresponding `Ui` host value and its ordinary authored properties.
- Generic children for native controls that accept logical children.
- Reactant handlers stored with their runtime model type and indexed by event
  kind and propagation phase.
- An optional key stored with its runtime Rust type, element ref, and portal
  target.
- Motion properties, typed variant information, and motion callbacks.

The ordinary property builders mirror the corresponding `Ui` host builders,
except that native `events` and `event_subscriptions` are absent. Reactant
derives native subscriptions from its handler slots during lowering.

Each façade struct and mirrored ordinary builder carries the useful rustdoc
from its `Ui` counterpart. The migration copies type descriptions, property
semantics, examples, and relevant Unity links, then changes only wording that
depends on the authoring layer. Reactant-specific methods keep Reactant-specific
documentation, and native-subscription documentation is not copied because the
façade exposes no such API.

Leaf façades do not expose `child` or `children`. Specialized event builders
appear only on eligible controls. Invalid operations are therefore absent from
the Rust API rather than rejected after rendering.

Raw `Ui` hosts do not implement Reactant `Render`. Reactant exposes neither
`From<UiButton> for Button` nor `From<Button> for UiButton`; private lowering is
the only bridge. This prevents native subscriptions and partially initialized
Reactant state from entering through an escape hatch.

### Host capabilities

One authoritative façade catalog defines which inherent methods each host
receives. It is generated or checked from the same native element catalog used
by `UiElement`; separate handwritten host lists must not drift.

- Every façade supports ordinary common properties, common logical events, a
  key, an element ref, a portal target, and Motion descriptors.
- `View`, `Box`, `ToggleButtonGroup`, `GroupBox`, `PopupWindow`,
  `ScrollView`, `Tab`, and `TabView` support logical children.
- Native child rules remain authoritative. `TabView` accepts `Tab`, and
  `ToggleButtonGroup` accepts `Button`; other constrained families follow the
  Battlement UI catalog.
- Controlled inputs, text controls, scroll views, and tab views expose only
  their applicable specialized handlers, as listed in
  [Event handlers](reconciliation-events-and-portals.md#event-handlers).
- Every façade accepts a Motion descriptor. The animation property catalog
  validates whether a particular `(host kind, property, value shape)` can be
  rendered.

Missing child and handler capabilities are compile-time API absence. Child
relationships that depend on rendered descendant types, cross-host ownership,
and property-specific Motion support require complete-tree validation.

## Order-independent authoring

All builders on one host façade remain available after every other valid host
builder. Authors may arrange calls according to the component's meaning rather
than an adapter stage order.

The same final state may be authored in different orders:

```rust
let first = View::new()
    .child(Label::new("Settings"))
    .style(panel_style())
    .on_click(close_panel);
let second = View::new()
    .on_click(close_panel)
    .style(panel_style())
    .child(Label::new("Settings"));
```

`first` and `second` lower to equal `UiVisualElement` properties, children, and
subscriptions. The same rule applies when motion, keys, refs, or portal targets
appear between ordinary calls.

Generic child or typed-motion state may change the inferred façade
specialization. Every specialization exposes the same valid ordinary,
children, event, motion, key, ref, and portal-target builders.

Nested values keep their own fluent contracts. For example,
`Transition::spring().stiffness(520.0).damping(32.0)` may require a meaningful
sequence inside the `Transition` builder. Applying that transition to a host
does not restrict later host methods.

Builders follow two replacement rules:

- Repeatable collections and property layers such as children, classes, and
  pseudo-styles append or merge in call order according to their own contract.
- Singleton values use the last call, including key, element ref, portal
  target, and each event handler slot.

Changing a key's Rust type remains meaningful because the erased key retains
its `TypeId`. Replacing a key during construction does not create nested keyed
positions; only the final value participates in reconciliation.

```rust
Button::new("Save")
    .key(old_id)
    .on_click(old_handler)
    .key(current_id)
    .on_click(save_game)
```

This host has `current_id` and `save_game`. The earlier singleton values do not
create nested state.

## Lowering and reconciliation

Lowering first converts each façade into one desired host description without
changing committed runtime state. The **desired tree** is the complete host and
component tree produced by the current render.

Façade-local lowering must:

- Convert the private `Ui` value into the matching `UiElement` variant.
- Lower generic children beneath that same host.
- Derive native subscriptions from the desired Reactant handlers.
- Attach the final key, element ref, portal target, and motion descriptor.

Complete-tree validation and reconciliation then:

- Allocate or reuse Reactant host identities.
- Validate native child relationships that depend on lowered descendant types.
- Reject duplicate keys, refs attached to multiple hosts, aliased portal
  targets, handler model mismatches, and incompatible Motion properties.
- Form the complete mutation plan before replacing committed runtime state.

The façade contributes one **logical position**, the sibling identity slot used
by reconciliation, and creates one Unity UI Toolkit element. Children, motion,
keys, refs, portals, and decorations do not insert an extra host merely because
they were added before or after another builder.

Reconciliation compares the lowered `UiElement` properties and Reactant
metadata by their existing rules. Method order is not observable. Two façades
with the same final state produce the same desired tree and mutation plan. A
validation failure preserves the previous committed tree and emits no partial
commit.

## Component and structural adapters

Host façades use inherent methods. Extension adapters remain only where the
receiver is not a host and therefore cannot own host state.

- Every non-host `Render` value retains the general `.key(value)` adapter,
  including components, memoized components, fragments, portals, boundaries,
  conditionals, tuples, arrays, vectors, and `Rc` values.
- Custom components retain motion forwarding because a component may render
  zero, one, or several hosts.
- Portals and boundaries retain their structural child builders.

`MotionComponentExt` collects a complete `MotionProps` value and passes it to
`MotionComponent::with_motion`. The component must apply those props unchanged
to exactly one stable façade. Applying forwarded motion does not restrict later
methods on that façade.

## Migration

The repository migrates atomically. Rust source compatibility is intentionally
broken: there are no aliases, deprecation period, or compatibility feature
flags. Protocol compatibility is exact.

The migration must:

- Rename every concrete `battlement-ui` host and update its documentation,
  doctests, imports, tests, samples, fake clients, and command construction
  sites.
- Copy the rustdoc for every core host and mirrored ordinary builder onto its
  Reactant façade, adapting examples and layer-specific wording without
  weakening the documented Unity semantics.
- Preserve `UiElement` variant names and serialized fixtures.
- Add all Reactant façades before removing direct `Render` implementations
  from raw `Ui` hosts.
- Replace public host extension traits and adapter return types with inherent
  façade methods.
- Update Reactant components without changing their rendered host hierarchy or
  observable event behavior.

Downstream code chooses its layer through imports. Document and command code
imports `UiButton`; component code imports `Button` from the Reactant prelude.

### Prerequisite boundary

The **core façade migration** covers host renames, façade privacy, ordinary
properties, children, events, keys, refs, portal targets, order-independent
authoring, and private one-host lowering. It is the prerequisite consumed by
the animation and asset-generator implementation plans.

The core migration does not require Motion descriptors, typed motion variants,
component motion forwarding, or Motion property validation. The animation
implementation adds those methods directly to the completed façades and must
then pass the animation-integration acceptance below.

## Implementation plan

Implementation should keep the repository compiling as one coherent change:

1. Rename the core host structs and `UiVisualElementProperties` trait while
   pinning existing enum discriminators and serialization fixtures.
2. Introduce opaque façades with copied core rustdoc, ordinary properties,
   generic children, handlers, singleton metadata, and private one-host
   lowering.
3. Move host-only key, ref, portal, and event behavior onto inherent façade
   methods; retain structural adapters where required.
4. Migrate Reactant, samples, tests, and documentation, then remove raw-host
   `Render` support and the obsolete public adapter types.
5. Add future host features such as motion as inherent façade state and methods
   without reintroducing public host stages.

## Automated validation

The core migration suite establishes the prerequisite contract:

- Golden documents, snapshots, create and update commands, events, and every
  renamed core host retain their existing JSON representation.
- Public-API catalog checks prove supporting values such as `Style`, `Prop`,
  `Length`, image and icon sources, control options, and event payloads keep
  their existing names and remain protocol-compatible.
- A raw `UiButton` does not satisfy Reactant `Render`.
- Compile-fail and public-API catalog checks prove old unprefixed
  `battlement-ui` aliases are absent, façade fields are private, and neither
  public conversion direction exists between a façade and its `Ui` host.
- Reactant exports `View` for `UiVisualElement` and does not export a
  `VisualElement` façade.
- Every façade lowers to one host with the expected properties and children.
- Every façade struct and mirrored ordinary builder has the corresponding core
  rustdoc, and every adapted doctest compiles through the Reactant prelude.
- Leaves reject children, and unsupported control events do not compile.
- Cross-category method permutations lower to equal desired trees and commands
  when they preserve the relative order of repeatable children, classes, and
  property layers.
- Compile-pass permutations prove every valid host method remains available
  after `.child`, `.children`, and every other core builder that changes the
  inferred façade specialization.
- Repeated singleton calls retain only the last value.
- Repeated children and classes preserve call order.
- Representative components, memo values, fragments, portals, boundaries,
  conditionals, tuples, collections, and `Rc` values retain `.key(value)`.
- Keys, refs, portals, and handlers do not add logical positions or Unity
  elements.
- Duplicate or cross-runtime refs, duplicate or aliased portal targets, handler
  model mismatches, and illegal lowered children fail transactionally without
  changing the committed tree.

The animation integration suite extends that evidence when motion is added:

- Typed motion variants and custom component forwarding preserve their type
  checks and every façade specialization retains the complete valid host API.
- Motion interleaves with every core host method and does not add a logical
  position or Unity element.
- Invalid Motion combinations fail transactionally without changing the
  committed tree.

## Manual QA

Use the Reactant sample and one direct Battlement UI fixture.

1. Open several Reactant screens and confirm their hierarchy, styles, events,
   focus actions, portals, and geometry behavior are unchanged.
2. Exercise a component whose methods deliberately interleave children,
   properties, events, motion, a key, and an element ref. Confirm it behaves as
   one host and exposes no order-related authoring failure.
3. Replace a key, ref, portal target, and handler twice in one builder chain.
   Confirm only the final singleton value is active.
4. Build a direct `UiDocument` with `UiButton`, serialize it, and confirm its
   element discriminator remains `"Button"` and Unity creates a normal button.
5. Inspect generated Rust documentation and autocomplete. Confirm Reactant
   authors see unprefixed façades, mirrored properties retain their useful
   core explanations and Unity links, and protocol authors see prefixed hosts.

## Static decorative paint

`View::paint(PaintStyle)` paints a solid or gradient background, polygon clip,
and optional shadows without creating Animate or Exit slots:

```rust
use battlement::{MotionColor, MotionLength};
use battlement_reactant::{host::View, paint::{PaintFill, PaintStyle}};

View::new().paint(
  PaintStyle::new()
    .background(PaintFill::Color(MotionColor::new(0.02, 0.04, 0.08, 1.0)))
    .clip_polygon([
      [MotionLength::percent(10.0), MotionLength::percent(0.0)],
      [MotionLength::percent(90.0), MotionLength::percent(0.0)],
      [MotionLength::percent(50.0), MotionLength::percent(100.0)],
    ]),
);
```

Paint coordinates use the host's border box. Padding changes child layout without
moving the painted polygon. Polygon clipping applies to this decorative paint;
it does not clip arbitrary descendant content. Solid fills use the same polygon
as gradients, without an additional rectangular background.

Paint is the underlying presentation for motion and gesture overrides. Updating
only paint preserves host identity, active gestures, and running animation
clocks. Omitted fields in a replacement `PaintStyle` remove the corresponding
static paint; an empty value clears it. Removing a static fill restores the
latest ordinary `Style::background_color`, including updates made while paint
was active. Ordinary style updates likewise replace their own underlying motion
values while preserving focus or hover feedback. Reset restores the native
style keyword so defaults and inherited values remain live.
