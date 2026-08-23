# Battlement UI technical design

Status: proposed implementation contract

This document is normative for the first implementation of `battlement-ui`,
`battlement-ui-fake`, and the mandatory Battlement UI runtime written in
**C# (C Sharp, Unity's managed programming language)**. Rust is
the source of truth for the protocol names, defaults, validation, hierarchy,
and committed application state described here. The Unity implementation must
mirror that contract exactly.

## Examples first

An **ObjectId** is the **UUID (universally unique identifier)** that Rust assigns
to any Battlement-controlled **GameObject (Unity's scene object)** or visual
element. **UI Toolkit** is Unity's hierarchy-based user-interface system. A
**visual tree** is UI Toolkit's ordered parent-and-child hierarchy. A **builder** is a Rust value
whose field-named methods set optional state and return the updated value.
**JSON (JavaScript Object Notation)** is the existing human-readable Battlement
**wire format**—the serialized message shape exchanged with Unity. An
**Addressables address** is the project-defined string that
Unity's Addressable Asset System uses to locate a **prepared asset**, an asset
loaded and type-checked before commands may refer to it.

### Build a **flex layout** with text, an image, and a scroll view

Flex layout is UI Toolkit's row-and-column sizing and placement model. The
first example creates one complete subtree with a single command. Every
omitted builder field uses the Battlement default and therefore does not appear
in JSON.

```rust
use crate::assets::mygame::ui;
use battlement::{Command, ObjectId};
use battlement_ui::prelude::LengthUnits;
use battlement_ui::{
    Align, Box, Button, Color, FlexDirection, Image, Label, ScrollView, Style,
    UiEventKind,
};

fn create_inventory(root_id: ObjectId, play_id: ObjectId) -> Command {
    Command::create_visual_element(
        root_id,
        Box::new()
            .style(
                Style::new()
                    .width(100.pct())
                    .height(100.pct())
                    .padding(24)
                    .row_gap(12)
                    .background_color(Color::rgb8(20, 24, 32)),
            )
            .child(
                Image::new()
                    .source(ui::HERO_PORTRAIT)
                    .style(Style::new().width(96).height(96)),
            )
            .child(
                ScrollView::new()
                    .style(Style::new().flex_grow(1))
                    .child(Label::new("Iron sword"))
                    .child(Label::new("Travel cloak")),
            )
            .child(
                Button::with_id(play_id, "Continue")
                    .events([
                        UiEventKind::PointerEnter,
                        UiEventKind::PointerLeave,
                        UiEventKind::Click,
                    ])
                    .style(
                        Style::new()
                            .align_self(Align::FlexEnd)
                            .flex_direction(FlexDirection::Row),
                    ),
            ),
    )
}
```

`Command::create_visual_element` appends the outer `Box` to `root_id`. The
children are serialized recursively in their displayed order. Unity constructs
the subtree while detached. A **usage lease** retains a prepared Addressable
asset while a live UI property refers to it. Unity applies every property and
acquires every required usage lease before it attaches the completed subtree.
The `ui::HERO_PORTRAIT` texture constant is generated from the
`mygame/ui/hero-portrait` Addressables key by `cargo battlement generate`; the
example imports its generated module so the call uses exactly one qualifier.

### Change hover and click behavior with a **synchronous call** to Rust

A **subscription** is an element's opt-in request for one event kind. Unity
sends no action for an event that has no applicable subscription. A subscribed
event calls Rust synchronously through the existing Battlement transport; the
synchronous call blocks Unity's event callback until Rust returns.

```rust
use battlement::{Action, ActionBody, Command, ObjectId};
use battlement_ui::{ButtonUpdate, Color, Style, UiEventBody};

fn handle_ui_action(action: &Action, play_id: ObjectId) -> Vec<Command> {
    let ActionBody::VisualElement(event) = &action.body else {
        return Vec::new();
    };
    if event.target_id != play_id {
        return Vec::new();
    }

    let color = match event.body {
        UiEventBody::PointerEnter(_) => Color::rgb8(66, 133, 244),
        UiEventBody::PointerLeave(_) => Color::rgb8(42, 48, 60),
        UiEventBody::Click(_) => Color::rgb8(52, 168, 83),
        _ => return Vec::new(),
    };

    vec![Command::update_visual_element(
        ButtonUpdate::new(play_id)
            .style(Style::new().background_color(color)),
    )]
}
```

Unity decodes the returned commands before the callback returns. It applies the
mutations after the current UI Toolkit event finishes but before the next
layout and repaint. The hover color therefore appears on the first rendered
frame after the pointer enters without mutating the hierarchy while UI Toolkit
is still traversing it.

### Commit a text field without sending every keystroke

A **controlled value** is application state that Rust commits. The Unity
control may hold a temporary local draft, but Rust remains authoritative.

```rust
use battlement::{ActionBody, Command, ObjectId};
use battlement_ui::{TextField, TextFieldUpdate, UiEventBody, UiEventKind, UiValue};

fn name_field(id: ObjectId) -> TextField {
    TextField::with_id(id)
        .label("Character name")
        .value("Ada")
        .events([UiEventKind::ValueCommitted])
}

fn accept_name(action: &ActionBody, id: ObjectId) -> Option<Command> {
    let ActionBody::VisualElement(event) = action else {
        return None;
    };
    let UiEventBody::ValueCommitted(UiValue::String(proposed)) = &event.body else {
        return None;
    };
    (event.target_id == id).then(|| {
        Command::update_visual_element(
            TextFieldUpdate::new(id).value(proposed.trim()),
        )
    })
}
```

Typing changes only the local draft. A **committed value** is the last control
value Rust accepted. Enter or focus loss proposes one `ValueCommitted` event.
Unity restores the last committed value with
`SetValueWithoutNotify`, then applies the value Rust returned. Subscribing to
`UiEventKind::Input` explicitly enables per-keystroke proposals.

### Test UI rules without Unity

`battlement-ui-fake` supplies a **fake client**, an in-memory substitute for
Unity used by Rust tests. It models the authoritative tree, validation, event
routing, and controlled commits. It deliberately does not calculate pixels or
render.

```rust
use battlement::{Command, ObjectId};
use battlement_fake::{FakeAssetCatalog, FakeClient};
use battlement_native::Engine;
use battlement_ui::Color;

fn assert_hover<E>(engine: E, assets: FakeAssetCatalog, play_id: ObjectId)
where
    E: Engine<Command = Command>,
{
    let mut client = FakeClient::connect(engine, assets);
    client.ui().pointer_enter(play_id);

    assert_eq!(
        client.ui().element(play_id).style().background_color(),
        Some(Color::rgb8(66, 133, 244)),
    );

    client.ui().click(play_id);
    assert!(client.ui().journal().iter().any(|command| {
        command.targets_visual_element(play_id)
    }));
}
```

The fake targets elements by `ObjectId`; it does not simulate hit testing,
font measurement, resolved layout, rendering, or frames.

## Related information

- [Battlement technical design](technical-design.md) defines sessions,
  snapshots, commands, batches, input, errors, and the thin-client boundary.
- [Battlement fake client design](fake-client-design.md) defines the existing
  fake's engine-driving and failure conventions.
- [JSON protocol migration design](json-protocol-migration-design.md) defines
  the tagged-union JSON encoding mirrored by C#.
- [Address code generation](address-code-generation.md) defines generated
  Addressables address types.
- [Controlled and uncontrolled components](https://legacy.reactjs.org/docs/uncontrolled-components.html)
  provides the established controlled-value distinction used by the input
  contract.
- The audited Unity 6000.5.8f1 UI Toolkit reference is available locally at
  [`/Users/dthurn/Downloads/UIElements`](/Users/dthurn/Downloads/UIElements/).

## Summary

`battlement-ui` supplies strongly typed Rust builders and wire values for a
broad programmatic subset of Unity UI Toolkit. It covers 23 runtime element
classes, all 86 stable web-like writable style properties selected below,
**screen-space**, **target-texture**, and **world-space** documents, Addressable UI assets,
typed events, and deterministic fake-client behavior.

**Screen-space** UI renders in display coordinates. **Target-texture** UI
renders into a **RenderTexture (a Unity texture that can receive rendering)**
asset instead of directly to the display.
**World-space** UI is placed and transformed in the Unity scene.

Rust owns the declared hierarchy, committed control values, application visual
state, and responses to user input. Unity owns UI Toolkit objects, rendering,
layout calculation, focus mechanics, drafts during text entry, pointer motion
during drags, scroll inertia, and other transient platform behavior. Unity does
not infer game rules or apply application-specific hover, active, selected, or
validation styles on its own.

The UI implementation is mandatory. It is not a custom command, registered
extension, Cargo feature, optional dependency, or separate optional Unity
package.

## Terminology and ownership

A **UIDocument** is Unity's component that connects one visual tree to a panel.
A **document root** is the `rootVisualElement` owned by one Unity `UIDocument`.
It receives its own `ObjectId`, but it is not a GameObject. A **logical child**
is a child added through `VisualElement.Add` or `Insert`; UI Toolkit may route
that child into a control-specific `contentContainer` whose physical hierarchy
also contains Unity-created implementation elements.

An **inline style** is a value written through `VisualElement.style`. A missing
inline value allows the panel theme and UI Toolkit defaults to resolve the
property. A **style clear** assigns `StyleKeyword.Null`, removing the inline
override. It is different from explicit `Auto`, `Initial`, or `None` values.

A prepared asset is an Addressables declaration loaded before a command may
refer to it. **Addressables** is Unity's runtime asset-loading and lifetime
system. A usage lease also prevents command-driven removal from the prepared
set.

A committed value is the last control value accepted by Rust. A **local
draft** is a temporary value owned by a native control while the user types,
drags, or scrolls. A **proposal event** reports a prospective value to Rust; it
does not itself change committed application state.

A **trickle phase** visits subscribed ancestors from the document root toward
the target. The **target phase** visits the target. A **bubble phase** visits
subscribed ancestors from the target back toward the document root. These are
UI Toolkit's existing event phases, not Battlement-specific concepts.

## Crate architecture

The workspace adds three mandatory crates:

| Crate | Responsibility | Dependencies |
|---|---|---|
| `battlement-types` | IDs, asset-address newtypes, colors, vectors, rectangles, shared scalar values, and shared physical-input values | `serde`, `uuid` |
| `battlement-ui` | UI documents, recursive elements, builders, styles, patches, events, routing, and validation | `battlement-types`, `serde` |
| `battlement-ui-fake` | In-memory execution of UI snapshots and UI command payloads, plus synthetic UI gestures | `battlement-types`, `battlement-ui` |

`battlement` depends unconditionally on `battlement-types` and `battlement-ui`.
A **reexport** makes a dependency's public Rust name available through the
depending crate. `battlement` reexports the existing names moved to
`battlement-types`, so game code may
continue importing `ObjectId`, colors, vectors, and existing asset addresses
from `battlement`. `battlement-ui` does not depend on `battlement`.
`battlement-fake` depends on and reexports
`battlement-ui-fake`. It matches the four `CommandBody` wrappers and delegates
their `battlement-ui` payloads to `UiWorld`; `battlement-ui-fake` never imports
the outer `Command` or `CommandBody` types.

The exact types moved to `battlement-types` are the generic typed-ID machinery;
`SessionId`, `ActionId`, `BatchId`, `CommandId`, `ObjectId`, and `SceneId`; the
generic asset-address machinery; existing `SceneAddress`, `PrefabAddress`,
`ParticleEffectAddress`, `MaterialAddress`, `TextureAddress`,
`AudioClipAddress`, and `FontAddress`; the new UI address types named below;
`Color`, `RgbColor`, `Vector2`, `Vector3`, `Quaternion`, `Rect`,
`ScreenPosition`, and `ScreenSize`; and the shared physical-input vocabulary
`PhysicalKey`, `KeyModifier`, `KeyModifiers`, `PointerButton`, and
`PointerType`. Commands, messages, snapshots, GameObjects, prepared-asset
declarations, domain-specific event enums, and validation stay in `battlement`
or `battlement-ui`.

World input and UI input are two delivery domains over this one vocabulary,
not two definitions of the devices. World delivery uses global key selection,
physics targeting, bottom-left screen coordinates, and world hits. UI delivery
uses focus, visual-tree routing, upper-left panel coordinates, and control
semantics. Those target, coordinate, and routing models remain distinct; the
physical key, modifier, button, and pointer-type names do not.

`PhysicalKey` replaces the existing core `KeyCode` name without an alias. It is
the closed W3C hardware-code set already supported by core keyboard input and
used by UI key events. `KeyModifiers` is a value type containing a unique set
of `KeyModifier` values in canonical enum order. It serializes as a JSON array,
rejects duplicates and noncanonical order, and is omitted from event payloads
when empty. The exact modifier cases are defined with the event wire types
below.

## Rust public API

### Commands and closed unions

The existing `Command` remains a struct with `command_id`, `blocking`, and
`body`. The following convenience constructors create blocking commands and
generate a fresh `CommandId` by default:

```rust
impl Command {
    pub fn create_visual_element(
        parent_id: ObjectId,
        element: impl Into<VisualElement>,
    ) -> Self;

    pub fn update_visual_element(
        patch: impl Into<VisualElementPatch>,
    ) -> Self;

    pub fn destroy_visual_element(object_id: ObjectId) -> Self;

    pub fn perform_visual_element_action(
        object_id: ObjectId,
        action: VisualElementAction,
    ) -> Self;
}
```

Each has a corresponding `_with_id` form whose first argument is an explicit
`CommandId`, for deterministic fixtures, replay tooling, and callers that must
correlate an ID before the command is built. Normal game code does not allocate
command IDs itself. This convenience affects construction only: `command_id`
remains required in the command struct and on the wire.

They wrap these four `CommandBody` cases and JSON tags:

| Rust case | JSON tag | Payload |
|---|---|---|
| `VisualElementCreate` | `VisualElementCreate` | `parent_id`, omitted append-default `child_index`, recursive `element` |
| `VisualElementUpdate` | `VisualElementUpdate` | aggregate `patch` |
| `VisualElementDestroy` | `VisualElementDestroy` | `object_id` |
| `VisualElementPerformAction` | `VisualElementPerformAction` | `object_id`, typed `action` |

`child_index` is a zero-based `u32`. Missing means append. An index greater than
the current logical child count is invalid. Destroying an element recursively
destroys its Rust-owned descendants. Destroying a document root through this
command is invalid; destroy its owning GameObject or replace the snapshot.

`ActionBody` gains exactly one case:

```rust
pub enum ActionBody {
    // Existing cases remain.
    VisualElement(UiEvent),
}
```

All detailed element, patch, action, and event unions live in `battlement-ui`.
A **closed enum** lists every permitted case and has no arbitrary extension
case. These unions are closed enums, and the C# JSON converter lists every case
explicitly.

### Builder and element representation

Each selected Unity class has a distinct public create builder and update
builder. `Button` converts into the internal recursive `VisualElement::Button`
case; `ButtonUpdate` converts into the matching `VisualElementPatch::Button`
case. Application code does not construct those internal cases directly.

Every create builder generates a fresh `ObjectId` in `new`. A parallel
`with_id` constructor accepts an explicit ID when application code needs to
retain a handle, refer to the element from events, or construct deterministic
fixtures. A `new` constructor accepts at most one semantic argument, except for
ordinary two-scalar value types such as `Size::new(width, height)`. Additional
optional state always uses field-named consuming methods. When a type has
multiple required values with no safe default, those methods use typestate,
similar to `bon`: only the state in which every required field is present can
convert into the protocol type. This keeps constructor call sites readable and
makes incomplete values unrepresentable without a multi-argument constructor.
An ergonomic argument such as Button text or a choice list may still equal its
wire default and is then omitted. Every builder exposes its generated ID
through `object_id(&self)`, so a caller may retain it before moving the builder
into a tree. All builders implement `Clone`, `Debug`, `PartialEq`,
`Serialize`, and `Deserialize` where the public representation crosses the
wire. Builder backing fields remain private; public getters expose values that
callers need to inspect.

Builder implementation should remain compile-time-light. Do not introduce a
procedural macro, `syn`, or `quote` where ordinary Rust or a declarative macro
is sufficient. In particular, the shared methods implemented by every visual
element builder should come from a small `macro_rules!` macro rather than a
custom derive or attribute macro.

The ergonomic `new` constructors are below. Every builder also has
`with_id(object_id, ...)` with the same remaining arguments.

| Builders | `new(...)` signature |
|---|---|
| `VisualElement`, `Box`, `Image`, `GroupBox`, `PopupWindow`, `ScrollView`, `Scroller`, `Tab`, `TabView`, `TextField`, `Toggle`, `RadioButton`, `ToggleButtonGroup`, `Slider`, `SliderInt`, `MinMaxSlider`, `ProgressBar` | `new()` |
| `TextElement`, `Label`, `Button` | `new(text: impl Into<String>)` |
| `RepeatButton` | `new(text: impl Into<String>)`; typestate requires `delay_ms(...)` and `interval_ms(...)` before conversion |
| `RadioButtonGroup`, `DropdownField` | `new<I, S>(choices: I) where I: IntoIterator<Item = S>, S: Into<String>` |

The generated or explicit `object_id` and RepeatButton timing are
protocol-required. RepeatButton's typestate setters may be called in either
order, but each required setter is available only once and conversion is
implemented only for the complete state.
Empty ergonomic text and choice values are valid constructor arguments but are
omitted from create JSON.

Container builders expose `child`, `children`, and `insert_child`. Leaf
builders expose none. `TabView` accepts only `Tab`; `ToggleButtonGroup` accepts
only `Button`. Rust's API enforces these cases where practical and `Validate`
repeats the rules for deserialized input.

**Picking mode** controls whether pointer hit testing may select the element.
**Delegated focus** sends focus requested on a container to a focusable child.
**Usage hints** are UI Toolkit's create-time rendering optimization flags.

All elements share the authored state below. `CommonVisualElement` is a
strictly internal backing record used by serialization, validation, and the
declarative builder-method macro. It is not exported from `battlement-ui`, and
application code never constructs, receives, or patches it directly. Create
and update builders expose the applicable field-named methods themselves.

| Rust field | Unity target | Default and wire behavior |
|---|---|---|
| `object_id` | Battlement identity map | Required UUID; never omitted |
| `name` | `VisualElement.name` | Empty; omitted |
| `enabled` | `SetEnabled` / `enabledSelf` | `true`; omitted |
| `picking_mode` | `pickingMode` | `Position`; omitted |
| `tooltip` | `tooltip` | Empty; omitted |
| `language_direction` | `languageDirection` | `Inherit`; omitted |
| `focusable` | `focusable` | Class default; omitted |
| `tab_index` | `tabIndex` | Class default; omitted |
| `delegates_focus` | `delegatesFocus` | Class default; omitted |
| `classes` | class list | Empty ordered unique list; omitted |
| `usage_hints` | `usageHints` | Empty; omitted and create-only |
| `events` | Battlement subscription table | Empty unique list; omitted |
| `style` | `VisualElement.style` | Empty; omitted |

`usage_hints` is create-only because Unity rejects changes after attachment to
a panel. `visible` is not duplicated as common state; use the `visibility`
style. `viewDataKey` is excluded because snapshot and Rust state replace local
Unity persistence.

### Aggregate updates

An **aggregate patch** combines every requested change to one element and is
applied completely or not at all. `VisualElementPatch` contains the required
`object_id` plus omitted-when-empty `common`, `style`, `parts`, `subscriptions`,
`placement`, and one type-specific patch.
The type-specific patch must match the live element class. A `ButtonUpdate`
cannot target a `Label`, even when both expose text.

On the wire, `VisualElementPatch` is externally tagged by the target class.
Its case payload has `object_id`, followed by optional `common`, `style`,
`parts`, `subscriptions`, `placement`, and `properties`. `properties` is the
matching class-specific patch record and is omitted when it has no changed
member; the outer class tag still fixes its type. No field is flattened into
`common`, and no second element-kind tag appears inside `properties`.

The internal common patch can update `name`, enabled state, picking mode,
tooltip, language direction, focusability, tab index, delegated focus, and
class additions or removals. It cannot change usage hints. `SubscriptionPatch`
contains unique `add` and `remove` lists; the lists must not overlap.
`PlacementPatch` contains a new `parent_id` and optional child index and
performs one logical reparent or reorder.

The same `Style` type is used for creation, snapshots, and aggregate updates.
Every field stores one of these three states:

| Rust state | JSON | Effect |
|---|---|---|
| `Unset` | property absent | Make no inline assignment |
| `Set(value)` | property contains `value` | Assign the exact inline value |
| `Clear` | property is `null` | Assign `StyleKeyword.Null` |

`Unset` has the context-appropriate effect: a create or snapshot leaves the
property without an inline value, while an update preserves the current inline
value. `Clear` is valid only when `Style` is carried by an update. Create and
snapshot validation reject any `Clear` field because there is no existing
inline value to remove. Rust exposes field-named `clear_*` methods on `Style`
so callers do not construct the tri-state values directly.

Rust implements the three states with a dedicated `StyleValue<T>` enum. Every
one of the 86 fields appears exactly once, on `Style`, as a
`StyleValue<ConcreteType>` where the concrete type retains its existing
inline-keyword support. `Unset` is skipped by the containing struct,
`Set(value)` serializes transparently as the concrete value, and `Clear`
serializes as JSON `null`. `Style::new()` initializes every field to `Unset`;
field-named setters store `Set`, and field-named `clear_*` methods store
`Clear`. `Style::is_empty` means every field is `Unset`. There is no separate
field-bearing `StylePatch` type; update builders accept `Style` directly.

C# mirrors `StyleValue<T>` with an `OptionalPatch<T>` value that separately
records whether the member was present and whether its token was null. Ordinary
nullable C# properties are insufficient because they collapse absent and
present-null. C# likewise declares one 86-property style record and uses it in
all three contexts. The JSON converter must reject `null` for fields that are
not clearable and the create/snapshot validators must reject a clear style
value.

The client validates the complete patch, prospective hierarchy, asset
references, and replacement leases before the first Unity setter runs. It
captures old protocol state and leases, applies properties in stable declaration
order, and rolls them back if a Unity setter throws. A successfully validated
patch is therefore atomic from the next event or render's perspective.

`StyleValue<T>` applies only to clearable inline style values. Every
other mutable member uses `SetPatch<T>`, whose exact states are `Unchanged`
(member absent) and `Set(T)` (member present). `Set(false)`, `Set(0)`, an empty
string, an empty list, and a default enum case must all serialize: they are
changes, not omission candidates. A semantically optional value uses
`SetPatch<Option<T>>`; present JSON `null` then sets `None` rather than clearing
an inline style. C# uses `OptionalPatch<T>` for both forms and records member
presence independently from the decoded value. Common and type-specific patch
builders therefore omit only `Unchanged`, never a value merely because it
equals the create default.

### One-shot actions

`VisualElementAction` has these exact cases:

| Case | Valid target and behavior |
|---|---|
| `Focus` | Focusable element; calls `Focus()` |
| `Blur` | Element in the focused panel; calls `Blur()` |
| `CapturePointer { pointer_id }` | Attached element; captures that pointer |
| `ReleasePointer { pointer_id }` | Element currently capturing that pointer |
| `ScrollTo { descendant_id }` | `ScrollView`; descendant must be in its logical content tree |
| `SelectText { cursor_index, selection_index }` | Selectable `TextElement` or text input; UTF-16 indices must be within its current text |

Scroll offset, focusability, selection preferences, and child order are
persistent fields or patches rather than actions.

## Snapshot, identities, and documents

### Snapshot shape

`Snapshot` adds these fields after `objects`:

```rust
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub ui: Vec<UiDocument>,

#[serde(default, skip_serializing_if = "PanelInputConfiguration::is_default")]
pub panel_input_configuration: PanelInputConfiguration,
```

There is one UI snapshot structure: `UiDocument`. It combines document
identity and root state directly; there is no `UiSnapshot`, `UiDocumentTree`,
or `UiRoot` wrapper. `UiDocument` is a public builder with a private protocol
representation. Every entry describes a `UIDocument` explicitly created by
Battlement. The root's class is always Unity `VisualElement`; it has no
type-specific state and cannot be reparented.

```rust
impl UiDocument {
    pub fn new(document_id: ObjectId) -> Self;
    pub fn with_root_id(document_id: ObjectId, root_id: ObjectId) -> Self;
    pub fn document_id(&self) -> ObjectId;
    pub fn root_id(&self) -> ObjectId;

    // The same field-named common, style, event, and child methods exposed by
    // an ordinary container element are available directly on this builder.
}
```

`UiDocument::new` generates a fresh root ID. `with_root_id` is the deterministic
two-scalar form for fixtures or callers that already allocated both IDs. The
builder exposes `style`, `events`, `child`, `children`, and the applicable
common field methods directly; it has no method that reveals or accepts
`CommonVisualElement`. `ui`, the internal common record, `style`, subscriptions,
and children are omitted when empty. `document_id` must resolve to a
`GameObjectKind::UiDocument` whose `root_id` equals this entry's `root_id`.
Every UI-document GameObject in `Snapshot.objects` must have exactly one
matching `UiDocument`, and every `UiDocument` must have one matching GameObject.

The entry identifies the `UIDocument` GameObject that owns its root. That
GameObject has `GameObjectKind::UiDocument(UiDocumentState)`. Its state is:

`PanelSettings` is a Rust protocol value and builder containing the supported
panel rendering, scale, target, and collider settings. It is not an asset or an
Addressables address. Unity creates a private runtime `PanelSettings` instance
for the document and copies the Rust-authored values into it. The package
runtime theme is always assigned to that instance; v1 does not expose a theme
asset reference.

| Field | Default | Unity property |
|---|---|---|
| `root_id` | Required | Identity attached to `rootVisualElement` |
| `panel_settings` | `PanelSettings::default()` | Rust settings copied into a Battlement-created runtime instance assigned to `panelSettings` |
| `position` | `Relative` | `UIDocument.position` |
| `world_space_size_mode` | `Fixed` | `worldSpaceSizeMode` |
| `world_space_size` | `1920 x 1080` | `worldSpaceSize` |
| `pivot_reference_size` | `BoundingBox` | `pivotReferenceSize` |
| `pivot` | `Center` | `pivot` |
| `sorting_order` | `0` | `sortingOrder` |

`visualTreeAsset` is always unset. All content is built programmatically. A UI
document GameObject may use ordinary Battlement scene placement, active state,
local transform, and world transform commands. It must not have another
`UIDocument` GameObject in its parent chain. Nested documents are invalid.

`UiDocumentState` is create/snapshot state. Changing its panel settings,
position mode, size mode, size, pivot, or sorting order requires destroying and
recreating that UI-document GameObject; there is no fifth UI command or
document-property patch. Existing object active/transform commands remain
valid. `ObjectDestroy` on the document GameObject recursively destroys its UI
root descendants, callbacks, captures, asset leases, and runtime panel settings
before destroying the GameObject. Command-driven `ObjectCreate` explicitly
creates the GameObject, `UIDocument`, runtime panel settings, and empty root;
subsequent `VisualElementCreate` commands populate it.

### One global identity namespace

GameObjects, document roots, and declared visual elements share one
session-wide `ObjectId` namespace. Snapshot validation builds a single index
before validating either hierarchy. A duplicate in any category is
`DuplicateObject` in Rust validation and `CoreErrorCode.DuplicateId` in Unity.

Commands resolve the ID once and then require the correct runtime kind. A
missing ID reports `UnknownObject`. An existing ID of the wrong kind reports
`ComponentMissing`; a property unsupported by the correct kind reports
`InvalidProperty`. Unity-created internal controls, viewports, labels, thumbs,
headers, and scrollers are never inserted into the identity index.

### Battlement-owned documents

Battlement only manages `UIDocument` components it explicitly creates for
`GameObjectKind::UiDocument`. It never scans for, selects, adopts, validates,
or mutates a project-authored `UIDocument`. Such components may coexist in the
Unity scene, but they never enter Battlement's identity, hierarchy, or event
systems. If `ui` is empty, Battlement creates no document.

Snapshot replacement and session teardown destroy every `UIDocument`, runtime
`PanelSettings`, callback, capture, root, and descendant that Battlement
created. Project-authored documents remain untouched.

### Panel modes

The Rust `PanelSettings` builder exposes supported scale, reference resolution,
screen matching, target display, optional prepared `RenderTexture` address,
dynamic-atlas, clearing, render-mode, and collider-policy values as field-named
consuming methods. `PanelSettings::new()` produces the documented defaults and
never accepts an asset address for a Unity `PanelSettings` object. Optional
target textures remain ordinary typed asset references and retain their own
usage leases.

All three required rendering configurations are supported. Unity 6000.5.8f1
has two `PanelRenderMode` enum values; target-texture rendering is the
screen-space mode with `PanelSettings.targetTexture` assigned rather than a
third enum value.

| Mode | Document behavior | Input behavior |
|---|---|---|
| `ScreenSpaceOverlay` with no target texture | UI Toolkit screen-space panel | Standard UI Toolkit pointer, keyboard, focus, and navigation events |
| `ScreenSpaceOverlay` with a target `RenderTexture` | Renders according to the prepared settings into that texture | No automatic pointer mapping; keyboard, focus, and programmatic actions only where Unity supplies them |
| `WorldSpace` | GameObject transform, world-space size and pivot, UI renderer, and generated document collider | UI Toolkit world-ray picking from Battlement's selected input camera |

The C# mirror uses the exact engine enum and target-texture property rather
than inventing another protocol render-mode enum. The Rust `PanelSettings`
value is the rendering source of truth for the private Unity instance.

`PanelInputConfiguration` is likewise a Rust protocol value and builder, not a
Unity asset, Addressables entry, or project-authored component. The snapshot's
single process-wide value exposes interaction layers, maximum interaction
distance, collider updates, trigger behavior, and input redirection through
field-named methods. Its defaults mirror Unity. World-space processing and
automatic panel components are required and are not configurable. Camera
selection continues to use the same Rust Battlement input API as ordinary
world input. A generated world-document collider is excluded from
`BattlementPointerInput`'s GameObject identity raycast, preventing a duplicate
world-object action for the same UI interaction.

Target-texture panels are rendering-only for pointer input. Battlement does not
install `SetScreenToPanelSpaceFunction`, accept a game-specific C# mapping
delegate, or infer UV coordinates from an object displaying the texture.

Before attaching the first interactive world-space document, Battlement
requires the existing active EventSystem and creates exactly one package-owned
Unity `PanelInputConfiguration` beneath it from the Rust value. With an explicit
selected input camera it disables `defaultEventCameraIsMainCamera` and sets
`eventCameras` to that camera; otherwise it enables the main-camera option and
leaves `eventCameras` empty. Battlement does not discover or use an authored
configuration; a project-authored component remains outside Battlement's
contract.

When the final Battlement world-space document disappears, snapshot replacement
occurs, or the session ends, Battlement destroys its configuration and the
panel components it caused Unity to create. Screen-space and target-texture
documents do not create a configuration by themselves.

## Exact element catalog

The following tables are the complete v1 runtime element set. “Common” means
the shared element state and 86 inline styles defined in this document.
“Reject” under children means deserialized child content is invalid even if
the Unity class could technically accept a physical child.

Each row is the complete per-class contract: properties not named in that row
or in an explicitly referenced shared set are excluded. The **general-event
set** means exactly `PointerDown`, `PointerMove`, `PointerUp`, `PointerCancel`,
`Click`, `PointerEnter`, `PointerLeave`, `PointerOver`, `PointerOut`, `Wheel`,
`KeyDown`, `KeyUp`, `NavigationMove`, `NavigationSubmit`, `NavigationCancel`,
`FocusIn`, `FocusOut`, `Focus`, `Blur`, `GeometryChanged`, `AttachToPanel`,
`DetachFromPanel`, `PointerCapture`, `PointerCaptureOut`, `TransitionStart`,
`TransitionEnd`, and `TransitionCancel`. Every selected class may subscribe to
that exact set; events occur only when UI Toolkit can originate them for its
current focus, picking, panel, and style state. “Text properties” means exactly
the ten `TextElement` fields named in its row. Rich-text link events are
available only on classes inheriting `TextElement` and only when rich text
contains links.

An internal element described as excluded below is excluded from logical
ownership, identity, events, and direct mutation. The closed part-style API
defined after the catalog may still style a named internal element.

Every `INotifyValueChanged<T>` class below applies a Rust value through its
public `SetValueWithoutNotify`. Display-only properties use their public
setter. `TabView`, which has no such method, uses a scoped command-origin
suppression counter around `selectedTabIndex`, active-tab, reorder, insert, and
remove calls; its callbacks return without forwarding while that counter is
nonzero. No class may use a notifying value setter for a Rust command.

### Structure and text

| Unity class; Rust builders | Required state and omitted defaults | Supported class properties | Events and controlled writes | Logical children and exclusions |
|---|---|---|---|---|
| `VisualElement`; `VisualElement`, `VisualElementUpdate` | ID only; empty name/classes, not focusable | Common | General-event set | Arbitrary children; exclude `userData`, binding, computed geometry/transform, `generateVisualContent`, manipulators, scheduler, obsolete `transform` and `cacheAsBitmap` |
| `Box`; `Box`, `BoxUpdate` | ID only; Unity adds `unity-box` | Common | General-event set | Arbitrary children |
| `TextElement`; `TextElement`, `TextElementUpdate` | ID and text; text empty, rich text and emoji fallback true, escape parsing false, elision tooltip true, selection disabled, tab index `-1` | `text`, `enable_rich_text`, `emoji_fallback_support`, `parse_escape_sequences`, `display_tooltip_when_elided`, `selectable`, `double_click_selects_word`, `triple_click_selects_line`, `select_all_on_focus`, `select_all_on_mouse_up` | General-event set, four link events, and `SelectionChanged`; Rust text uses `SetValueWithoutNotify` | Reject authored children; exclude glyph post-processing, measurement/buffer APIs, `isElided`, cursor geometry, and editing API |
| `Label`; `Label`, `LabelUpdate` | ID and text; otherwise `TextElement` defaults | Same ten supported text properties | General-event set and four link events; no normal user value proposal | Reject children and text measurement APIs |
| `Button`; `Button`, `ButtonUpdate` | ID and text; no icon; focusable with tab index `0` | Ten text properties and `icon: Option<IconSource>` | General-event set; `Click` maps pointer and navigation activation as specified below | Reject children; exclude `Clickable`, C# delegate constructors, and obsolete `onClick` |
| `RepeatButton`; `RepeatButton`, `RepeatButtonUpdate` | ID, text, nonnegative initial delay, and positive repeat interval are required | Ten text properties, `delay_ms: u32`, `interval_ms: NonZeroU32` | General-event set; `Click::Repeat` for every fixed forwarding invocation | Reject children; never serialize `Action` or expose `SetAction` |
| `Image`; `Image`, `ImageUpdate` | ID; no source, scale-to-fit, white tint, **UV texture coordinates** `(0,0,1,1)` | Exclusive addressed `source: Option<ImageSource>`, `source_rect`, `tint_color`, `scale_mode`, `uv` | General-event set | Reject children; `source_rect` is invalid for sprites; raw Unity objects excluded |
| `GroupBox`; `GroupBox`, `GroupBoxUpdate` | ID; empty title creates no internal label | `text` | General-event set | Arbitrary children except Rust `RadioButton` descendants; internal title label has no ID but has a typed style slot; use `RadioButtonGroup` for exclusive choices |
| `PopupWindow`; `PopupWindow`, `PopupWindowUpdate` | ID; empty text and internal content container | Text properties | General-event set and four link events | Arbitrary children through `contentContainer`; no positioning, modal, menu, or lifecycle promise |

### Containers

| Unity class; Rust builders | Required state and omitted defaults | Supported class properties | Events and controlled writes | Logical children and exclusions |
|---|---|---|---|---|
| `ScrollView`; `ScrollView`, `ScrollViewUpdate` | ID; vertical, offset zero, wheel size `18`, deceleration `0.135`, elasticity `0.1`, elastic interval `16 ms`, clamped touch | `mode`, `nested_interaction`, horizontal/vertical scroller visibility, `scroll_offset`, horizontal/vertical page size, `mouse_wheel_scroll_size`, `touch_scroll_behavior`, `scroll_deceleration_rate`, `elasticity`, `elastic_animation_interval` | General-event set plus `ScrollSettled`; `ScrollChanged` is live scroll; Rust offset without notification | Arbitrary children route to content container; viewport, scrollers, sliders, tracks, draggers, and buttons have typed style slots but no IDs; `ScrollTo` is an action; obsolete show flags excluded |
| `Scroller`; `Scroller`, `ScrollerUpdate` | ID; low/high/value `0`, vertical | `low_value: f32`, `high_value: f32`, `direction: SliderDirection`, `value: f32` | General-event set plus final `ValueCommitted(F32)`; `ValueChanging(F32)` is live value; Rust value without notification | Reject children; internal slider/buttons have typed style slots; adjustment methods excluded |
| `Tab`; `Tab`, `TabUpdate` | ID; empty label/icon, not closeable | `label: String`, `icon: Option<IconSource>`, `closeable: bool` | General-event set and mandatory `TabCloseRequested` when closeable | Arbitrary children route to content container; header parts have typed style slots; delegates excluded |
| `TabView`; `TabView`, `TabViewUpdate` | ID; no tabs means selected index `None`; first tab is selected when nonempty; reorderable false | `selected_index: Option<u32>`, `reorderable: bool` | General-event set plus `ValueCommitted(Index)` and `TabReordered`; command-origin guard prevents echo | Only `Tab` children; header/content containers have typed style slots; lookup methods, delegates, and view persistence excluded |

### Text input

`TextField` supports common state plus `label: String`,
`show_mixed_value: bool`, `max_length: i32`, `is_password: bool`,
`is_read_only: bool`, `mask_character: char`,
`placeholder: String`, `hide_placeholder_on_focus: bool`,
`vertical_scroller_visibility: ScrollerVisibility`,
`select_all_on_focus: bool`, `select_all_on_mouse_up: bool`,
`double_click_selects_word: bool`, `triple_click_selects_line: bool`,
`emoji_fallback_support: bool`, `hide_soft_keyboard_on_focus: bool`,
`hide_mobile_input: bool`, `keyboard_type: TouchScreenKeyboardType`, and
`auto_correction: bool`. The defaults are empty label and placeholder,
unlimited length, not password, `mask_character = '*'`,
not read-only after field initialization, hidden vertical scroller, do not
select all on focus or mouse-up, do select words/lines on double/triple click,
emoji fallback enabled, do not hide either mobile keyboard surface, default
keyboard type, and automatic correction disabled. The platform-wide mobile
input appearance may override `hide_mobile_input`, matching Unity.

Battlement sets the native `isDelayed` behavior needed by its controlled commit
policy and does not expose that Unity property separately. It sets placeholder
state through the public `ITextEdition` interface while keeping the interface
object itself out of the wire protocol. Unity selection handles,
touch-keyboard objects, cursor coordinates, and editing objects are excluded.

`TextField` supports the general-event set plus `Input`, `ValueCommitted`, and
`SelectionChanged`. `Input` and `SelectionChanged` are explicit high-frequency
subscriptions. `ValueCommitted` is also explicit and uses `UiValue::String`.

| Unity class; Rust builders | Required state and omitted defaults | Additional properties and wire type | User events and Rust writes | Children |
|---|---|---|---|---|
| `TextField`; `TextField`, `TextFieldUpdate` | ID; empty value/label, unlimited, single-line, non-password, `'*'` mask | `value: String`, `multiline` | `Input` only when subscribed; `ValueCommitted(String)` on Enter/focus loss; `SetValueWithoutNotify` | Reject |

### Choice and Boolean controls

| Unity class; Rust builders | Required state and omitted defaults | Supported class properties | Events and controlled writes | Logical children and exclusions |
|---|---|---|---|---|
| `Toggle`; `Toggle`, `ToggleUpdate` | ID; false, no label | `label: String`, `show_mixed_value: bool`, `value: bool` | General-event set plus `ValueCommitted(Bool)`; `SetValueWithoutNotify` | Reject logical children; label/input/checkmark/text have typed style slots |
| `RadioButton`; `RadioButton`, `RadioButtonUpdate` | ID; false, no label | `label: String`, `show_mixed_value: bool`, `value: bool` | General-event set plus `ValueCommitted(Bool)`; standalone controlled Boolean only | Reject children, obsolete `SetSelected`, and Rust `GroupBox` ancestry; C# mounts it inside a one-option package-owned GroupBox with no ID so Unity cannot coordinate it with another authored radio; use `RadioButtonGroup` for exclusive choices |
| `RadioButtonGroup`; `RadioButtonGroup`, `RadioButtonGroupUpdate` | ID and ordered choices; no selection, no label | `label: String`, `show_mixed_value: bool`, `choices: Vec<String>`, `selected_index: Option<u32>` | General-event set plus `ValueCommitted(Index)`; Rust uses `SetValueWithoutNotify` | Reject Rust children; generated radio buttons have no IDs; no authored descendants can change index semantics |
| `ToggleButtonGroup`; `ToggleButtonGroup`, `ToggleButtonGroupUpdate` | ID and zero to 64 `Button` children; single selection, empty selection disallowed; first child selected when nonempty | `label: String`, `show_mixed_value: bool`, `multiple_selection: bool`, `allow_empty_selection: bool`, unique sorted `selected_indices: Vec<u32>` | General-event set plus one `ValueCommitted(Indices)`; Rust constructs Unity's mask and calls `SetValueWithoutNotify` | Only `Button` children; order defines indices; internal 64-bit mask and `GetButton` excluded |
| `DropdownField`; `DropdownField`, `DropdownFieldUpdate` | ID and ordered choices; no selection, no label | `label: String`, `show_mixed_value: bool`, `choices: Vec<String>`, `selected_index: Option<u32>` | General-event set plus `ValueCommitted(Choice)`; Rust uses `SetValueWithoutNotify` | Reject logical children and formatting callbacks; field parts have typed style slots; Rust supplies display-ready strings |

For radio and dropdown groups, a selected index must name an existing choice.
`None` maps to Unity index `-1` and an empty displayed value. Duplicate choice
strings are allowed because selection identity is the index, not the label.
For a nonempty `TabView`, `selected_index = None` is invalid and omission
derives index `0`; an empty TabView requires `None`. Removing the selected Tab
selects `min(previous_index, remaining_count - 1)` under the command-origin
guard, or `None` when no tabs remain. For a nonempty `ToggleButtonGroup` with
`allow_empty_selection = false`, an empty selection is invalid and omission
derives `[0]`; removing its selected Button derives `[0]` among the remaining
children. Single-selection mode rejects more than one selected index.

Rust-authored `RadioButton` does not participate in Unity's panel-wide or
GroupBox-wide `DefaultGroupManager`. That manager deselects siblings through
notifying setters and cannot preserve Battlement's controlled-write rule. The
package-owned one-option physical wrapper isolates each standalone radio while
leaving its Rust logical parent and `ObjectId` unchanged. Validation rejects a
Rust `RadioButton` anywhere under a Rust `GroupBox`. Exclusive authoring uses
`RadioButtonGroup`, whose single selected-index value is restored and applied
with `SetValueWithoutNotify` as one controlled control. The fake enforces the
same child/ancestry rule and does not infer sibling selection for standalone
radios.

### Range and status controls

| Unity class; Rust builders | Required state and omitted defaults | Supported class properties | Events and controlled writes | Children and exclusions |
|---|---|---|---|---|
| `Slider`; `Slider`, `SliderUpdate` | ID; range `0..10`, value `0`, horizontal, page size `0`, no fill/input, not inverted | `low_value: f32`, `high_value: f32`, `value: f32`, `fill: bool`, `page_size: f32`, `show_input_field: bool`, `direction: SliderDirection`, `inverted: bool` | General-event set plus final `ValueCommitted(F32)` on release; `ValueChanging(F32)` is live value; Rust without notification | Reject logical children; label, input, track, dragger, fill, and optional text input have typed style slots |
| `SliderInt`; `SliderInt`, `SliderIntUpdate` | Same defaults as `Slider`, integer values | `low_value: i32`, `high_value: i32`, `value: i32`, `fill: bool`, `page_size: i32`, `show_input_field: bool`, `direction: SliderDirection`, `inverted: bool` | General-event set plus final `ValueCommitted(I32)`; `ValueChanging(I32)` is live value | Same native-part style methods as `Slider`; reject logical children |
| `MinMaxSlider`; `MinMaxSlider`, `MinMaxSliderUpdate` | ID; selected `0..10`, limits `Unbounded`, mapped to Unity's `float.MinValue` and `float.MaxValue` defaults without putting extreme values on the wire | `min_value: f32`, `max_value: f32`, `low_limit: LowerLimit`, `high_limit: UpperLimit`; a set limit is finite | General-event set plus final `ValueCommitted(F32Range)`; `ValueChanging(F32Range)` is live range; values clamp to limits | Reject logical children and read-only range; range parts have typed style slots |
| `ProgressBar`; `ProgressBar`, `ProgressBarUpdate` | ID; low `0`, high `100`, value `0`, empty title | `low_value: f32`, `high_value: f32`, `value: f32`, `title: String` | General-event set only; output-only in Battlement; Rust uses `SetValueWithoutNotify` | Reject logical children; background, progress, and title have typed style slots |

Ranges require `low <= high`; values must lie within the declared range after
the patch is applied. All transmitted floating-point values must be finite.
`LowerLimit` is `Unbounded` or `Inclusive(f32)` and `UpperLimit` is the matching
upper form. `Unbounded` is the omitted default and maps to Unity's native
finite `float.MinValue` or `float.MaxValue` endpoint without serializing either
long decimal value.

### Styles for Unity-created parts

Unity-created implementation elements remain outside the logical child tree:
they have no `ObjectId`, cannot receive common properties or subscriptions,
and cannot be reparented or destroyed independently. They are nevertheless
first-class style targets through owner-specific builder methods. Every named
slot becomes a consuming `<slot>_style(style: Style) -> Self` method on that
control's create and update builders. An update builder additionally exposes
`clear_<slot>_style() -> Self` to remove all inline overrides owned by the part.
There is no public part enum or generic `part_style` method, so another
control's parts cannot appear in the method set. A small `macro_rules!` family
generates these repetitive methods and their private wire keys without a
procedural macro. Application code cannot pass a string, CSS selector, or
hierarchy path.

For example, a vertical scrollbar can be authored without taking ownership of
Unity's physical elements:

```rust
ScrollView::new()
    .vertical_track_style(
        Style::new().background_color(Color::rgb8(24, 28, 36)),
    )
    .vertical_dragger_style(
        Style::new().background_color(Color::rgb8(92, 104, 124)),
    )
    .vertical_low_button_style(Style::new().display(Display::None))
    .vertical_high_button_style(Style::new().display(Display::None))
```

The table lists public method stems. For example, `vertical_track` means
`vertical_track_style(Style)` on both builders and
`clear_vertical_track_style()` on the update builder.

| Builders | Public style method stems |
|---|---|
| `Button`, `ButtonUpdate` | `icon` |
| `GroupBox`, `GroupBoxUpdate` | `title` |
| `PopupWindow`, `PopupWindowUpdate` | `content_container` |
| `ScrollView`, `ScrollViewUpdate` | `content_and_vertical_scroll_container`, `viewport`, `content_container`, `horizontal_scroller`, `horizontal_slider`, `horizontal_low_button`, `horizontal_high_button`, `horizontal_track`, `horizontal_dragger`, `horizontal_dragger_border`, and the corresponding seven `vertical_...` stems |
| `Scroller`, `ScrollerUpdate` | `slider`, `low_button`, `high_button`, `track`, `dragger`, `dragger_border` |
| `Tab`, `TabUpdate` | `header`, `label`, `icon`, `underline`, `close_button`, `drag_handle`, `drag_handle_leading_bar`, `drag_handle_trailing_bar`, `content_container` |
| `TabView`, `TabViewUpdate` | `content_viewport`, `header_container`, `content_container`, `previous_button`, `next_button` |
| `TextField`, `TextFieldUpdate` | `label`, `input`, `text_element`, `multiline_scroll_view`, `vertical_scroller`, `vertical_slider`, `vertical_low_button`, `vertical_high_button`, `vertical_track`, `vertical_dragger`, `vertical_dragger_border` |
| `Toggle`, `ToggleUpdate` | `label`, `input`, `checkmark`, `text` |
| `RadioButton`, `RadioButtonUpdate` | `label`, `input`, `checkmark_background`, `checkmark`, `text` |
| `RadioButtonGroup`, `RadioButtonGroupUpdate` | `label`, `input`, `choices_container`, `content_container`, `all_options`; indexed methods are `option_style(index, style)`, `option_checkmark_background_style(index, style)`, `option_checkmark_style(index, style)`, and `option_text_style(index, style)`, with matching update clear methods |
| `ToggleButtonGroup`, `ToggleButtonGroupUpdate` | `label`, `input`; its logical `Button` children use their own ordinary styles |
| `DropdownField`, `DropdownFieldUpdate` | `label`, `input`, `text`, `arrow` |
| `Slider`/`SliderInt` create and update builders | `label`, `input`, `track`, `dragger`, `dragger_border`, `fill`, `text_input` |
| `MinMaxSlider`, `MinMaxSliderUpdate` | `label`, `input`, `track`, `minimum_thumb`, `maximum_thumb`, `range_dragger` |
| `ProgressBar`, `ProgressBarUpdate` | `container`, `background`, `progress`, `title_container`, `title` |

The list is closed for Unity 6000.5.8f1. The C# adapter captures each supported
native reference when it constructs the control or materializes a conditional
part, using a public element property where Unity exposes one and an audited,
control-scoped class-name lookup otherwise. Command execution never performs a
global query. A missing or ambiguous required part fails with `UnityException`
so a Unity upgrade cannot silently style the wrong element.

Part styles serialize as an omitted-when-empty `parts` list of `{ part, style
}` records inside the owning create, snapshot, or aggregate-update record.
Create and snapshot styles are non-null. In an update, `style: null` is the
generated `clear_<slot>_style` operation, while a present `Style` patches only
its present fields. Private part keys must be unique within one record. Indexed
radio-group parts must name an existing choice; `AllOptions` applies first and
an indexed style then overrides the fields it sets. A style for a conditional
part such as
`title`, `icon`, `button`, `close_button`, `multiline_scroll_view`, `fill`, or
`text_input` is valid only when the aggregate's final state creates that part.
Removing a styled conditional part requires its generated clear method in the
same aggregate, which releases its style asset leases. Property changes and
part styles in one aggregate update are validated and applied together.

Part styles use the complete `Style` contract and asset-lease behavior below.
Creates and snapshots reject clears; updates accept per-field clears. Part
style application is included in detached construction and aggregate rollback,
and destruction releases its asset leases. The fake stores and patches every
part style by its typed key but does not model the physical elements.

### Explicitly unsupported elements and surfaces

The first implementation excludes:

- `BindableElement`, `TemplateContainer`, Unity data binding, **UXML (Unity XML
  markup)** cloning, and runtime UXML or **USS (Unity Style Sheets)** authoring.
- `IMGUIContainer`, `ImmediateModeElement`, **IMGUI (Unity's immediate-mode
  graphical user interface)**, custom mesh generation, arbitrary
  render callbacks, and custom materials.
- Inspector-oriented `HelpBox`, `Foldout`, typed numeric text fields,
  `TwoPaneSplitView`, generic `PopupField<T>`, and enum, mask, GUID, hash,
  bounds, rectangle, and vector fields.
- `ListView`, `TreeView`, `MultiColumnListView`, and
  `MultiColumnTreeView`. Their make/bind/unbind/recycle callbacks would require
  synchronous Rust virtualization. Use `ScrollView` with ordinary Rust-owned
  children.
- Editor drag-and-drop, command events, contextual-menu population, tooltip
  callbacks, custom-style callbacks, mouse compatibility duplicates, and
  obsolete APIs.

These exclusions are capability boundaries, not invitations for custom C# to
make game-rule or visual-state decisions.

## Inline style contract

### Style value types

Every supported style field is `Unset` by default. `Style` writes only fields
explicitly set or cleared. All 86 fields are clearable with JSON `null` in an
update; creation and snapshots reject clears.

Builder setters accept ergonomic inputs with `Into` while the stored and wire
types remain the exact types in the table below. In particular:

- An integer passed to a length-valued setter means pixels, so `.padding(24)`,
  `.width(96)`, and `.row_gap(12)` do not require casts or decimal literals.
  `f32` remains accepted for fractional pixels.
- `LengthUnits`, reexported by `battlement_ui::prelude`, adds `.px()` and
  `.pct()` to `i32`, `u32`, and `f32`. Percentages stay explicit, as in
  `.width(100.pct())`; plain numeric values never mean percentages.
- Float-valued setters accept `impl Into<FloatValue>`. `FloatValue` has
  conversions from `i32`, `u32`, and `f32`, so `.flex_grow(1)` is valid while
  finiteness and property-specific bounds remain validated centrally.
- Asset-valued setters accept the relevant typed address directly. `From`
  implementations convert `TextureAddress`, `SpriteAddress`,
  `VectorImageAddress`, and `RenderTextureAddress` into `ImageSource`,
  `IconSource`, or `BackgroundSource` wherever that source kind is valid.
  Callers therefore write `.source(texture_address)` rather than naming the
  enum case. Explicit enum construction remains available when useful.

The four-sided style families additionally expose one CSS-order shorthand
instead of separate `_all`, `_horizontal`, or `_vertical` methods:

```rust
Style::new()
    .padding(24)
    .margin((8, 16))
    .border_width((1, 2, 3, 4))
```

`padding`, `margin`, `border_width`, and `border_color` accept one value for all
sides, `(vertical, horizontal)`, `(top, horizontal, bottom)`, or
`(top, right, bottom, left)`. `border_radius` accepts the analogous one-, two-,
three-, or four-value corner form in top-left, top-right, bottom-right,
bottom-left order. Each component independently uses the same `Into` conversion
as its corresponding individual setter. The shorthand expands into the four
ordinary fields before serialization; it adds no aggregate wire type. The
individual side and corner setters remain available for isolated changes, and
later calls win when a shorthand and an individual setter overlap.

| Rust type | Accepted values and validation | Unity conversion |
|---|---|---|
| `Length` | finite `Px(f32)` or `Percent(f32)` | `UnityEngine.UIElements.Length` |
| `LengthOrAuto` | `Px`, `Percent`, or `Auto` | `StyleLength`; `Auto` is explicit |
| `FloatValue` | finite `f32`; property-specific bounds below | `StyleFloat` |
| `AspectRatio` | `Auto`, or finite positive width and height whose `width / height` quotient is finite | `StyleRatio.Auto()` or `StyleRatio` receiving the single `width / height` float |
| `Color` | finite RGBA components in `[0,1]` | `StyleColor` |
| `BackgroundSource` | prepared texture, sprite, vector image, render texture, linear gradient, or radial gradient | `StyleBackground` / `Background` |
| `BackgroundPosition` | horizontal/vertical keyword plus finite offset | `StyleBackgroundPosition` |
| `BackgroundRepeat` | independent x/y `Repeat`, `NoRepeat`, `Round`, or `Space` | `StyleBackgroundRepeat` |
| `BackgroundSize` | `Auto`, `Cover`, `Contain`, or two `LengthOrAuto` axes | `StyleBackgroundSize` |
| `Cursor` | `Default` or prepared texture with finite nonnegative hotspot | `StyleCursor` |
| `Rotate` | finite degrees around a finite axis; zero axis invalid | `StyleRotate` |
| `Scale` | finite x/y values | `StyleScale` |
| `Translate` | x/y `Length` and finite z pixels | `StyleTranslate` |
| `TransformOrigin` | x/y `Length` plus finite z pixels | `StyleTransformOrigin` |
| `TextShadow` | finite x/y offset and blur radius plus color; blur nonnegative | `StyleTextShadow` |
| `TransitionList<T>` | zero or more values; parallel transition lists follow UI Toolkit repetition rules | `StyleList<T>` |
| `TimeValue` | finite nonnegative milliseconds for duration; delay may be negative | UI Toolkit seconds |
| `FontSource` | prepared legacy Unity `Font` or UI Toolkit/TextCore font asset | `StyleFont` or `StyleFontDefinition` |
| `TextAutoSize` | `None`, or `BestFit` with finite positive pixel minimum and maximum and minimum no greater than maximum; `None` uses Unity's `10px`/`100px` stored bounds | `StyleTextAutoSize` |

Every row also accepts `InlineKeyword::Initial`, encoded as
`{"Keyword":"Initial"}` and assigned as `StyleKeyword.Initial`; its normal
concrete value keeps the direct encoding shown in the table. Property-specific
`Auto` and `None` cases are part of the listed concrete type—for example,
`AspectRatio::Auto`, `LengthOrAuto::Auto`, `Display::None`, and
`TextAutoSize::None`—so they never serialize as JSON `null`. A clear remains
the patch-only JSON `null` operation that assigns `StyleKeyword.Null`.

`Length` percentages are not globally clamped because layout positions,
transforms, and sizes may intentionally exceed `0..100`. Border widths,
outline widths, slice sizes, and text-shadow blur must be nonnegative. Opacity
is in `0..1`. Flex grow and shrink are nonnegative. Integer slice sizes are
nonnegative. All enum cases use the Unity names converted to Rust casing.
`Cursor::Default` assigns a default `StyleCursor`; the texture case assigns a
prepared `Texture2D` and hotspot. Named operating-system cursors are excluded
because UI Toolkit's runtime system-cursor identifier is not a public setter in
the audited source.

### Complete 86-property matrix

The Rust field is the snake-case form shown below. Every row is one
`StyleValue<T>` field on `Style`, defaults to `Unset`, and is omitted when
unset. Every row accepts JSON `null` when used in an update and rejects it in a
create or snapshot. “Clear” therefore means `StyleKeyword.Null` in every row.

| Rust field | Rust value | Unity `IStyle` property | Additional validation |
|---|---|---|---|
| `align_content` | `Align` | `alignContent` | `Auto`, `FlexStart`, `Center`, `FlexEnd`, `Stretch` |
| `align_items` | `Align` | `alignItems` | Same |
| `align_self` | `Align` | `alignSelf` | Same |
| `aspect_ratio` | `AspectRatio` | `aspectRatio` | `Auto`, or both components positive with finite quotient |
| `background_color` | `Color` | `backgroundColor` | Components `0..1` |
| `background_image` | `BackgroundSource` | `backgroundImage` | Asset prepared and compatible, or valid gradient |
| `background_position_x` | `BackgroundPosition` | `backgroundPositionX` | Horizontal keyword |
| `background_position_y` | `BackgroundPosition` | `backgroundPositionY` | Vertical keyword |
| `background_repeat` | `BackgroundRepeat` | `backgroundRepeat` | Valid x/y modes |
| `background_size` | `BackgroundSize` | `backgroundSize` | Finite axes |
| `border_bottom_color` | `Color` | `borderBottomColor` | Components `0..1` |
| `border_bottom_left_radius` | `Length` | `borderBottomLeftRadius` | Nonnegative |
| `border_bottom_right_radius` | `Length` | `borderBottomRightRadius` | Nonnegative |
| `border_bottom_width` | `f32` | `borderBottomWidth` | Nonnegative |
| `border_left_color` | `Color` | `borderLeftColor` | Components `0..1` |
| `border_left_width` | `f32` | `borderLeftWidth` | Nonnegative |
| `border_right_color` | `Color` | `borderRightColor` | Components `0..1` |
| `border_right_width` | `f32` | `borderRightWidth` | Nonnegative |
| `border_top_color` | `Color` | `borderTopColor` | Components `0..1` |
| `border_top_left_radius` | `Length` | `borderTopLeftRadius` | Nonnegative |
| `border_top_right_radius` | `Length` | `borderTopRightRadius` | Nonnegative |
| `border_top_width` | `f32` | `borderTopWidth` | Nonnegative |
| `bottom` | `LengthOrAuto` | `bottom` | Finite length |
| `color` | `Color` | `color` | Components `0..1` |
| `column_gap` | `Length` | `columnGap` | Nonnegative |
| `cursor` | `Cursor` | `cursor` | `Default`, or prepared texture and nonnegative hotspot |
| `display` | `Display` | `display` | `Flex` or `None` |
| `flex_basis` | `LengthOrAuto` | `flexBasis` | Finite length |
| `flex_direction` | `FlexDirection` | `flexDirection` | `Column`, `ColumnReverse`, `Row`, `RowReverse` |
| `flex_grow` | `f32` | `flexGrow` | Nonnegative |
| `flex_shrink` | `f32` | `flexShrink` | Nonnegative |
| `flex_wrap` | `FlexWrap` | `flexWrap` | `NoWrap`, `Wrap`, `WrapReverse` |
| `font_size` | `Length` | `fontSize` | Positive |
| `height` | `LengthOrAuto` | `height` | Finite; negative rejected |
| `justify_content` | `Justify` | `justifyContent` | `FlexStart`, `Center`, `FlexEnd`, `SpaceBetween`, `SpaceAround`, `SpaceEvenly` |
| `left` | `LengthOrAuto` | `left` | Finite length |
| `letter_spacing` | `Length` | `letterSpacing` | Finite |
| `margin_bottom` | `LengthOrAuto` | `marginBottom` | Finite |
| `margin_left` | `LengthOrAuto` | `marginLeft` | Finite |
| `margin_right` | `LengthOrAuto` | `marginRight` | Finite |
| `margin_top` | `LengthOrAuto` | `marginTop` | Finite |
| `max_height` | `LengthOrAuto` | `maxHeight` | Finite; negative rejected |
| `max_width` | `LengthOrAuto` | `maxWidth` | Finite; negative rejected |
| `min_height` | `LengthOrAuto` | `minHeight` | Finite; negative rejected |
| `min_width` | `LengthOrAuto` | `minWidth` | Finite; negative rejected |
| `opacity` | `f32` | `opacity` | `0..1` |
| `overflow` | `Overflow` | `overflow` | `Visible` or `Hidden` |
| `padding_bottom` | `Length` | `paddingBottom` | Nonnegative |
| `padding_left` | `Length` | `paddingLeft` | Nonnegative |
| `padding_right` | `Length` | `paddingRight` | Nonnegative |
| `padding_top` | `Length` | `paddingTop` | Nonnegative |
| `position` | `Position` | `position` | `Relative` or `Absolute` |
| `right` | `LengthOrAuto` | `right` | Finite length |
| `rotate` | `Rotate` | `rotate` | Finite, nonzero axis |
| `row_gap` | `Length` | `rowGap` | Nonnegative |
| `scale` | `Scale` | `scale` | Finite |
| `text_overflow` | `TextOverflow` | `textOverflow` | `Clip` or `Ellipsis` |
| `text_shadow` | `TextShadow` | `textShadow` | Nonnegative blur |
| `top` | `LengthOrAuto` | `top` | Finite length |
| `transform_origin` | `TransformOrigin` | `transformOrigin` | Finite |
| `transition_delay` | `TransitionList<TimeValue>` | `transitionDelay` | Finite; negative allowed |
| `transition_duration` | `TransitionList<TimeValue>` | `transitionDuration` | Nonnegative |
| `transition_property` | `TransitionList<TransitionProperty>` | `transitionProperty` | Only names in this 86-property set plus `All` |
| `transition_timing_function` | `TransitionList<EasingFunction>` | `transitionTimingFunction` | `Ease`, `EaseIn`, `EaseOut`, `EaseInOut`, `Linear`, and the `EaseIn`, `EaseOut`, and `EaseInOut` variants for Sine, Cubic, Circ, Elastic, Back, and Bounce |
| `translate` | `Translate` | `translate` | Finite |
| `unity_background_image_tint_color` | `Color` | `unityBackgroundImageTintColor` | Components `0..1` |
| `unity_font` | `LegacyFontAddress` | `unityFont` | Prepared legacy Unity `Font` |
| `unity_font_definition` | `UiFontAddress` | `unityFontDefinition` | Prepared UI Toolkit/TextCore font asset |
| `unity_font_style_and_weight` | `FontStyle` | `unityFontStyleAndWeight` | `Normal`, `Bold`, `Italic`, `BoldAndItalic` |
| `unity_overflow_clip_box` | `OverflowClipBox` | `unityOverflowClipBox` | `PaddingBox` or `ContentBox` |
| `unity_paragraph_spacing` | `Length` | `unityParagraphSpacing` | Finite |
| `unity_slice_bottom` | `i32` | `unitySliceBottom` | Nonnegative |
| `unity_slice_left` | `i32` | `unitySliceLeft` | Nonnegative |
| `unity_slice_right` | `i32` | `unitySliceRight` | Nonnegative |
| `unity_slice_scale` | `f32` | `unitySliceScale` | Positive |
| `unity_slice_top` | `i32` | `unitySliceTop` | Nonnegative |
| `unity_slice_type` | `SliceType` | `unitySliceType` | `Sliced` or `Tiled` |
| `unity_text_align` | `TextAnchor` | `unityTextAlign` | Nine Unity anchor cases |
| `unity_text_auto_size` | `TextAutoSize` | `unityTextAutoSize` | `None` or `BestFit`; positive ordered pixel min/max |
| `unity_text_outline_color` | `Color` | `unityTextOutlineColor` | Components `0..1` |
| `unity_text_outline_width` | `f32` | `unityTextOutlineWidth` | Nonnegative |
| `unity_text_overflow_position` | `TextOverflowPosition` | `unityTextOverflowPosition` | `Start`, `Middle`, or `End` |
| `visibility` | `Visibility` | `visibility` | `Visible` or `Hidden` |
| `white_space` | `WhiteSpace` | `whiteSpace` | `Normal`, `NoWrap`, `Pre`, or `PreWrap` |
| `width` | `LengthOrAuto` | `width` | Finite; negative rejected |
| `word_spacing` | `Length` | `wordSpacing` | Finite |

The seven audited writable properties intentionally excluded are
`animationPlayState`, `backdropFilter`, `filter`, `unityAnimationClip`,
`unityEditorTextRenderingMode`, `unityMaterial`, and `unityTextGenerator`.
There is no generic string escape hatch for them or for future Unity style
properties. Adding a property requires Rust and C# types, validation, JSON
parity tests, and this table to change together.

### Backgrounds and gradients

`BackgroundSource` has exact cases for prepared `TextureAddress`,
`SpriteAddress`, `VectorImageAddress`, and `RenderTextureAddress`, plus one
inline `BackgroundGradient` case. An asset case holds only its typed address.
The gradient mirrors Unity's audited `BackgroundGradient`: `Linear` or
`Radial`, one to four ordered `BackgroundGradientStop` values, a finite linear
angle in radians, radial `Ellipse` or `Circle` shape, radial `FarthestCorner`,
`FarthestSide`, `ClosestCorner`, or `ClosestSide` extent, and a radial center
whose x/y fractions are each in `0..1`. A stop contains a color and either a
percentage fraction in `0..1` or a finite pixel position. Stop order is
preserved; Battlement does not sort or reinterpret mixed percentage/pixel
stops. More than four stops are rejected because Unity's audited renderer keeps
only four and would otherwise truncate the value.

`ImageSource` is a separate closed enum containing the same four asset cases
and no gradient case, so Rust rejects an Image gradient at construction rather
than only during validation. Assigning one source clears the other three native
source properties before setting the chosen one. Button and Tab icons use
`IconSource`, another asset-only enum, because gradients are backgrounds rather
than control icons.

`Image.source`, `Button.icon`, and `Tab.icon` are create-state
`Option<...>` values omitted when `None`. Their update members are
`SetPatch<Option<...>>`: absence means unchanged, a present asset sets or
replaces it, and present JSON `null` sets `None`, clears every corresponding
native source property, and releases the old usage lease.

## Addressable assets and leases

`battlement-types` adds `SpriteAddress`, `VectorImageAddress`,
`RenderTextureAddress`, `LegacyFontAddress`, and `UiFontAddress`. Existing
`TextureAddress` remains the `Texture2D` address. Existing `FontAddress`
continues to mean the TextMesh Pro font used by Battlement's world-space text
and is not silently redefined. Neither `PanelSettings` nor
`PanelInputConfiguration` has an address type.

`PreparedAsset` adds matching `Sprite`, `VectorImage`, `RenderTexture`,
`LegacyFont`, and `UiFont` cases. `LegacyFont` resolves to
`UnityEngine.Font`; `UiFont` resolves to a UI Toolkit-compatible
`UnityEngine.TextCore.Text.FontAsset`, including compatible derived assets. The
C# asset store validates the exact resolved Unity type before the set becomes
active. UI commands never initiate an Addressables load; they may use only the
active prepared set.

`Battlement.UI` defines the narrow `IBattlementUiAssetLookup` and
`IBattlementUiAssetLease` interfaces it consumes. `Battlement.Runtime`, which
already owns `BattlementPreparedAssets`, implements those interfaces and passes
the implementation into the UI manager; `Battlement.UI` never references the
runtime assembly. Every live asset-backed property, including a panel's
optional target texture, owns one lease. A detached subtree acquires all leases
before attachment. A patch acquires replacement leases before releasing old
ones. Recursive destruction, root cleanup, session teardown, and authoritative
snapshot replacement release all UI leases. A
command-driven prepared-set replacement that removes an in-use UI asset fails
with `AssetInUse`; an authoritative snapshot replacement may retire the entry
until its final lease is released, matching existing asset behavior.

## Event contract

### Shared physical vocabulary and separate delivery

World and UI events use the same `PhysicalKey`, `KeyModifiers`,
`PointerButton`, and `PointerType` values. They deliberately do not share one
outer input event or one subscription system:

| Concern | World delivery | UI delivery |
|---|---|---|
| Target | Global key selection or collider-resolved GameObject | Focused or picked visual element |
| Dispatch | Flat core `ActionBody` case | One `ActionBody::VisualElement(UiEvent)` routed through the Rust visual tree |
| Coordinates | Bottom-left screen position plus world hit | Upper-left panel position, delta, and related UI target where applicable |
| Click meaning | Matching pointer press and release on one runtime object | Pointer activation, navigation activation, or repeat activation |
| Keyboard meaning | Selected physical transitions; repeat suppressed | Focused key events with text and native repeat state |

This separation prevents invalid combinations such as a navigation click with
a world hit or a global key transition with a UI phase. It does not create a
second definition of a key, modifier, pointer button, or pointer device.

### Subscription and routing model

`UiEventSubscription` contains an event kind and `UiEventPhase`. The exact
phase cases are `Trickle`, `Target`, and `Bubble`; `Target` is the omitted
default used by the `.events([UiEventKind])` builder shorthand. For a
propagating event, a subscription on a strict ancestor may be `Trickle` or
`Bubble`, while a subscription on the originating target must be `Target`.
For an event that does not propagate, only `Target` is valid. Duplicate
`(kind, phase)` pairs on one element are invalid.

Routing is deterministic and never invokes one element twice for one
subscription. Given `root -> panel -> button`, a pointer event originating at
`button` is delivered in this order: `root/Trickle`, `panel/Trickle`,
`button/Target`, `panel/Bubble`, `root/Bubble`, omitting any pair that is not
subscribed. A `Trickle` or `Bubble` subscription on the originating element is
dormant for that event rather than being reinterpreted as `Target`.
Target-only events route only `target/Target`.

C# forwards one `UiEvent` for one native event, never one action per subscribed
ancestor. It maps Unity-created internal targets to the nearest Rust-owned
logical ancestor, includes that `target_id`, and includes only the native
event-specific payload. It does not serialize the ancestor path or subscriber
IDs. `battlement-ui::route_event` uses the Rust-owned current tree and
subscriptions to return ordered `(ObjectId, UiEventPhase)` deliveries.

The C# root uses one callback per propagating event family where UI Toolkit
permits root observation. Non-propagating events such as geometry changes use
per-element callbacks with registration reference counts. If no target or
ancestor is subscribed, C# does not allocate a protocol payload or call Rust.

There is one explicit exception: setting `Tab.closeable = true` installs the
mandatory `TabCloseRequested` callback whether or not it appears in `events`,
because Rust must decide whether destruction occurs. All other semantic
events, including `ValueCommitted`, `ScrollSettled`, and `TabReordered`, need a
`Target` subscription. A controlled control with no commit subscription still
restores its committed value when local interaction ends and sends no traffic.
The fake applies the same rule and the text example subscribes explicitly.

Rust cannot prevent a native default action, stop propagation, or stop
immediate propagation. `UiEvent` therefore contains no cancellation result and
`Response` remains unchanged.

### Complete event matrix

All events carry `target_id`; pointer and keyboard payloads omit default
pointer ID, button, click count, and modifier fields.

| `UiEventKind` / body | Payload | Native source and propagation | Default forwarding |
|---|---|---|---|
| `PointerDown`, `PointerUp` | pointer ID, panel position/delta, changed button, pressed-button mask, pressure, click count, modifiers, pointer type | Corresponding `Pointer*Event`; native trickle/bubble | Only when subscribed |
| `PointerMove` | pointer ID, panel position/delta, optional changed button, pressed-button mask, pressure, click count, modifiers, pointer type | `PointerMoveEvent`; native trickle/bubble | Only when subscribed; may be high frequency |
| `PointerCancel` | pointer ID, panel position/delta, pressed-button mask, pressure, modifiers, pointer type | `PointerCancelEvent`; native trickle/bubble | Only when subscribed |
| `Click` | `Pointer` with pointer details, `Navigation` with no pointer fields, or `Repeat` with no pointer fields | `ClickEvent` for ordinary pointer activation; `NavigationSubmitEvent` for Button navigation; fixed callback for each RepeatButton invocation | Discrete event |
| `PointerEnter`, `PointerLeave` | pointer ID, panel position, pointer type | `PointerEnterEvent`, `PointerLeaveEvent`; target semantics | Only subscribed target |
| `PointerOver`, `PointerOut` | pointer ID, panel position, related target ID when Rust-owned | `PointerOverEvent`, `PointerOutEvent`; trickle/bubble | Only when subscribed |
| `Wheel` | panel position, finite x/y/z delta, modifiers | `WheelEvent`; trickle/bubble | Only when subscribed |
| `KeyDown`, `KeyUp` | **W3C physical key code** (the standardized hardware-key location name) where mapped, character, modifiers, repeat | `KeyDownEvent`, `KeyUpEvent`; trickle/bubble | Only when subscribed |
| `NavigationMove` | direction and finite move vector | `NavigationMoveEvent`; trickle/bubble | Discrete |
| `NavigationSubmit`, `NavigationCancel` | no extra payload | Corresponding navigation event | Discrete |
| `FocusIn`, `FocusOut` | related target ID when Rust-owned, direction | `FocusInEvent`, `FocusOutEvent`; trickle/bubble | Only when subscribed |
| `Focus`, `Blur` | related target ID when Rust-owned, direction | `FocusEvent`, `BlurEvent`; target semantics | Only subscribed target |
| `Input` | current local string value | `InputEvent` on `TextField` | Explicit opt-in only |
| `ValueChanging` | typed intermediate proposed value | Slider or scroller native change while dragging | Explicit high-frequency opt-in only |
| `ValueCommitted` | `UiValue` old committed and proposed values | Fixed Battlement control adapter | Final-only when subscribed |
| `SelectionChanged` | cursor and selection **UTF-16 code-unit indices** used by C# strings | Text selection callbacks | Explicit opt-in |
| `ScrollSettled` | finite x/y offset | Battlement's exact 100-millisecond idle-and-no-capture rule below | Final-only when subscribed; never every frame |
| `ScrollChanged` | finite x/y offset | `ScrollView.scrollOffset` change | Explicit high-frequency opt-in only |
| `GeometryChanged` | old and new finite `Rect` | `GeometryChangedEvent`; target-only | Explicit subscription |
| `AttachToPanel`, `DetachFromPanel` | no extra payload | Corresponding panel events; target-only | Explicit subscription |
| `PointerCapture`, `PointerCaptureOut` | pointer ID | Corresponding capture events | Explicit subscription |
| `TransitionStart`, `TransitionEnd`, `TransitionCancel` | nonempty supported style-property list and finite elapsed milliseconds | Corresponding transition event | Explicit subscription |
| `LinkEnter`, `LinkLeave`, `LinkDown`, `LinkUp` | link ID, link text, panel position, button where applicable | Corresponding rich-text link events | Explicit subscription |
| `TabCloseRequested` | tab ID and containing TabView ID | Fixed package `closing` callback | Always for a closeable tab |
| `TabReordered` | old and proposed indices | `TabView` reorder callback | Final-only when subscribed; no live mode |

### Exact event wire shapes

`UiEvent` has exactly `target_id: ObjectId` and `body: UiEventBody`. The body is
an externally tagged enum using the case names in the first column below. Rust
and C# use the following exact payload records; `Point` and `Vector` are two
finite `f32` values in panel pixels. Panel origin is the upper-left, positive x
points right, and positive y points down.

| Body cases | Rust payload and exact members |
|---|---|
| `PointerDown`, `PointerUp` | `PointerButtonEvent { pointer_id: i32, position: Point, delta: Vector, button: PointerButton, buttons: u32, pressure: f32, click_count: u32, modifiers: KeyModifiers, pointer_type: PointerType }` |
| `PointerMove` | `PointerMoveEvent { pointer_id: i32, position: Point, delta: Vector, changed_button: Option<PointerButton>, buttons: u32, pressure: f32, click_count: u32, modifiers: KeyModifiers, pointer_type: PointerType }` |
| `PointerCancel` | `PointerCancelEvent { pointer_id: i32, position: Point, delta: Vector, buttons: u32, pressure: f32, modifiers: KeyModifiers, pointer_type: PointerType }` |
| `Click` | `ClickEvent::Pointer { pointer_id: i32, position: Point, button: PointerButton, click_count: u32, modifiers: KeyModifiers }`, `Navigation`, or `Repeat` |
| `PointerEnter`, `PointerLeave` | `PointerBoundaryEvent { pointer_id: i32, position: Point, pointer_type: PointerType }` |
| `PointerOver`, `PointerOut` | `PointerCrossingEvent { pointer_id: i32, position: Point, related_target_id: Option<ObjectId>, pointer_type: PointerType }` |
| `Wheel` | `WheelEvent { position: Point, delta: Vector3, modifiers: KeyModifiers }` |
| `KeyDown`, `KeyUp` | `KeyEvent { physical_key: Option<PhysicalKey>, text: String, modifiers: KeyModifiers, repeat: bool }` |
| `NavigationMove` | `NavigationMoveEvent { direction: NavigationDirection, move: Vector }` |
| `NavigationSubmit`, `NavigationCancel` | unit payload encoded as `{}` |
| `FocusIn`, `FocusOut`, `Focus`, `Blur` | `FocusEvent { related_target_id: Option<ObjectId>, direction: FocusDirection }` |
| `Input` | `InputEvent { local_text: String }` |
| `ValueChanging` | `ValueChangingEvent { proposed: UiValue }` |
| `ValueCommitted` | `ValueCommitEvent { previous: UiValue, proposed: UiValue }` |
| `SelectionChanged` | `SelectionEvent { cursor_index: u32, selection_index: u32 }` |
| `ScrollSettled` | `ScrollEvent { offset: Vector }` |
| `ScrollChanged` | `ScrollEvent { offset: Vector }` |
| `GeometryChanged` | `GeometryEvent { previous: Rect, current: Rect }` |
| `AttachToPanel`, `DetachFromPanel` | unit payload encoded as `{}` |
| `PointerCapture`, `PointerCaptureOut` | `PointerCaptureEvent { pointer_id: i32 }` |
| `TransitionStart`, `TransitionEnd`, `TransitionCancel` | `TransitionEvent { properties: Vec<TransitionProperty>, elapsed_ms: f32 }` |
| `LinkEnter`, `LinkLeave`, `LinkDown`, `LinkUp` | `LinkEvent { link_id: String, link_text: String, pointer_id: i32, position: Point, button: Option<PointerButton> }` |
| `TabCloseRequested` | `TabCloseEvent { tab_id: ObjectId, tab_view_id: ObjectId }` |
| `TabReordered` | `TabReorderEvent { previous_index: u32, proposed_index: u32 }` |

The shared `PointerButton` cases are `Left`, `Middle`, `Right`, and
`Other(i32)`. `Other` preserves a nonnegative native button index greater than
two and encodes as `{ "Other": value }`; core world input emits only the first
three cases. The shared `PointerType` cases are `Mouse`, `Touch`, `Pen`, and
`Unknown`.
The shared `KeyModifier` cases, in canonical `KeyModifiers` order, are `Alt`,
`Control`, `Command`, `Shift`, `CapsLock`, `Numeric`, and `FunctionKey`.
`NavigationDirection` is `None`, `Left`, `Up`, `Right`,
`Down`, `Next`, or `Previous`. `FocusDirection` is `None`, `Unspecified`,
`Left`, `Right`, or `Other(i32)`; `Other` preserves a project focus ring's
nonstandard public direction value without serializing a Unity object.
The shared `PhysicalKey` contains the closed W3C code names supported by
Unity's native `KeyCode`; an unmapped UI key is `None` rather than an arbitrary
string. Core global-key selection and core key actions use this same type and
therefore always contain a mapped value.

`UiValue` has exact cases `String(String)`, `Bool(bool)`, `I32(i32)`,
`F32(f32)`, `F32Range { min: f32, max: f32 }`, `Index(Option<u32>)`,
`Choice { index: Option<u32>, value: Option<String> }`, and
`Indices(Vec<u32>)`. Radio groups and TabView use `Index`; DropdownField uses
`Choice`; ToggleButtonGroup uses `Indices`. A dropdown `Choice` is either two
matching `Some` values or two `None` values. Floats are finite; indices are
in-range; index lists are unique and sorted.

Payload omission is exact: omit `pointer_id` when zero; omit `button` when
`Left` in `PointerButtonEvent` and `ClickEvent::Pointer`; omit
`changed_button` when `None`; omit `buttons` and `pressure` when zero; omit `click_count` when one in
`PointerButtonEvent`/`ClickEvent::Pointer` and when zero in `PointerMoveEvent`; omit
`pointer_type` when `Mouse`, empty modifiers, `repeat` when false,
`related_target_id`/`physical_key`/optional link button when
`None`, and empty key text. Positions, deltas, committed values, selection
indices, geometry, transition fields, and tab fields are never omitted. An
empty transition property list is invalid.

A minimal pointer-enter event is therefore:

```json
{
  "target_id": "22222222-2222-4222-8222-222222222222",
  "body": { "PointerEnter": { "position": { "x": 24.0, "y": 12.0 } } }
}
```

A full dropdown proposal is:

```json
{
  "target_id": "33333333-3333-4333-8333-333333333333",
  "body": {
    "ValueCommitted": {
      "previous": { "Choice": { "index": 0, "value": "Low" } },
      "proposed": { "Choice": { "index": 1, "value": "High" } }
    }
  }
}
```

For `Button` and `RepeatButton`, `Click` means activation rather than only a
native `ClickEvent`. Button observes `ClickEvent` and `NavigationSubmitEvent`;
RepeatButton uses its required fixed forwarding callback for each repeated
invocation. For navigation targeting a Button, C# examines the complete
logical target/ancestor path before encoding: if any applicable `Click`
subscription exists, it emits one `Click::Navigation` and every
`NavigationSubmit` subscription on that path is dormant; otherwise, it emits
one `NavigationSubmit` if that kind has an applicable subscription. With
neither, it emits nothing. This route-wide `Click` precedence applies even
when the two subscription kinds are on different elements. Pointer, keyboard,
and gamepad activation therefore never double-submit.

The root `ClickEvent` bridge ignores a target mapped to `RepeatButton`; the
fixed RepeatButton callback is its sole `Click` source. One press emits its
initial `Click::Repeat`, a hold emits only the timer repetitions, and release
does not add a root-observed click. This exception prevents Unity's pointer-up
click detector from duplicating the repeat callback.

Unity's link-out event lacks link ID and text. The bridge caches the most
recent link-enter identity per `(ObjectId, pointer_id)` and uses it to populate
`LinkLeave`; it removes the entry on leave, detach, recursive destruction,
snapshot replacement, input disable, and session teardown. A leave without a
matching cache entry is not forwarded because it cannot satisfy the typed
payload contract.

Duplicate mouse compatibility events are not forwarded when the corresponding
pointer event exists. Editor drag/drop, command validation/execution,
contextual-menu population, tooltip population, and custom-style callbacks are
excluded.

### Controlled interaction timing

Discrete controls such as Button, Toggle, radio groups, DropdownField, and Tab
selection submit synchronously. Before submitting a proposed value, the
adapter captures the committed value. After Rust returns, it restores that
value through `SetValueWithoutNotify`; only returned commands establish the new
committed value. There is no visible rollback because both occur before the
next repaint.

Text controls retain a local draft. Enter and focus loss submit a
commit. Escape restores the committed value without an action. A subscribed
`Input` event sends per-keystroke proposals but does not change committed state
unless Rust returns an update.

Slider, `SliderInt`, `MinMaxSlider`, and `Scroller` keep native local state
during pointer capture. Release submits one final proposal,
restores committed state, and applies Rust's response. A live-value subscription
to `ValueChanging` opts into native change frequency. `ScrollView` owns scroll
offset and inertia locally, emits `ScrollSettled` only when subscribed, and
uses the separate opt-in `ScrollChanged` event for native change frequency.

Scroll settlement uses one exact clock-driven rule. The bridge observes
`scrollOffset` with a command-origin suppression counter and Unity's monotonic
unscaled clock. Every user-originated offset change records the latest value
and sets a deadline to 100 milliseconds after that change. It emits nothing
while the ScrollView, its viewport, either scroller, or their descendants hold
pointer capture. At the first Update at or after the deadline with no such
capture—and only if no offset change occurred during the preceding 100
milliseconds—it emits one `ScrollSettled` carrying the latest offset and
disarms. Inertia naturally postpones settlement because every inertial offset
change resets the deadline. A gesture that never changes offset emits nothing;
a later change arms a new deadline. Rust writes, rollback/restoration, input
disable, detach, destruction, and snapshot replacement cancel the pending
deadline without an event. `ScrollChanged`, when subscribed, still emits for
each user-originated offset change independently of this timer.

`battlement-ui-fake` uses the same 100-millisecond algorithm and a manual
monotonic clock. Gesture helpers update capture and offset; tests advance the
clock explicitly, so production and fake settle at the same observable
boundary without sleeping.

A closeable Tab installs a `closing` delegate that always returns false. It
synchronously emits `TabCloseRequested`. Rust accepts by returning
`destroy_visual_element` for that Tab; the deferred mutation removes it after
dispatch. Doing nothing rejects the close. This preserves Rust authority
without adding event cancellation to `Response`.

The existing snapshot `input_disabled` flag gates world and UI input together.
On a transition to disabled, the UI manager first suppresses forwarding, then
releases all pointer captures, blurs the focused element, restores every draft,
drag, and scroll value to its committed state without notification,
and clears link caches. Those cleanup operations emit no Rust actions. UI
commands and snapshot application remain enabled. Session teardown and
snapshot replacement perform the same cleanup before identities disappear;
snapshot replacement then installs only the new snapshot's committed state.

### Synchronous response and deferred mutation

Native and localhost HTTP submission remain synchronous and use the existing
`ClientMessage` and `Response` pipeline. A **dispatch gate** is the Runtime
boundary that temporarily holds a complete response while UI Toolkit is
traversing an event. No UI-specific **C ABI (C application
binary interface)**, secondary socket, callback pointer, or asynchronous event
queue is added.

While a UI Toolkit event callback is active:

1. C# constructs at most one subscribed `UiEvent` and calls Rust synchronously.
2. Runtime decodes the response and validates transport/session identity
   immediately, but does not pass any message in that response to
   `BattlementBatchScheduler.Schedule` or any snapshot/command executor.
3. It enqueues the entire decoded response in arrival order behind the
   UI-dispatch gate. Deferring only UI commands is forbidden because eager
   execution of a world command or snapshot could still mutate state during
   native propagation.
4. The callback and native event propagation finish.
5. A **late player-loop flush** (a package callback near the end of Unity's
   per-frame update sequence) feeds each queued response into the ordinary
   response admission and batch scheduler exactly once, before UI Toolkit's
   next layout and repaint. Queued messages retain response, batch, and group
   order.

The gate sits in `Battlement.Runtime` immediately before the existing response
dispatcher; it is not an executor inside `Battlement.UI`. The flush component
has an explicit Unity execution order after the EventSystem and before
rendering. Immediately runnable UI commands in the response from normal
pointer, keyboard, navigation, and control input execute in that flush and are
visible in the first repaint. A delayed or dependency-blocked batch, a command
behind earlier blocking work, and an authoritative snapshot retain their
ordinary scheduler semantics and have no same-frame promise. If an event
originates after the flush, such as a late geometry event during rendering,
its response applies at the next safe flush. The first-render hover test uses
one ungrouped, dependency-free UI update so it exercises the guaranteed path.

The synchronous call runs on Unity's main thread. Game code must keep handlers
within its frame budget. Battlement avoids routine high-frequency calls through
draft, drag, and scroll defaults rather than moving application decisions into
C#.

## JSON and payload-size contract

UI uses the existing externally tagged, descriptive JSON convention. It does
not use abbreviated keys, numeric type IDs, a second encoding, or a runtime
schema registry. A minimal create body is shaped as follows:

```json
{
  "VisualElementCreate": {
    "parent_id": "11111111-1111-4111-8111-111111111111",
    "element": {
      "Button": {
        "object_id": "22222222-2222-4222-8222-222222222222",
        "text": "Continue"
      }
    }
  }
}
```

An update that intentionally restores create-default values and clears one
inline style is shaped as follows; none of the present values may be omitted:

```json
{
  "VisualElementUpdate": {
    "patch": {
      "Button": {
        "object_id": "22222222-2222-4222-8222-222222222222",
        "common": { "enabled": false },
        "style": { "background_color": null },
        "properties": { "text": "" }
      }
    }
  }
}
```

A part-style update uses the owning element's private part tag and the same
minimal `Style` patch:

```json
{
  "VisualElementUpdate": {
    "patch": {
      "ScrollView": {
        "object_id": "33333333-3333-4333-8333-333333333333",
        "parts": [
          {
            "part": "VerticalDragger",
            "style": { "background_color": { "r": 0.36, "g": 0.41, "b": 0.49, "a": 1.0 } }
          }
        ]
      }
    }
  }
}
```

The following omission rules are mandatory:

- Create and snapshot records omit default Booleans, numeric values, enum
  values, empty strings where the class permits them, empty vectors, empty
  subscription sets, empty styles, empty part-style lists, and append
  placement.
- A create subtree carries each ID, type tag, protocol-required value, and
  nondefault property exactly once.
- Updates never resend full element state or unchanged fields. A present patch
  value is never omitted for equaling a create default: `false`, zero, an empty
  string/list, and a default enum case remain on the wire. Present `null` means
  inline-style clear, `None`, or whole-part-style removal according to the
  field's declared patch type.
- Part updates contain only changed style fields for the named part; they never
  resend the outer element style or another part.
- Subscription updates send additions and removals rather than a replacement
  copy.
- Event payloads contain no propagation path, subscriber list, resolved style,
  layout snapshot, or unchanged control state.
- Addressable properties send typed addresses, never asset metadata or bytes.

Serialization goldens cover each distinct wire shape and the omission rules
that Rust's type system cannot enforce. They include a minimal create, a
representative populated create subtree, an aggregate patch containing explicit
default values, outer and part-style clears, and representative event payloads.
Tagged-union cases are covered where their tags or dispatch arms are written by
hand. Declarative fields that share the same serialization mechanism do not
each receive a separate golden. A focused assertion verifies that a
one-property patch contains no unrelated state; exact byte counts are not part
of the contract.

The existing 16 MiB message limit remains the outer cap. One snapshot may
contain at most 100,000 combined GameObjects, document roots, and declared
visual elements, with no hierarchy deeper than 256. Every string and address
is at most 65,536 UTF-8 bytes. A command group remains capped at 4,096 commands.
Recursive decoding and validation enforce depth before native construction.

## Unity package and assembly architecture

UI remains in the mandatory `com.battlement.client` Unity package. It is not a
second **UPM (Unity Package Manager)** package. The package adds the
`Battlement.UI` runtime assembly and a package runtime theme asset.

The implementation introduces the lower `Battlement.Protocol` assembly and
moves plain protocol mirrors into it without changing their JSON.

| Assembly | Contains | References |
|---|---|---|
| `Battlement.Protocol` | IDs, addresses, values, messages, command/action unions, world and UI wire records | Newtonsoft JSON annotations and Unity scalar assemblies only |
| `Battlement.UI` | UI identity entries, document and element managers, builders/executors, event adapters, controlled values, and leases | `Battlement.Protocol`, Addressables, Input System, UI Toolkit |
| `Battlement.Runtime` | Existing runner, world, scheduler, transport, assets, input, direct core dispatch, and UI-response dispatch gate | `Battlement.Protocol`, `Battlement.UI`, existing runtime dependencies |
| `Battlement.Json` | Explicit union converters and bootstrap | `Battlement.Protocol`, `Battlement.Runtime` |

UI command records live in `Battlement.Protocol` beside the rest of the closed
`CommandBody`; UI execution lives in `Battlement.UI`. `Battlement.Runtime`
matches the four UI command bodies directly and calls the UI manager. There is
no reflection-based handler, service locator, optional assembly probe, custom
command registration, or package feature flag.

### C# ownership boundaries

The UI assembly separates these responsibilities into composable classes kept
near the repository's 500-line source-file target:

| Component | Responsibility |
|---|---|
| document manager | Create declared documents and their runtime panel settings, assign roots, configure Rust-authored world-space input, and clean up sessions |
| identity index | Coordinate one global index with the world, map native elements to `ObjectId`, and reject duplicate or wrong-kind access |
| subtree factory | Validate and build detached logical trees, capture typed native parts, apply defaults/styles, acquire leases, and attach once complete |
| element executor | Validate and apply aggregate properties, outer/part styles, placement, destruction, and one-shot actions atomically |
| style converter | Exhaustive typed conversion for the 86 outer and part properties; no reflection or string property lookup |
| usage-lease set | Own per-element, per-part, and per-document leases; acquire replacements before releasing originals |
| event bridge | Maintain subscription counts, map internal targets, encode one typed event, and call the existing submit function |
| controlled-value adapters | Capture, propose, restore without notification, and accept Rust-returned values for each control family |

The dispatch gate is a `Battlement.Runtime` component beside the ordinary
response dispatcher, not a `Battlement.UI` component. The UI event bridge only
calls `IBattlementUiHost.SubmitUiEvent`; Runtime owns response queueing and the
late flush.

`Battlement.UI` also owns the host interfaces implemented by
`Battlement.Runtime`. Their required surface is:

```csharp
public interface IBattlementUiHost
{
    bool InputDisabled { get; }
    ObjectKind? FindObjectKind(ObjectId objectId);
    bool TryReserveIdentity(ObjectId objectId, ObjectKind kind);
    void ReleaseIdentity(ObjectId objectId, ObjectKind kind);
    void SubmitUiEvent(UiEvent value);
    InputCameraSelection ResolveInputCamera();
    void RegisterUiCollider(Collider value);
    void UnregisterUiCollider(Collider value);
}

public enum InputCameraMode { AutomaticMain, Explicit }

public readonly record struct InputCameraSelection(
    InputCameraMode Mode,
    Camera? Camera);
```

`SubmitUiEvent` blocks until Rust's response is decoded and safely queued, as
specified by the dispatch gate; it does not execute that response inline.
`AutomaticMain` returns the current `Camera.main`, which may be null;
`Explicit` must contain the selected camera. The mode remains distinguishable
even when the explicit camera happens to equal `Camera.main`, allowing the
generated `PanelInputConfiguration` to apply the correct rule.
`ObjectKind` distinguishes at least `GameObject`, `UiDocumentRoot`, and
`VisualElement`. Runtime owns the one global reservation table, while the UI
identity index owns the native `VisualElement` handles. A UI lookup first asks
`FindObjectKind`; a non-UI result produces the same wrong-kind failure as a
world lookup of a UI ID. The Runtime runner constructs one UI manager after
the asset store and global registry, passes itself as `IBattlementUiHost` plus
the asset lookup/lease implementation, and destroys the UI manager before
those dependencies at shutdown. UI never calls a Runtime static singleton.

Snapshot application validates world and UI identities, hierarchy, document
rules, and asset declarations before mutation. It prepares all assets and
scenes through existing phases, then replaces world and UI state in one session
transition. If a Unity call throws during application, the snapshot fails with
`UnityException`, the loading failure surface remains, and the client does not
continue with a partially admitted session.

## Validation and failure behavior

Rust's `Validate` implementations reject malformed snapshots and commands
before serialization when called. Unity repeats every invariant needed to
protect native state because remote JSON is untrusted.

Validation covers:

- Nonzero UUIDs and uniqueness across the global object namespace.
- Root ownership, no nested documents, no cross-document element parenting,
  acyclic logical trees, depth, child count, and child-class rules.
- Correct live element type for every typed patch and action.
- Unique classes and subscriptions; nonoverlapping subscription add/remove
  sets; valid propagation phase for the event.
- Unique private part keys, valid indexed parts, and conditional parts present in
  the aggregate's final control state.
- Finite numbers, style-specific ranges, ordered ranges, indices, text
  selection bounds, transition-list values, gradient stops, and scroll values.
- Prepared asset presence and exact Unity type before lease acquisition.
- Exact one-to-one matching between declared UI-document GameObjects and
  flattened `UiDocument` entries.
- World-space camera availability when interactive world-space documents are
  active, valid Rust `PanelInputConfiguration` values, and an active
  EventSystem.

The existing `CoreErrorCode` mapping is:

| Failure | Code |
|---|---|
| malformed union, scalar, patch tri-state, or JSON number | `InvalidEncoding` |
| count, depth, string, button-group, or message limit | `LimitExceeded` |
| duplicate global ID | `DuplicateId` |
| missing target ID | `UnknownObject` |
| existing target of the wrong runtime kind | `ComponentMissing` |
| invalid property, range, child class, selection, root rule, or action state | `InvalidProperty` or `InvalidHierarchy` as applicable |
| unknown, unprepared, wrong-type, or leased asset | existing asset-specific code |
| thrown Unity API | `UnityException` |

Command failure follows existing batch scheduling. A failed create leaves no
attached element or lease. A failed update retains old protocol state and
leases. A failed destroy leaves the subtree alive. A failed deferred event
response is reported through the existing batch/operation failure path; it does
not retroactively cancel the native event.

## `battlement-ui-fake`

`battlement-ui-fake` owns an in-memory `UiWorld` indexed by `ObjectId`. It
validates and executes `Vec<UiDocument>` replacement and the create, update,
destroy, and action payload types defined in `battlement-ui`. The outer
`battlement-fake` dispatcher unwraps the four UI `CommandBody` cases. `UiWorld`
stores recursive order, common and type-specific state, outer and typed-part
inline style values, subscriptions, prepared-asset references, focus, pointer
capture, controlled values, and an execution journal.

`battlement-fake::FakeClient` composes the UI world with the existing world and
engine driver. Its `ui()` facade supplies typed helpers for pointer enter/leave,
click, focus/blur, text input and commit, toggle/radio/dropdown selection,
slider drag completion, scroll settlement, tab selection/close/reorder, pointer
capture, and link interaction. A helper verifies the target exists, is the
correct class, is enabled, and has the required target or ancestor subscription.
It then submits exactly one raw `ActionBody::VisualElement(UiEvent)` to the
engine synchronously—the same action boundary used by production—and applies
returned commands immediately. It does not expand routing into multiple engine
actions. Application code may call `battlement_ui::route_event` on that one
event to enumerate subscribed trickle, target, and bubble deliveries.

The fake implements the same sole implicit closeable-Tab subscription and the
same `input_disabled`, teardown, and snapshot-replacement cleanup. Disabled UI
gestures produce no action; disabling restores drafts and drags, releases fake
captures and focus, and clears cached link identity. These behaviors are
observable through public fake state and its action/command journal.

The fake panics on invalid setup or operations, matching the existing fake
failure policy. It does not serialize JSON, load Unity assets, calculate flex
layout, resolve inherited styles, measure text, render pixels, hit-test, model
world rays, simulate inertia, or simulate frames. A fake asset catalog records
typed availability; usage references are counted so removal checks match the
production semantic result.

## Automated validation plan

### Rust contract tests

Rust tests protect behavior and wire semantics that are not already guaranteed
by the compiler. They do not restate every struct field, builder method, or
typed conversion in test code.

- A small set of compile-time examples covers representative builders,
  typestate completion, and the child restrictions whose enforcement is not
  evident from ordinary type checking. Generated or repetitive builder
  surfaces are checked by compilation rather than one test per method.
- Constructor tests cover generated versus explicit IDs and access to an ID
  before consuming a builder. One element builder and `UiDocument` are enough
  unless another constructor has distinct logic.
- Serialization goldens cover distinct record shapes, handwritten tagged-union
  cases, and the unusual omission rules: explicit `false`, zero, empty values,
  optional `None`, and style `null` must remain present in patches while absent
  peers remain absent. Representative fields cover shared style, tuple, scalar,
  and address-conversion machinery; there is no golden for every element or
  style field.
- Catalog metadata or generated code receives a structural consistency test
  for unique tags, keys, and supported mappings. This replaces compile-fail and
  serialization tests whose only purpose would be to enumerate the public
  surface a second time.
- Table-driven validation tests cover each independent invariant and its useful
  boundaries, including identity and hierarchy, aggregate part validity,
  numeric limits, asset references, and document matching. Fields governed by
  the same validator share representative cases.
- Routing and state-machine tests cover event phase ordering, subscription
  changes, one-action submission, controlled-value acceptance and restoration,
  input-disable cleanup, and the special closeable-Tab behavior. Add a case per
  distinct state machine, not per control class that delegates to it.
- Fake-client tests stay black-box and cover one representative of each
  behavior family plus any control with custom behavior. They assert public
  state and journaled commands rather than duplicating serialization tests.
- Payload tests verify the protocol's configured limits and that a minimal
  patch contains no unrelated state. They do not freeze exact byte counts.

### Unity EditMode and protocol tests

- Rust fixture JSON round-trips through C# for each handwritten union dispatch
  case and for representative scalar, enum, omission, and clear encodings.
- A parameterized catalog test verifies that each supported element selects its
  declared Unity class. Property conversion tests cover each distinct
  conversion or setter behavior, not every ordinary assignment.
- Style and private-part catalogs use one authoritative mapping where possible.
  Structural tests verify complete and unique mappings; representative tests
  cover each style conversion family and part lookup strategy. Missing and
  ambiguous audited lookups must fail rather than selecting another descendant.
- Detached subtree failure attaches nothing and releases all acquired leases.
- Aggregate patch failure restores previous state, hierarchy, and leases.
- Recursive destroy removes identities, callbacks, captures, and leases.
- Each distinct controlled-value adapter is tested for proposal, acceptance,
  restoration, and suppression of command-originated echo. Controls that share
  an adapter do not duplicate that suite.
- TabView's scoped command-origin guard prevents selection, reorder, insertion,
  and removal echoes despite the absence of `SetValueWithoutNotify`.
- Native internal control targets map to the correct nearest Rust ID and one
  native event causes one submission even with ancestor subscriptions.
- RepeatButton press, hold, and release counts come only from its fixed
  callback; the root bridge contributes no extra pointer-up click.
- Text draft, Enter/focus commit, Escape restore, slider release, settled
  scroll, tab veto/destroy, and tab reorder behave as specified.
- Clock-driven scroll tests cover the settlement boundary, capture or continued
  movement postponing settlement, and cleanup. The fake uses the same scenario
  against its manual clock.
- Link-leave caching handles multiple pointers, detach, destruction, input
  disable, and an unmatched native leave.
- Document lifecycle tests cover replacement and teardown while proving that
  project-authored documents are ignored.
- Screen and world panels receive input; target-texture panels render without
  claiming automatic pointer input.
- World input tests cover representative valid configuration, validation
  failure, camera selection, EventSystem requirements, and package cleanup.
- World document colliders never emit a duplicate Battlement GameObject pointer
  action.
- Session teardown and snapshot replacement destroy Battlement-created
  documents and release all leases without touching project-authored roots.

### Integration, performance, and CI

A native integration fixture must move a pointer onto a subscribed Button,
have Rust return one background-color patch, capture the first rendered frame
after entry, and show the new color in that frame. The same fixture destroys
the event target from a click response and proves UI Toolkit completes the
originating dispatch without an exception.

The integration fixture records whether Battlement adds an extra frame before
normal pointer feedback; component-level timing breakdowns are diagnostic, not
required pass/fail tests without an established performance budget. A
representative unsubscribed high-frequency event and one controlled-value event
verify that opt-in traffic suppression works through the complete boundary.

Run `cargo test --workspace`, Unity EditMode tests, protocol fixture tests, and
the repository `./scripts/ci.py` entrypoint. Intended changes must be staged
before the final CI run so its metadata refresh succeeds.

## Implementation order

1. Extract `battlement-types` and preserve `battlement` reexports.
2. Add `battlement-ui` values, builders, styles, events, validation, routing,
   assets, command bodies, `ActionBody`, and snapshot integration.
3. Add representative serialization, validation, routing, and protocol-limit
   tests.
4. Split the C# protocol assembly without changing existing JSON, then add UI
   mirrors and converter cases.
5. Implement document/root and global identity coordination, asset leases,
   detached construction, styles, patches, and actions.
6. Implement typed event bridges, controlled-value adapters, and the safe
   same-turn dispatch gate.
7. Add screen, target-texture, and world-space integration tests and assets.
8. Add `battlement-ui-fake`, compose it into `battlement-fake`, and complete
   black-box engine tests.
9. Run the automated suites and the risk-based manual smoke scenarios, then
   update the canonical design documents in the same implementation series.

Each step must leave the workspace compiling and its public contracts tested.
There is no compatibility shim, protocol version negotiation, optional UI
feature, or fallback custom-command route.

## Manual QA

Use a Unity 6000.5.8f1 player with native transport and a Rust fixture engine.
Run the scenarios affected by a change; run the complete smoke set for the
initial feature milestone and before release. Capture screenshots, video, and
fixture logs only for the distinct visible or timing risks under review rather
than for every scenario.

1. **Rendering smoke.** Render the examples-first screen-space document with a
   representative mix of text, image, layout, outer style, private-part style,
   and prepared assets. Resize once, apply and clear a representative style,
   and confirm an unrelated project-authored document remains untouched.
2. **Interaction smoke.** Exercise a Button, one text control, one selection
   control, one drag control, ScrollView settlement, and a closeable Tab.
   Confirm one Rust action per subscribed native event, local drafts restore on
   rejection, Rust writes do not echo, and hover feedback reaches the first
   eligible rendered frame.
3. **Mutation and lifecycle.** During dispatch, update one element and destroy
   another. Replace the snapshot while focus, capture, or a draft is active,
   then end the session. Confirm no dispatch exception, stale local state,
   identity, callback, or asset lease survives.
4. **Panel modes.** Render one target-texture document and one transformed
   world-space document. Confirm their intended input behavior, camera
   selection, collider tracking, absence of duplicate GameObject actions, and
   generated-component cleanup.
5. **Focused edge cases.** When changing routing, input disable, TabView command
   guards, rich-link caching, transition callbacks, or a control-specific
   adapter, run the corresponding focused scenario from this design. These are
   regression checks for touched behavior, not a mandatory matrix for unrelated
   changes.
