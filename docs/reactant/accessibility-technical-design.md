# Reactant Accessibility Technical Design and Phased Implementation Plan

Status: Proposed

## Normative status

The semantic model, lifecycle, ownership rules, validation, wire behavior,
fallback rules, acceptance scenarios, and completion criteria are normative.
Rust and C# snippets are illustrative syntax; their type boundaries and data
ownership are normative, while final type and method names may follow repository
conventions. The Unity capability matrix is required v1 coverage. The only v1
assistive-technology integration substrate is `UnityEngine.Accessibility`;
custom native plugins and WebGL DOM/ARIA projection are explicitly out of scope.

The phased plan is delivery guidance. It may be split into smaller tasks without
changing a phase's dependencies or exit criteria.

## Decision

Reactant will own a platform-neutral semantic tree that is reconciled beside the
visual tree. Rust declares accessible meaning and finite interaction policies.
Unity owns the live semantic mirror, geometry, input modality, focus scopes, and
all operations that a Unity accessibility callback must complete synchronously.
The Unity host lowers the mirror into `AccessibilityHierarchy` and
`AccessibilityNode` on platforms where Unity supports `AssistiveSupport`.

The public Rust API follows the lower-level React Aria model: state, accessible
behavior, interaction, rendering, and styling remain separate. A hook may return
semantic, focus, and interaction properties, but it never chooses a Reactant host
type, child structure, class, or style. Developers compose those properties onto
their own elements.

The semantic tree is keyed by stable logical Reactant identities. A portal can
move pixels to another `UiDocument` without moving the node in semantic or event
ancestry. Motion can retain a physical host after logical removal without keeping
it accessible. Reconnect rebuilds the same logical semantic identities even when
Unity objects are recreated.

This design intentionally does not model accessibility as DOM attributes, infer
it from UI Toolkit classes, or use focus order as a substitute for reading order.
V1 preserves semantics that Unity cannot currently publish so applications are
ready to benefit when Unity expands its accessibility surface; Reactant does not
work around those limitations by calling platform accessibility APIs directly.

## Related information

Repository designs:

- [Reactant technical design](reactant-technical-design.md)
- [Component authoring](component-authoring.md)
- [Hooks and effects](hooks-and-effects.md)
- [Reconciliation, events, and portals][reconciliation-design]
- [Refs, geometry, and floating UI][refs-design]
- [Animations and presence](animations.md)
- [Battlement UI technical design](../battlement-ui-technical-design.md)
- [Ditto technical design](../ditto-technical-design.md)
- [Ditto scenario guide](../ditto.md)

External design references:

- [React Spectrum repository](https://github.com/adobe/react-spectrum), especially
  the separation between React Stately, React Aria hooks, and rendering layers
- [React Aria `useButton`](https://react-aria.adobe.com/Button/useButton),
  [`useCheckbox`](https://react-aria.adobe.com/Checkbox/useCheckbox),
  [`useSlider`](https://react-aria.adobe.com/Slider/useSlider), and
  [`useDialog`](https://react-aria.adobe.com/Dialog/useDialog)
- [WAI-ARIA Authoring Practices][apg-patterns] for widget behavior conventions
- [APG keyboard interface guidance][apg-keyboard]
- [Accessible Name and Description Computation 1.2][accname]
- [Core Accessibility API Mappings 1.2][core-aam]
- [WAI-ARIA 1.2](https://www.w3.org/TR/wai-aria-1.2/)
- [Unity mobile accessibility][unity-mobile] and
  [`AccessibilityHierarchy`][unity-hierarchy]
- [`AccessibilityNode`][unity-node]
- [`AccessibilityRole`][unity-role] and
  [`AccessibilityState`][unity-state]
- [`AssistiveSupport`][unity-assistive]

[reconciliation-design]: reconciliation-events-and-portals.md
[refs-design]: refs-geometry-and-floating-ui.md
[apg-patterns]: https://www.w3.org/WAI/ARIA/apg/patterns/
[apg-keyboard]: https://www.w3.org/WAI/ARIA/apg/practices/keyboard-interface/
[accname]: https://www.w3.org/TR/accname-1.2/
[core-aam]: https://www.w3.org/TR/core-aam-1.2/
[unity-mobile]: https://docs.unity3d.com/6000.5/Documentation/Manual/mobile-accessibility.html
[unity-hierarchy]: https://docs.unity3d.com/6000.5/Documentation/ScriptReference/Accessibility.AccessibilityHierarchy.html
[unity-node]: https://docs.unity3d.com/6000.5/Documentation/ScriptReference/Accessibility.AccessibilityNode.html
[unity-role]: https://docs.unity3d.com/6000.5/Documentation/ScriptReference/Accessibility.AccessibilityRole.html
[unity-state]: https://docs.unity3d.com/6000.5/Documentation/ScriptReference/Accessibility.AccessibilityState.html
[unity-assistive]: https://docs.unity3d.com/6000.5/Documentation/ScriptReference/Accessibility.AssistiveSupport.html

WAI-ARIA is used here as a vocabulary and behavior reference. It is not the wire
format and it does not imply a browser accessibility tree. The v1 Unity adapter
maps that vocabulary to the smaller set of roles, states, values, and actions
that `UnityEngine.Accessibility` exposes. Reactant does not create browser DOM
nodes or invoke UIKit, Android accessibility, NSAccessibility, or UI Automation
APIs directly.

The mockups under `/Users/dthurn/Documents/mockups/` were behavioral evidence for
settings tabs, key rebinding, dialogs, sliders, announcements, focus treatments,
and exit animation. They are not a continuing dependency: the relevant behavior
is restated in the acceptance scenarios below. They are not an API, DOM, styling,
or component-structure specification.

## Goals and boundaries

The subsystem must:

- let arbitrary developer-authored visuals reuse correct semantics and behavior;
- expose roles, names, descriptions, values, states, relationships, grouping,
  reading order, headings, landmarks, and collection metadata;
- keep semantic accessibility distinct from input focus, navigation, event
  propagation, and default-action ownership;
- support keyboard, controller, touch, pointer, switch-control, and screen-reader
  activation through one declared interaction contract;
- remain correct across portals, nested overlays, virtualization, reconnects,
  localization, right-to-left layouts, and presence animation;
- produce the most plausible Unity accessibility representation available
  without reducing the canonical model to Unity's current feature set;
- detect invalid compositions before they become a partially committed Unity
  tree; and
- provide deterministic Rust, Unity, protocol, and Ditto tests while identifying
  the behavior that still requires real assistive technology.

This design does not create a Reactant component library or a visual design
system. It does not select host elements, draw focus rings, localize application
content, or make game-world content automatically accessible. It provides the
primitives with which those layers are built.

## Existing contracts that constrain the design

Reactant renders synchronously on the engine thread. It reconciles a work tree,
validates it, and commits an ordered mutation plan. Hooks are positional and
event handlers resolve through the committed logical tree. Portals preserve that
logical ancestry while changing only the physical UI Toolkit parent.

Unity receives native UI events before Rust can change their default behavior.
Rust logical capture and bubble can stop Reactant propagation, but cannot undo a
UI Toolkit default action. Controlled native controls therefore keep a local
draft, send a proposal, restore the committed value without notification, and
later apply the Rust-authoritative response.

Unity also owns the live `VisualElement` index, geometry, hit testing, capture,
focus, and deferred response gate. `ElementRef` actions are queued after structural
mutations. Reconnect preserves Rust logical identities while recreating Unity
hosts and invalidating attachment and geometry.

`AnimatePresence` deliberately retains removed hosts, hook state, handlers, and
object IDs until exit completion. Accessibility cannot inherit that physical
lifetime: an element that has logically disappeared must stop receiving
assistive-technology focus and actions before its exit animation runs.

Ditto creates a fresh engine per scenario and drives the production input path.
It can prove semantic snapshots and actions, but it cannot prove what VoiceOver
or TalkBack actually speaks through Unity's mobile integration.

These contracts lead to four rules:

1. Rust declares semantics and bounded policies; it does not issue platform API
   calls.
2. Unity applies a semantic commit and accessibility callbacks on the engine
   thread.
3. Logical Reactant ancestry is authoritative for semantics and relationships.
4. A Unity default that must happen during an input callback is owned by Unity,
   based on a policy already declared by Rust.

### Repository dependency contract

This document uses the following Reactant terms with fixed meanings:

- `ObjectId` is the stable logical identity assigned to a reconciled host. It is
  preserved while the keyed logical host survives and across reconnect, even
  though the corresponding Unity object is recreated.
- `UiDocument` is a physical UI Toolkit panel root. Moving a host between portal
  targets may change its document without changing its logical ancestry.
- the synchronous runtime call starts with a Unity event, dispatches it through
  the committed logical Rust tree, renders resulting state, and returns one
  response before the call completes;
- the safe response gate admits that response only after the current UI Toolkit
  propagation/default-action stack has unwound, but still before the next
  repaint;
- the production input path is Unity input or a Unity accessibility callback,
  then the synchronous runtime call, then safe-gate application; tests must not
  call an application handler directly; and
- attachment invalidation means a reconnect or host removal immediately makes
  cached Unity geometry and focus handles unusable until the same logical ID is
  attached in the new backend generation.

These are dependencies, not extension points. Accessibility code must preserve
their ordering and failure behavior.

## Layered public model

The public API has five independent layers.

### State

State types represent selection, toggling, ranges, disclosure, overlays, and
collections. They contain no host props and no platform concepts. Applications
may use Reactant state hooks, keep authoritative state in `G`, or adapt another
store to the same read-and-intent interfaces.

Examples include:

- `ToggleState` and `use_toggle_state`;
- `SingleSelectionState<K>` and `use_single_selection_state`;
- `MultipleSelectionState<K>` and `use_multiple_selection_state`;
- `RangeState<T>` and `use_range_state`;
- `OverlayState` and `use_overlay_state`; and
- `CollectionState<K, T>` and `use_collection_state`.

State transitions are still Rust-owned. A synchronous Unity interaction may move
focus or a local controlled draft, but the committed selected value, checked
state, open state, or range value changes only after Rust accepts the intent.

### Accessible behavior

Behavior hooks translate state and options into semantic properties and standard
actions. They understand the contract for a pattern such as a button, tab, radio,
or listbox. They do not choose markup or styling.

### Interaction

Interaction hooks declare press, hover, focus-visible, typeahead, directional
navigation, dismissal, drag, and keyboard/controller policy. Unity executes the
bounded synchronous part. Rust receives logical intent events through the normal
committed event tree.

### Rendering

Any compatible Reactant host can accept returned properties. The developer may
use a native `Button`, a styled `VisualElement`, composed text and images, or a
custom host facade. A pattern can expose multiple property bundles where its
semantics require multiple authored elements.

### Styling

Hooks expose interaction snapshots such as `is_pressed`, `is_hovered`,
`is_focus_visible`, `is_open`, and `is_dragging`. The developer maps them to
classes, inline styles, Motion, sounds, or no visual response at all.

## Common Rust types

The exact module split is an implementation detail, but the public vocabulary is
part of this design.

```rust
pub struct AccessibilityId {
    pub identity_owner: ObjectId,
    pub slot: AccessibilitySlot,
    pub incarnation: NodeIncarnation,
}

pub enum AccessibilitySlot {
    Host,
    Hook(u16),
    Keyed {
        hook: u16,
        key: SemanticKey,
    },
}

pub struct AccessibilityRef(AccessibilityId);

pub struct SemanticProps {
    pub role: SemanticRole,
    pub name: AccessibleName,
    pub description: AccessibleDescription,
    pub state: SemanticState,
    pub value: Option<SemanticValue>,
    pub relations: SemanticRelations,
    pub collection: Option<CollectionItemInfo>,
    pub collection_window: Option<CollectionWindowInfo>,
    pub visibility: SemanticVisibility,
    pub actions: ActionSet,
    pub reading_order: ReadingOrder,
    pub geometry: GeometrySource,
    pub fallback: FallbackPolicy,
    pub alias: Option<SemanticAlias>,
    pub inert: bool,
    pub content: SemanticContent,
}
```

`AccessibilitySlot::Host` identifies the semantic node attached to a host.
`Hook` slots identify unkeyed virtual nodes owned by that same logical host, such
as a slider thumb or a label with no independent `VisualElement`. The reconciler
assigns the next slot number to each semantic-slot hook in positional hook order.
Conditional hook calls are invalid under the existing hook rules. A removed hook
slot is not reassigned during that host's lifetime.

`Keyed` slots identify collection children and other reorderable virtual nodes.
`SemanticKey` is deliberately not a serialized Reactant key. Reactant keys may be
any owned `Eq + Hash + Clone + 'static` type and preserve `TypeId`, which cannot be
sent over the wire. Keyed semantic APIs therefore require the application key
type to implement this additional trait:

```rust
pub trait AccessibilityKey {
    fn accessibility_key(&self) -> SemanticKey;
}

pub struct SemanticKey {
    pub namespace: &'static str,
    pub value: SemanticKeyValue,
}

pub enum SemanticKeyValue {
    String(String),
    Signed(i64),
    Unsigned(u64),
    Uuid(Uuid),
}
```

`namespace` is an explicit stable application/domain identifier, not a Rust type
name or `TypeId`. Domain newtypes implement their own mapping; Reactant never
implicitly converts them through an underlying primitive. Within one keyed
semantic child sequence, unequal Reactant keys must map to unequal semantic keys.
A collision panics before commit and reports both application key types without
printing private values.

The `(identity_owner, hook, semantic_key)` tuple is collision-free on the wire and
is not reduced to a hash. Reordering retains the ID. Removing and later recreating
the same key creates a new logical lifetime with a new node incarnation, so the
complete structured ID changes and stale Unity accessibility callbacks cannot
reach it.

`NodeIncarnation` is a nonzero session-local counter allocated whenever a
semantic slot enters a new logical lifetime. It remains stable through reorder,
rendering through an unchanged portal target, and reconnect. Removing and
recreating the same keyed slot gets a new incarnation, so a stale callback cannot
reach the replacement. Rust retains the last counter for removed
`(identity_owner, slot)` pairs until the session ends; counters are never reused.

`ObjectId` stability follows logical host reconciliation: a surviving keyed host
keeps its ID through reorder, an unchanged portal target, and reconnect; an
unkeyed host keeps it only while the same reconciled position survives. Remount
creates a new `ObjectId`. The snapshot carries current IDs directly rather than
recomputing them. Duplicate structured IDs in one candidate tree are invalid.

`identity_owner` exists only for identity allocation and logical lifetime. A
semantic node separately carries an optional `focus_host: ObjectId` in its
resolved snapshot. Attachment checks, input focus, hit testing, and
`GeometrySource::FocusHost` use `focus_host`; they never use `identity_owner`
implicitly. A virtual node can therefore be logically owned by one host and use
another named host ref for geometry/focus.

`AccessibilityRef` is typed and session-local. It cannot be created from an
arbitrary UUID or platform identifier. Relationship-specific wrappers such as
`LabelRef`, `DescriptionRef`, `ControlRef`, and `ErrorMessageRef` prevent common
miswiring at compile time. The wire format uses the underlying structured ID.

Virtual nodes use an explicit insertion primitive rather than appearing by
implication from `SemanticProps`:

```rust
pub enum SemanticParent {
    LogicalHost,
    Node(AccessibilityRef),
}

pub struct VirtualSemanticNode<G> {
    pub parent: SemanticParent,
    pub semantic: SemanticProps,
    pub focus: FocusProps,
    pub interaction: InteractionProps<G>,
}

pub fn use_semantic_node<G>(
    node: VirtualSemanticNode<G>,
) -> AccessibilityRef;

pub struct SemanticCollectionEntry<G, K> {
    pub key: K,
    pub parent: SemanticCollectionParent<K>,
    pub node: VirtualSemanticNode<G>,
}

pub enum SemanticCollectionParent<K> {
    External(SemanticParent),
    Key(K),
}

pub fn use_semantic_collection<G, K>(
    entries: Vec<SemanticCollectionEntry<G, K>>,
) -> HashMap<K, AccessibilityRef>
where
    K: AccessibilityKey + Eq + Hash + Clone;
```

`LogicalHost` inserts below the current host's exposed host slot, or below its
nearest logical semantic ancestor when that host is transparent. `Node` must
reference a live node in the current logical subtree that was declared earlier in
the render. Cycles, forward references, and cross-root parents are invalid.

Children with the same parent follow declaration or keyed collection order;
`ReadingOrder::DirectChildren` may override that order. Reorder updates child
order without changing keyed IDs. Virtual nodes may parent other virtual nodes,
so a table, row, and cell hierarchy is expressible even when one physical host
owns all slots. Removing a parent removes every virtual descendant in the same
semantic commit.

`use_semantic_collection` is one positional hook regardless of entry count. Its
entries are parent-before-child; `Key(parent)` resolves within that hook's keyed
set. Missing parents, duplicate keys, or a cycle are invalid. Code must not call
`use_semantic_node` in a variable-length loop.

The remaining common values are:

```rust
pub enum AccessibleName {
    None,
    Text(LocalizedText),
    LabelledBy(Vec<LabelRef>),
    Contents,
}

pub enum AccessibleDescription {
    None,
    Text(LocalizedText),
    DescribedBy(Vec<DescriptionRef>),
}

pub enum SemanticValue {
    Text(LocalizedText),
    Range {
        current: f64,
        minimum: f64,
        maximum: f64,
        step: Option<f64>,
        text: Option<LocalizedText>,
    },
}

pub struct CollectionItemInfo {
    pub position: usize,
    pub set_size: CollectionSize,
    pub level: Option<usize>,
}

pub struct CollectionWindowInfo {
    pub total_size: CollectionSize,
    pub first_materialized: Option<NonZeroUsize>,
    pub last_materialized: Option<NonZeroUsize>,
    pub before: CollectionContinuation,
    pub after: CollectionContinuation,
}

pub enum CollectionContinuation {
    Unavailable,
    Available {
        neighbor_key: Option<SemanticKey>,
        action: AccessibilityActionKind,
    },
}
```

The normative supporting schema is:

```rust
pub enum SemanticVisibility {
    Exposed,
    NameSourceOnly,
    Hidden,
}

pub struct SemanticState {
    pub disabled: bool,
    pub read_only: bool,
    pub required: bool,
    pub invalid: InvalidState,
    pub busy: bool,
    pub checked: Option<CheckedState>,
    pub pressed: Option<PressedState>,
    pub selected: Option<bool>,
    pub expanded: Option<bool>,
    pub current: Option<CurrentKind>,
    pub modal: bool,
    pub multiselectable: Option<bool>,
    pub orientation: Option<Orientation>,
    pub sort: Option<SortDirection>,
}

pub struct SemanticRelations {
    pub error_message: Option<ErrorMessageRef>,
    pub controls: Vec<ControlRef>,
    pub details: Vec<DetailsRef>,
    pub flow_to: Vec<AccessibilityRef>,
    pub active_descendant: Option<AccessibilityRef>,
    pub invoker: Option<AccessibilityRef>,
}

pub struct ActionSet {
    pub standard: BTreeSet<AccessibilityActionKind>,
    pub custom: Vec<CustomAccessibilityAction>,
}

pub enum ReadingOrder {
    Natural,
    DirectChildren(Vec<AccessibilityRef>),
}

pub enum GeometrySource {
    FocusHost,
    Element(ElementRef),
    Union(Vec<ElementRef>),
    None,
}

pub enum FallbackPolicy {
    Inherit,
    AllowAll,
    Forbid(BTreeSet<FallbackClass>),
}

pub struct SemanticContent {
    pub text: Option<LocalizedText>,
    pub image: Option<ImagePurpose>,
    pub heading_level: Option<HeadingLevel>,
    pub live_region: Option<LiveRegionPolicy>,
    pub table: Option<TableInfo>,
    pub cell: Option<CellInfo>,
}

pub struct LiveRegionPolicy {
    pub politeness: Politeness,
    pub atomic: bool,
    pub relevance: LiveRelevance,
    pub announce_initial: bool,
}

pub struct TableInfo {
    pub row_count: CollectionSize,
    pub column_count: CollectionSize,
}

pub struct CellInfo {
    pub row: usize,
    pub column: usize,
    pub row_span: NonZeroUsize,
    pub column_span: NonZeroUsize,
    pub row_headers: Vec<CellRef>,
    pub column_headers: Vec<CellRef>,
}
```

`AccessibleName::LabelledBy` is the only authored labelled-by source, and
`AccessibleDescription::DescribedBy` is the only authored described-by source.
Projection copies those same references into `ResolvedSemanticRelations` for
canonical inspection and future lowering. Unity v1 resolves their text into
`label` or `hint` rather than publishing a relationship. Text/contents name or
description sources produce no such relation. There are no duplicate relation
fields that can disagree with name computation.

All `SemanticState` booleans default to false; all options default to `None`.
Relations and actions default empty, reading order defaults natural, geometry
defaults to the focus host, fallback defaults to `Inherit`, alias defaults absent,
inert defaults false, and content fields default absent. The semantic root must
choose `AllowAll` or `Forbid`; descendants inherit from the nearest explicit
ancestor policy. `CollectionSize` is `Known(NonZeroUsize)` or `Unknown`.
Position, level, row, column, and heading level are one-based.

A collection window exists only on a collection container. Its first and last
positions are both absent for an empty window or both present with first no later
than last. Every materialized child's position lies inside that range. A known
total contains the range. A continuation action is exactly scroll backward for
`before` or scroll forward for `after` and must have a routed proposal handler.
`neighbor_key` is the known immediately adjacent application key; it may be absent
when the data source knows more content exists but has not loaded that key.

`CheckedState` and `PressedState` are `False`, `True`, or `Mixed`.
`InvalidState` is `False`, `Grammar`, `Spelling`, or `True`. `CurrentKind` is
`Page`, `Step`, `Location`, `Date`, `Time`, or `True`. `SortDirection` is
`Ascending`, `Descending`, `Other`, or `None`. `Orientation` is horizontal or
vertical.

`AccessibilityActionKind` is the closed set activate, increment, decrement, set
range value, toggle, select, expand, collapse, dismiss, show help, scroll forward,
scroll backward, page forward, page backward, first, last, set text, insert text,
delete backward, delete forward, replace selection, paste, and set selection. A
custom action is a stable action key plus a nonempty localized name. Standard and
custom action keys are unique within one node.

The wire action carries its kind-specific payload:

```rust
pub enum AccessibilityAction {
    Activate,
    Increment,
    Decrement,
    SetRangeValue { value: f64 },
    Toggle,
    Select { key: SemanticKey },
    Expand,
    Collapse,
    Dismiss,
    ShowHelp,
    Scroll { direction: ScrollDirection },
    Page { direction: ScrollDirection },
    MoveTo { boundary: CollectionBoundary },
    Text(TextEditAction),
    Custom { key: CustomActionKey },
}

pub enum TextEditAction {
    SetText { text: String },
    InsertText { text: String },
    DeleteBackward { granularity: TextGranularity },
    DeleteForward { granularity: TextGranularity },
    ReplaceSelection { text: String },
    Paste { text: String },
    SetSelection { selection: TextSelection },
}

pub struct TextSelection {
    pub anchor: usize,
    pub focus: usize,
}
```

`TextGranularity` is Unicode scalar, word, or line. Protocol selection offsets are
Unicode scalar indices; Unity converts native UTF-16 ranges at the boundary and
rejects a non-boundary or out-of-range value. Every text proposal carries the
complete previous and proposed `ControlValue::Text { text, selection }`, even
when the action payload is a delta. Rust accepts or rejects that complete value,
and the authoritative resolution returns the complete text and selection.
Read-only or disallowed edit kinds are rejected before proposal creation.
`ScrollDirection` is forward or backward and `CollectionBoundary` is first or
last. `AccessibilityAction::kind()` maps each payload variant to the closed kind
used by `ActionSet`; a payload can never be encoded as the wrong kind.

`ImagePurpose` is `Informative` or `Decorative`. A decorative image declaration
is validated but omitted from the canonical semantic forest; it cannot have a
name, relations, focus, or actions. `HeadingLevel` is one through six.
`Politeness` is polite or assertive. `LiveRelevance` independently selects text
additions, removals, and replacements. `CellRef` may target only a live row or
column header in the same table/grid. Table counts may be unknown; known cell
coordinates and spans must fit them.

`SemanticAlias` is an optional application-supplied diagnostic/test identifier.
It must be unique in the current semantic tree, is never exposed through Unity
accessibility, and is not identity. Changing it does not remount a node.

`SemanticRole` is exactly the closed set listed below:

`LocalizedText` contains a resolved string and locale identifier. Reactant does
not carry localization keys over the wire or ask Unity to translate application
content. Unity or the operating-system service may supply standard role/action
phrases, but Reactant never invents a control's name, help text, validation
message, or formatted value.

- button, link, checkbox, switch, radio, radio group, slider, tab, tab list, and
  tab panel;
- dialog, alert dialog, menu, menu bar, menu item, checkable menu item, listbox,
  option, combobox, and tooltip;
- progress indicator, status, alert, log, timer, and marquee;
- text field, search field, image, static text, heading, separator, group, and
  generic container;
- application, main, navigation, complementary, banner, content information,
  form, region, and search landmarks; and
- list, list item, table, row, column header, row header, cell, grid, tree,
  tree item, disclosure, and scroll area.

The canonical enum is intentionally richer than Unity's current
`AccessibilityRole` enum. The Unity adapter maps unsupported roles by the
fallback rules defined later; it never changes the canonical tree. Developers
should declare the most accurate Reactant role instead of pre-degrading their
application to Unity's current subset.

Role validation follows these normative families; the detailed pattern sections
add stricter behavior:

| Role family | Required data | Allowed role-specific state/action |
| --- | --- | --- |
| Button/link/disclosure | nonempty name | pressed or expanded when applicable; activate |
| Checkbox/switch | nonempty name | checked; toggle; switch forbids mixed |
| Radio | nonempty name and radio-group parent | selected/checked; select |
| Slider | nonempty name and numeric range | read-only/disabled; range actions |
| Tab | nonempty name, tab-list parent, one panel | selected; select/activate |
| Dialog/alert dialog | nonempty name | modal; dismiss when declared |
| Menu/listbox/tree item | nonempty name and matching parent | selected, checked, expanded as role permits |
| Combobox/text/search field | nonempty name and text value | required, read-only, invalid, expanded; text actions |
| Progress | nonempty name | range value or busy, never both |
| Heading | contents or explicit name and level | no actions |
| Landmark/region | role rules for naming | no control state/actions |
| Image | explicit name or decorative marker | no actions unless separately composed as a control |
| Table/grid/tree/list | structural name when pattern requires | collection state and matching child roles |
| Status/alert/log/timer/marquee | text and live-region policy | no input focus or control actions |
| Tooltip/static text/separator/group | role-appropriate text/orientation | no control actions |

A state or action not named for a family is invalid. `disabled` and `busy` are
allowed on any interactive family; `read_only`, `required`, and `invalid` are
allowed only on value-entry, range, selection, and form-control families.
Structural collection metadata is valid only on collection containers/items.
The exact child-role constraints are those in the pattern sections and are
enforced after logical transparency and portal resolution.

The JSON representation is the generated repository convention: structs use
snake-case field names, enums use internally tagged snake-case variants, IDs are
structured objects, and option/default fields are emitted explicitly in golden
fixtures. Rust and C# are generated from the same closed schema; unknown fields or
enum variants fail decoding.

## Host composition contract

Every semantic-capable UI host builder gains `.semantic(...)`. Focus-capable
hosts additionally gain `.focus_props(...)`, and hosts that can receive local
interaction gain `.interaction_props(...)`:

```rust
element
    .semantic(button.semantic)
    .focus_props(button.focus)
    .interaction_props(button.interaction)
```

Each method accepts one bundle. Bundles compose internally through typed merge
functions. Two bundles cannot both assign an exclusive role, accessible name,
focus owner, or default action. Additive descriptions, event listeners, and
actions merge in deterministic hook order. Conflicts are developer errors and
panic during render validation.

Semantic properties do not make a host focusable. Focus properties do not expose
a host to assistive technology. Interaction properties do not imply either.
Pattern hooks usually return all three because a standard control requires them,
but lower-level hooks can be composed independently.

The focus and interaction bundles use this closed public contract:

```rust
pub struct FocusProps {
    pub host: FocusHostBinding,
    pub tab_stop: TabStop,
    pub autofocus: bool,
    pub sync_from_accessibility: bool,
    pub restore_key: Option<FocusRestoreKey>,
    pub scroll_route: Option<ScrollRoute>,
}

pub enum FocusHostBinding {
    None,
    AttachedHost,
    Element(ElementRef),
}

pub enum TabStop {
    Auto,
    Excluded,
    CompositeMember,
}

pub struct InteractionProps<G> {
    pub handlers: InteractionHandlers<G>,
    pub press: Option<PressPolicy>,
    pub navigation: Option<NavigationPolicy>,
    pub range: Option<RangeInteractionPolicy>,
    pub text: Option<TextInteractionPolicy>,
    pub dismiss: Option<DismissPolicy>,
    pub input_capture: Option<InputCapturePolicy>,
}
```

All fields default to absent/false. `host` defaults to `AttachedHost` for an
interactive bundle composed on a physical host and `None` for a virtual or
structural node. A focusable virtual node must explicitly use
`Element(ElementRef)`; `AttachedHost` is invalid outside host composition.
`tab_stop` defaults to `Auto` for a pattern's primary interactive node and
`Excluded` for structural nodes. There is no positive tab-index API. An explicit
direct-child reading order is the only author mechanism that changes both reading
and ordinary Tab order.

Each policy mutation is `Install { owner, complete_policy }` or
`Remove { owner, policy_kind }`. Installing another policy of the same kind on
one owner is invalid. `InteractionHandlers<G>` is Rust-only and serializes as the
set of routed intent kinds; the finite policy data is serialized separately.

The finite policy enums are closed:

- `NavigationKind` is roving one-dimensional, active-descendant, grid, or tree.
  A policy includes current member, member eligibility, orientation, wrap,
  direction behavior, selection-follows-focus, page size, and optional typeahead.
- `DirectionBehavior` is logical or spatial. Hooks in this design use spatial for
  horizontal visual movement and logical for direction-independent hierarchy.
- `TypeaheadPolicy` contains locale, timeout milliseconds, search mode
  (prefix or contains), and ordered `(member, resolved_name)` entries.
- `PressPolicy` contains accepted devices/keys, press-on-release versus
  press-on-down, repeat policy, and whether release outside cancels.
- `RangeInteractionPolicy` contains range, step, page size, orientation,
  direction, drag geometry source, and the controlled target value kind.
- `TextInteractionPolicy` contains editable/read-only mode, multiline mode,
  selection support, password/privacy mode, and allowed native edit intents.
- `DismissPolicy` is none, Escape, controller cancel, Unity accessibility
  dismiss, or their explicit combination. Pointer backdrop dismissal is a
  separate press policy.
- `InputCapturePolicy` contains the eligible input-device/control set and reserved
  cancel controls. Rebind is its standard constructor, not a wire enum shortcut.
- `ScrollRoute` contains the scroll-container `ElementRef`, reveal alignment,
  axis, and maximum one-frame reveal distance.

`FallbackClass` is role, relation, state, value/range, collection, custom action,
live announcement, geometry, modal, or virtual continuation. Every Unity
lowering records zero or more of these classes. The record explains what Unity
could not publish; it does not remove that information from the canonical tree.

`FocusRestoreKey` is an application key scoped to the nearest focus scope. It is
stable through keyed reorder and reconnect, unique among live descendants of that
scope, and never exposed through Unity. Restoration resolves the key to the current
eligible semantic node; it does not retain a Unity object reference.

Focus scopes and presentation promotion use this closed declaration:

```rust
pub struct FocusScopeProps {
    pub owner: AccessibilityRef,
    pub kind: FocusScopeKind,
    pub trap: bool,
    pub inert_outside: bool,
    pub initial_focus: FocusTarget,
    pub restore: FocusRestoration,
    pub presentation: PresentationPolicy,
}

pub enum FocusTarget {
    None,
    Explicit(AccessibilityRef),
    RestoreKey(FocusRestoreKey),
    FirstEligible,
    ScopeOwner,
}

pub struct FocusRestoration {
    pub candidates: Vec<FocusRestoreTarget>,
}

pub enum FocusRestoreTarget {
    Invoker,
    Target(FocusTarget),
    NearestEligibleLogicalAncestor,
    FirstEligibleInParentScope,
}

pub enum PresentationPolicy {
    Inline,
    PromoteNonmodal,
    PromoteModal,
}
```

`FocusScopeKind` is application, composite, overlay, or rebind capture. Scope
owners must be nested by canonical ancestry; partially intersecting scopes are
invalid. Promoted scopes are ordered by activation commit, then canonical order
within one commit. A modal scope must set `trap` and `inert_outside`; a nonmodal
scope must not set `inert_outside`.

At most one `autofocus` declaration may exist in an activating scope. A scope's
non-`None` `initial_focus` wins over descendant autofocus. Descendant autofocus
applies only when the scope has no current focus and is not behind a modal. At
application startup, canonical order breaks ties only after validation has
reported multiple autofocus declarations as an error.

`FocusScopeProps` is attached with `.focus_scope(...)`, independent of semantic,
focus, and interaction bundles. Its complete snapshot and install/remove mutation
cross the wire. Restoration tries candidates once in vector order and clears
input focus if none is eligible. Empty restoration is valid and means clear.
`use_dialog` builds invoker, nearest eligible logical ancestor, optional explicit
scope fallback, then first eligible in the parent scope.

Plain visual descendants do not need semantic props. Text and images only enter
the accessible name-from-content walk when the owning role permits it and their
semantic visibility is `Exposed` or `NameSourceOnly`. A decorative image uses
`SemanticRole::Image` with an explicit empty/decorative designation rather than
an empty guessed name.

## Semantic-tree model

### Tree membership

The semantic tree is a projection of the committed logical Reactant tree, not the
physical UI Toolkit hierarchy. An exposed semantic node has:

- an `AccessibilityId`;
- a logical semantic parent and ordered semantic children;
- an identity owner plus an optional physical focus host used for attachment and
  input focus;
- resolved name, description, state, value, relations, and actions;
- optional reading-order and collection metadata; and
- a source location in development builds for diagnostics.

Component and fragment nodes are normally transparent. A host without semantics
is also transparent. An explicit group, landmark, list, or other structural role
creates a semantic node. Virtual nodes may be inserted by hooks, but remain owned
by a stable logical host and cannot outlive it.

Projection maintains three explicit views:

- the **declaration graph** contains every current semantic declaration,
  including `NameSourceOnly` and `Hidden` nodes, so diagnostics and references
  have stable source objects;
- the **canonical semantic forest** contains declared `Exposed` nodes in logical
  semantic ancestry, plus text contributed by referenced `NameSourceOnly` nodes;
  it is independent of transient UI Toolkit attachment and computed visibility;
  and
- the **active presentation forest** is the Unity adapter's traversal view after
  attachment, computed visibility, inherited author inertness, modal exclusion,
  reading order, and overlay-root promotion.

`Hidden` nodes never contribute text or relations and cannot be relationship
targets. `NameSourceOnly` nodes are absent from both forests but may be the target
of labelled-by, described-by, details, or error-message. They cannot own focus,
actions, state, values, collection metadata, or exposed descendants.

Default reading order is depth-first semantic child order. An optional
`ReadingOrder` can reorder only direct semantic children and must name each
exposed child exactly once. It cannot reparent nodes or reach across a modal
scope. This keeps visual overlays and unusual layouts expressible without making
arbitrary global ordering normal.

Ordinary Tab traversal uses one deterministic projection. Start at the active
presentation roots, walk active semantic nodes depth-first in effective reading
order, and collect nodes whose `FocusProps.tab_stop` is `Auto`, whose focus host
is attached, and which are neither disabled nor inert. `Excluded` nodes are
skipped.
For each composite, only the policy's current `CompositeMember` is inserted; all
other members are navigated internally. Portals have no effect because the walk
is logical. A modal scope replaces the page roots with the top modal presentation
root. Shift-Tab reverses the resulting sequence.

There is no separate author-supplied numeric focus order. This prevents reading
and Tab order from drifting. Programmatic and accessibility focus may target an
eligible `Excluded` node, such as a text field managed by active descendant, but
ordinary Tab cannot.

### Portals

A portal changes the physical parent used for rendering. It does not change the
semantic parent, relationship scope, name computation, event ancestry, or
ownership of an overlay trigger. Geometry still comes from the physical host.

An overlay may be promoted to a presentation root while open so Unity can send a
screen-change notification and publish only the active scope. Its canonical
parent does not change. The snapshot therefore carries both `logical_roots` and ordered
`presentation_roots`; one node may have a canonical parent and simultaneously be
a presentation root. The adapter omits the canonical edge above a presentation
root from Unity traversal, without cloning the node. Relations such as
`controls`, `labelled_by`, and `described_by` continue to resolve through stable
IDs.

For a top modal, the active presentation forest contains only that dialog subtree
and any nested overlay roots. For a nonmodal overlay, page and overlay roots are
both active in declared presentation order. Closing an overlay removes its
promotion before Unity focus restoration.

Changing a portal's target follows the existing Reactant remount contract. The
old subtree and semantic IDs are removed, new hosts receive new `ObjectId` and
node incarnations, and focus restoration uses an explicit restore key if the
application wants continuity. Reconnect rebinding is different: all UI Toolkit
hosts are recreated while surviving logical portal state and semantic IDs remain.

### Names and descriptions

Reactant implements one canonical computation before sending a commit. The
algorithm follows the intent and precedence of Accessible Name and Description
Computation 1.2, translated to Reactant nodes:

1. `AccessibleName::Text` wins.
2. `LabelledBy` concatenates referenced label text in reference order.
3. `Contents` performs a logical descendant text walk only for roles that permit
   name from contents.
4. A role that requires a name and resolves to an empty string is invalid.

Whitespace is collapsed to single spaces and leading or trailing whitespace is
removed. A visually hidden label declares `NameSourceOnly` and contributes only
when reached by an explicit permitted reference. `Hidden` nodes never contribute,
even through an explicit reference. An `Exposed` label may both appear in reading
order and contribute by reference.

Description computation is separate and never repeats the source that supplied
the name. Cycles, duplicate references, cross-session references, and references
to removed nodes are errors. Application-authored role words such as “button” are
not appended because Unity and the operating-system service supply them.

### States, values, and relationships

`SemanticState` carries independent flags and enums for disabled, read-only,
required, invalid, busy, checked, pressed, selected, expanded, current, modal,
multiselectable, orientation, and sort direction. Invalid state may reference an
`ErrorMessageRef`; it does not automatically announce the error.

Range values are numeric first. `value.text` is an optional localized speech
override such as “Quiet” or “75 percent.” The Unity adapter uses the `Slider` role
and increment/decrement events for supported ranges and retains the text in
`AccessibilityNode.value`.

Resolved relationships include the labelled-by and described-by sources derived
from accessible name/description, plus authored error-message, controls, details,
flow-to, active-descendant, and dialog invoker. There is no `owns` relation:
canonical ownership is always logical semantic parentage. A portaled popup remains
a logical descendant or uses `controls` when it is a sibling. Unity lowering never
uses a relationship to silently rewrite canonical ancestry.

Headings carry a level from 1 through 6. Landmarks require a name only when more
than one landmark of the same role exists in the current scope. A validation
warning is emitted on the first ambiguous duplicate and becomes an error in the
strict accessibility profile.

### Hidden, inert, and exiting nodes

A node is excluded from the active presentation forest when it or an ancestor is
detached, has `display: none`, has UI Toolkit visibility hidden, is inside an
inactive modal background, declares `SemanticVisibility::Hidden`, or inherits
`SemanticProps.inert`. Unity computes and stores the corresponding
`PresentationExposure`; render-hidden and detached nodes stay in the mirror but
are not published through Unity. A `NameSourceOnly` node remains in the declaration
graph only. Opacity alone never hides semantics. The author must hide a purely
visual zero-opacity duplicate explicitly.

When presence removes a logical child, Reactant removes that subtree from the
semantic tree in the same commit that starts its exit animation. Unity first
moves accessibility and input focus to the declared restoration target or an
eligible ancestor. The retained physical hosts then animate with input disabled
and no semantic nodes. There is no option to keep an interactive exiting subtree
accessible.

An element may remain visually mounted but semantic-inert by declaring `inert` on
any semantic ancestor or focus scope. Inert is inherited and disables Unity
publication, accessibility actions, input focus, and focus navigation. Disabled is
not the same as hidden or inert: disabled controls remain discoverable and expose
their disabled state.

### Dynamic content and announcements

Live regions are semantic containers with `Politeness::Polite` or
`Politeness::Assertive`, an atomicity policy, and an optional relevance filter.
Reactant compares their resolved text after reconciliation and emits an
announcement only for an eligible committed change. Initial mount is silent
unless `announce_initial` is explicit.

Imperative announcements use a `use_announce` handle. They are ordered one-shot
commit records with `AnnouncementId`, locale, politeness, resolved text, and
`Deduplication::None`, `UntilChanged(key)`, or `Within(key, duration)`.

An announcement moves through `Pending`, `Submitted`, `Acknowledged`, or
`Dropped`. Rust creates `Pending`; Unity changes it to `Submitted` only when its
notification dispatcher accepts the call, then immediately returns an
acknowledgement event. Acknowledgement means accepted by Unity, not spoken.
Dispatcher rejection or lack of support changes it to `Dropped` with a reason and
reports degraded health.

Unity's `SendAnnouncement(string)` accepts only text. It cannot express polite
versus assertive delivery, locale, atomicity, or relevance. Reactant still uses
those declarations to decide what text to resolve, deduplicate, coalesce, and
submit, and preserves the full announcement record canonically. Every submitted
announcement records `FallbackClass::LiveAnnouncement` because Unity chooses the
delivery urgency. Submission order is best effort and does not imply interruption
or speech order.

Polite records with the same key coalesce while pending in one frame. On
reconnect, pending polite records older than two seconds are stale and dropped;
newer records submit once. A pending or submitted assertive record submits once
in the new backend generation unless Rust already received its acknowledgement.
`UntilChanged` suppresses the same text hash until different text is submitted
for that key. `Within` suppresses the same key/text pair until its Rust-runtime
deadline. Live regions use `UntilChanged` by default; imperative announcements
default to no deduplication.

Assertive messages from one commit preserve commit order. Empty text is ignored.
Diagnostic tooling records the canonical record, every state transition, backend
generation, and disposition. Disconnect, failure, duplicate-key, delayed-ack,
and replay transitions are deterministic tests.

### Virtualized collections

Collection semantics are keyed by application item keys, never rendered row
indices. A virtual window declares total size or `Unknown`, each materialized
item's one-based position, selection state, and the keys immediately before and
after the window when known.

`CollectionWindowInfo` is the wire declaration for that state. Unity offers
continuation only when the relevant side is `Available`. It sends the declared
scroll proposal and requests focus for the known neighbor key; when the key is
unknown, the accepted `ControlValue` resolution must name the focus key in the
new window. An accepted response whose window bounds, continuation, materialized
item positions, and focus key disagree is invalid.

The canonical mirror exposes materialized items with position and set-size
metadata and never synthesizes thousands of nodes. Unity v1 may fold useful
position text into `value` or `hint`, but has no collection-continuation action.
Keyboard, controller, or Ditto next, previous, page, or scroll interaction at a
window edge dispatches the typed collection intent. Rust updates the window
synchronously through the existing runtime call; Unity admits the response,
applies it at the safe response gate in the same frame, sends a layout-changed
notification when supported, and restores accessibility focus by item key.

Because Unity v1 cannot request collection continuation through an accessibility
action, it exposes the current window and reports the total size in the adapted
collection summary. It must not claim that the last materialized item is the last
item in the collection.

## Focus, navigation, events, and default actions

Accessibility integrates four systems that remain separately observable.

### Focus kinds

Reactant distinguishes:

- **input focus**, the UI Toolkit element that receives keyboard and controller
  input;
- **accessibility focus**, the canonical semantic node selected by assistive
  technology;
- **navigation focus**, the current item in a composite or directional focus
  scope; and
- **focus-visible state**, the styling signal derived from input modality.

Moving one does not universally move the others. Accessibility focus on static
text does not move input focus. Accessibility focus or activation of an
interactive control requests input focus when its `FocusProps` says
`sync_from_accessibility: true`. Input focus normally updates accessibility focus
only after keyboard or controller navigation, not after a pointer click.

Unity tracks the last meaningful modality synchronously:

- pointer, touch, keyboard, controller, accessibility, or programmatic;
- synthetic mouse events following touch do not change modality;
- programmatic focus preserves the preceding modality unless the request carries
  an explicit reason; and
- reconnect starts with `Programmatic` and no focus-visible ring until an actual
  user modality or a restored keyboard/controller focus is known.

`use_focus_visible` returns focused and focus-visible state without styling it.
Keyboard, controller, and accessibility focus are focus-visible. Pointer and
touch focus are not, unless the platform accessibility setting or author policy
requests an always-visible indicator.

Accessibility focus requests use one Unity state machine:

1. A new process-local `FocusRequestId` cancels any globally pending older focus
   request, then `Validate` resolves generation, node incarnation, active
   exposure, active screen-reader status, focus host, and optional scroll route.
2. A visible node moves to `FocusRequestPending`. The adapter requests focus by
   passing the node to `SendLayoutChanged` or `SendScreenChanged`; those methods
   return no success result, so Reactant does not yet change canonical focus or
   focus-visible state.
3. An offscreen but revealable node moves to `RevealPending`; Unity returns
   handled to the Unity request but keeps the previous accessibility focus.
4. Reveal is a Unity-local scroll-to-element operation identified by the same
   `FocusRequestId`; it is not a Rust proposal and creates no wire event. Unity
   applies it at the safe gate and waits at most two frames or 250 ms, whichever
   comes first, for nonempty visible geometry.
5. Successful reveal continues through `FocusRequestPending`. An exact
   `AccessibilityNode.focusChanged(true)` or `AssistiveSupport.nodeFocusChanged`
   callback for the requested node moves to `FocusNow`, optionally correlates
   input focus, and emits `Focused`.
6. Reveal failure, screen-reader deactivation, focus moving to another node, or
   no confirming Unity focus event within one second moves to `Failed`, preserves
   the last confirmed focus, and reports a diagnostic/result event.

Only `FocusNow` updates the canonical accessibility-focus field. A Unity callback
reporting that assistive technology already focused a visible node may
enter at that step. It cannot immediately focus a clipped node and bypass reveal.
There is at most one pending accessibility-focus request for the runtime. Latest
request wins; cancellation prevents the older request from setting focus or
emitting `Focused`, even if its layout becomes visible or a delayed notification
arrives later.

### Declared navigation policies

Ordinary Tab traversal follows the active reading-order algorithm defined above.
Composite patterns install a bounded `NavigationPolicy` in Unity:

```rust
pub struct NavigationPolicy {
    pub kind: NavigationKind,
    pub orientation: Orientation,
    pub wrap: bool,
    pub direction: DirectionBehavior,
    pub selection_follows_focus: bool,
    pub typeahead: Option<TypeaheadPolicy>,
    pub members: Vec<AccessibilityRef>,
}
```

Unity can therefore handle arrows, Home, End, Page Up, Page Down, controller
direction, submit, and cancel before UI Toolkit performs an unrelated default.
The policy is data, not remotely executable Rust code. Unity moves focus
synchronously, updates a controlled local draft when appropriate, and emits a
typed logical intent afterward.

Tab enters a composite at its active or first eligible member, then leaves the
composite on the next Tab. Arrow keys move within radio groups, tab lists, menus,
listboxes, grids, and trees. Disabled members are skipped for focus except in a
menu policy that explicitly permits discoverable disabled items.

Horizontal spatial navigation mirrors in right-to-left layouts. `ArrowRight`
moves visually right and `ArrowLeft` visually left; the logical next and previous
key selected by that motion therefore reverse. Home and End remain first and last
in declared collection order. Horizontal range controls reverse increment and
decrement so the larger value follows the visual increasing direction. Vertical
controls are unchanged.

### Event ownership

Logical Reactant capture, target, and bubble remain authoritative for application
intent events, including across portals. Accessibility action events use the same
committed logical ancestry and can be stopped from reaching ancestors. Stopping
propagation does not cancel a native action that Unity already performed.

Accessibility actions use a parallel committed handler registry keyed by the
complete `AccessibilityId`, including slot/key and incarnation. Composing
`InteractionProps` on a host binds its handlers to that host slot; passing them to
`VirtualSemanticNode` binds them to the allocated virtual ID. Handlers are never
looked up by `identity_owner` alone.

The semantic dispatcher resolves the target ID, collects its canonical semantic
ancestors, and runs the same capture, target-capture, target-default, and bubble
phase order as the host dispatcher. Because canonical semantic ancestry is
projected from logical Reactant ancestry, portals do not alter the route. A
transparent logical host contributes no semantic handler; an application that
needs an action boundary declares a transparent-named `Group` semantic node and
attaches capture/bubble handlers there.

```rust
pub struct AccessibilityActionEvent {
    pub target: AccessibilityId,
    pub current_target: AccessibilityId,
    pub phase: EventPhase,
    pub action: AccessibilityAction,
}
```

The event also exposes `logical_owner() -> ElementTarget` and
`stop_propagation()`. Target and current target retain semantic slot/key identity;
the logical owner is only an inspection bridge to the existing host identity.
All handlers share the existing single mutable borrow of `G` and state batch. An
unknown, stale, inactive, unsubscribed, or role-invalid target invokes nothing.
Admission-backed actions reject when no target-default handler accepts. Rust
stopping propagation never rolls back the Unity-owned default.

Every standard interaction declares one default-action owner:

| Interaction | Class | Synchronous owner | Rust result |
| --- | --- | --- | --- |
| Move composite focus | Unity-owned | navigation policy | focus observation |
| Activate/press/show help/custom | admission-backed | action adapter | handler admission |
| Toggle/select/change tab | proposal-backed | controlled adapter | boolean/key resolution |
| Increment/decrement/set range | proposal-backed | range adapter | numeric resolution |
| Edit text/selection | proposal-backed | text adapter | text/selection resolution |
| Expand/collapse | proposal-backed | disclosure adapter | expanded resolution |
| Dismiss overlay | proposal-backed | overlay stack | open=false resolution |
| Continue virtual window | proposal-backed | collection adapter | window/key resolution |
| Raw application shortcut | post-default event | UI Toolkit | key observation |

Rust never attempts late `prevent_default`. A behavior hook installs the finite
Unity policy required to own a standard default. Raw event handlers remain
post-default and are unsuitable for reimplementing those standards.

Admission-backed actions have no local Unity value to roll back. The callback returns
handled when a matching committed handler runs and returns accept. Rejection or
runtime failure leaves focus and semantic state unchanged.

Unity-owned navigation focus moves before Rust observation and persists if the
observation handler is absent, rejects, or fails. Selection does not follow that
move unless its separate proposal is accepted. If an accepted render removes the
focused target, normal focus restoration runs during commit.

For a proposal-backed action, Unity may expose a local draft to assistive
technology during the callback. It submits the proposal to Rust, restores the
committed state without notification, and applies the authoritative response at
the dispatch gate. On rejection, it sends a layout/value notification only when
Unity or the operating-system service had already announced the draft.

Every controlled proposal carries
`ProposalId { backend_generation, sequence }`, target ID including node
incarnation, action kind, previous canonical value, and proposed value. Unity
creates IDs monotonically on the main thread and allows at most one in-flight
proposal per target/action kind. A second action is serialized after the
synchronous runtime response; it never overwrites the first draft.

The matching Rust behavior handler returns `IntentResult::Accept` or
`IntentResult::Reject`. Convenience setters accept after invoking their callback.
The same runtime response must contain exactly one
`ResolveProposal { id, result, authoritative_value }`. Missing, duplicate, stale,
or mismatched resolutions reject the entire response. The authoritative value is
also present in the resolved semantic upsert, and the two must agree.

Unity restores the previous value before leaving the accessibility callback, admits the
response, and applies the authoritative value at the safe gate. “Handled” is true
only for an accepted resolution. Reject, runtime error, disconnect, or callback
timeout returns false and leaves the previous value observable. A response from
an old generation is discarded. This correlation applies to toggle, selection,
range, text, expand/collapse, dismiss, and virtual-window proposals. Their valid
`ControlValue` variants are, respectively: boolean/mixed, key or key set, number,
text/selection, expanded boolean, expanded boolean, and window range plus focus
key. An incorrect variant rejects the response before commit.

### Input disable and rebinding

Global `input_disabled` disables input focus, accessibility actions, focus
navigation, controlled drafts, capture, and overlay dismissal. The semantic tree
remains readable with a busy or disabled state supplied by the application.

An input-rebinding scope declares `InputCapturePolicy::rebind(...)`. Unity routes
the
next eligible physical key or controller control to that scope instead of normal
activation. Reserved cancel controls remain available. The resulting binding is
a Rust intent; conflicts and validation are Rust state. The capture dialog keeps
its semantic name and instructions stable, updates its validation description,
and announces a new conflict through a polite status region.

## Pattern APIs

The following signatures are representative and normative in shape. Naming may
change during implementation only if the separation and returned contracts stay
intact.

### Shared hook result

```rust
pub struct AccessibleBehavior<G, S> {
    pub semantic: SemanticProps,
    pub focus: FocusProps,
    pub interaction: InteractionProps<G>,
    pub state: S,
}
```

`InteractionProps<G>` contains logical handlers plus serializable synchronous
policies. It is invariant in `G` and cannot be attached to a runtime using another
global state type.

### Button

```rust
let button = use_button(ButtonOptions {
    name: AccessibleName::Text(text("Save changes")),
    is_disabled: saving,
    on_press: callback(|app: &mut App| app.save()),
});

VisualElement::new()
    .semantic(button.semantic)
    .focus_props(button.focus)
    .interaction_props(button.interaction)
    .class_if("pressed", button.state.is_pressed)
    .children(save_icon_and_label())
```

`use_button` returns the button role, disabled state, activate action, press
policy, input-focus policy, and `PressState`. It supports pointer, touch, Enter,
Space, controller submit, and Unity accessibility activation without emitting
duplicate logical presses. The hook does not require a native `Button` host.

A repeat button composes `use_button` with `use_repeat_press`. A toggle button
adds `pressed` state and a controlled toggle proposal; it is not a checkbox.

### Link

`use_link` exposes a link role, destination description when useful, focus and
press behavior, and `OpenLinkIntent`. The application owns navigation. A link is
not inferred from underlined text and a button does not become a link because its
handler opens a URL.

### Checkbox and switch

```rust
let state = use_toggle_state(props.is_checked, props.on_change);
let checkbox = use_checkbox(
    &state,
    CheckboxOptions {
        label: labelled_by(label_ref),
        description: described_by(help_ref),
        is_required: true,
        validation: props.validation,
    },
);

GroupBox::new()
    .semantic(checkbox.semantic)
    .focus_props(checkbox.focus)
    .interaction_props(checkbox.interaction)
    .children(custom_checkmark(state.is_selected()))
```

The semantic checked value supports `False`, `True`, and `Mixed`. Read-only keeps
the control focusable and readable but removes toggle actions. Disabled keeps it
readable and exposes disabled state. Required and invalid are independent.

`use_switch` reuses toggle state and press mechanics but returns the switch role.
It is reserved for an immediate on/off setting; a choice that participates in a
form or supports mixed state uses checkbox semantics.

### Radio groups

`use_radio_group` returns group semantics, orientation, one group-level
description/error bundle, and a roving navigation policy. `use_radio` returns each
radio's semantics, position, selected state, focus props, and selection proposal.
Only one eligible radio is in the Tab sequence. Arrows move and select by default;
an explicit manual-selection option is allowed only for product requirements that
conflict with the APG convention.

Every radio belongs to exactly one live group and uses an application key rather
than its rendered index. A group with no label is invalid.

### Slider

```rust
let range = use_range_state(0.0..=100.0, volume, set_volume);
let slider = use_slider(
    &range,
    SliderOptions {
        label: text("Music volume"),
        orientation: Orientation::Horizontal,
        page_size: 10.0,
        format_value: percent_formatter(),
    },
);
let thumb = use_slider_thumb(&slider, 0);

VisualElement::new()
    .semantic(slider.track_semantic)
    .children(
        VisualElement::new()
            .semantic(thumb.semantic)
            .focus_props(thumb.focus)
            .interaction_props(thumb.interaction)
            .style(custom_thumb_style(thumb.state)),
    )
```

The thumb exposes the numeric minimum, maximum, current value, optional step,
localized value text, increment, decrement, set-value, and drag actions. Arrow
keys use one step; Page keys use `page_size`; Home and End use endpoints.
Pointer/touch drag remains a controlled proposal. A multi-thumb range slider uses
one semantic thumb per value and dynamically constrains each thumb to avoid
crossing its neighbors.

The track may be semantically transparent for a single slider or an explicit
group for multiple thumbs. The hook does not require UI Toolkit `Slider`.

### Tabs

```rust
let selection = use_single_selection_state(active_tab, set_active_tab);
let tabs = use_tabs(&selection, TabsOptions::horizontal(text("Settings")));
let tab = use_tab(&tabs, TabKey::Audio, text("Audio"));
let panel = use_tab_panel(&tabs, TabKey::Audio);

TabHost::new()
    .semantic(tab.semantic)
    .focus_props(tab.focus)
    .interaction_props(tab.interaction);

PanelHost::new()
    .semantic(panel.semantic)
    .hidden(!panel.state.is_selected)
    .children(audio_settings())
```

`use_tabs` returns tab-list semantics and roving navigation. Each tab owns a typed
`controls` relation to exactly one panel; each panel is labelled by its tab. The
default is automatic activation because Reactant panels are local. Applications
may choose manual activation for expensive content, in which case arrows move
focus and Enter, Space, or controller submit selects.

Only the selected panel is exposed. A panel retained by presence becomes hidden
and inert at deselection before its visual exit. Removing the selected tab chooses
the nearest enabled tab in collection order and announces the new selection only
through Unity's ordinary focus/selection feedback.

### Dialogs and overlays

```rust
let overlay = use_overlay_state(is_open, set_open);
let dialog = use_dialog(
    &overlay,
    DialogOptions {
        name: labelled_by(title_ref),
        description: described_by(instructions_ref),
        modality: DialogModality::Modal,
        initial_focus: FocusTarget::FirstEligible,
        dismiss: DismissPolicy::EscapeAndControllerCancel,
    },
);

Portal::new(overlay_root())
    .child(
        VisualElement::new()
            .semantic(dialog.semantic)
            .focus_scope(dialog.focus_scope)
            .interaction_props(dialog.interaction)
            .children(authored_dialog_contents()),
    )
```

`use_dialog` returns dialog semantics and a `FocusScopeProps`, not a backdrop,
title, close button, layout, or portal. A modal scope:

- hides and makes inert every sibling scope below it in the overlay stack;
- moves focus after the dialog's semantic nodes and hosts are committed;
- traps Tab and controller navigation within eligible descendants;
- routes Escape, controller cancel, and Unity accessibility dismiss to the top scope;
- preserves a typed invoker/restoration target; and
- restores focus only after the closing subtree becomes semantic-inert.

Nested overlays form a Unity-resident stack. Closing the top overlay restores the
previous overlay's focus, not the underlying page. If the invoker no longer
exists, restoration tries the nearest eligible logical ancestor, then the
scope's explicit fallback, then the first page control. Failure clears input
focus and sends a screen-changed notification; it never focuses an arbitrary
physical portal sibling.

An alert dialog additionally requires a concise name and an initial-focus target
that does not cause long static content to be skipped. Backdrop click is an
independent pointer interaction option and is never inferred from modal state.

### Menus

`use_menu_trigger` composes a button with expanded state, a controls relation, and
open/close focus restoration. `use_menu` returns menu semantics, vertical roving
navigation, Home/End, typeahead, and dismissal. `use_menu_item` returns item
semantics and activation; check and radio variants add checked state.

Submenus use an overlay scope linked to their parent item. Right and Left open or
close according to layout direction. Escape closes only the deepest menu. Tab
closes the menu stack and resumes normal traversal rather than moving among menu
items.

### Listboxes and comboboxes

`use_listbox` returns collection semantics, selection policy, typeahead, and
focus policy. `use_option` returns option semantics, selected/disabled state,
position metadata, and selection intent. Single and multiple selection are
distinct state interfaces.

`use_combobox` composes text-input, trigger-button, and listbox bundles:

- the text field exposes the combobox role, current text, expanded state, and
  controls relation;
- the button opens without stealing persistent text focus;
- the popup listbox is portaled but logically related;
- active descendant identifies the currently navigated option; and
- input value, selected key, and open state remain independently controlled.

Typing filters through Rust state. Unity owns arrows, Home, End, page movement,
Escape, and controller navigation while the popup is open. Enter or submit commits
the active option. On close, accessibility focus returns to the field and input
focus remains there.

### Tooltips

`use_tooltip_trigger` returns hover, focus, delay, and described-by behavior.
`use_tooltip` returns tooltip semantics. A tooltip never receives input focus,
contains interactive children, or substitutes for a required label. Escape can
dismiss a visible tooltip without moving focus. Touch platforms use an explicit
help action or long-press policy rather than hover emulation.

### Progress, status, and common structural patterns

`use_progress` exposes determinate range values or indeterminate busy state. It
requires a name unless an enclosing labelled operation supplies one. Value
changes are not live announcements by default; native range progress feedback is
preferred, and milestones can be announced explicitly.

Other low-level hooks include:

- `use_heading`, `use_landmark`, `use_group`, `use_separator`, `use_image`, and
  `use_static_text`;
- `use_text_field`, `use_search_field`, and `use_validation`;
- `use_disclosure`, `use_table`, `use_grid`, `use_tree`, and their keyed item
  hooks;
- `use_collection`, `use_roving_focus`, `use_typeahead`, and
  `use_active_descendant`;
- `use_press`, `use_hover`, `use_focus_visible`, `use_drag`, and
  `use_long_press`; and
- `use_live_region` and `use_announce`.

Table and grid APIs keep row, column, and header relationships explicit. A visual
layout made from arbitrary elements can therefore expose a table without using a
native table component. A grid opts into two-dimensional keyboard navigation; a
plain table remains reading-only.

## Reconciliation and commit lifecycle

### Rust projection

Semantic projection runs after visual reconciliation has produced stable logical
children and before either tree is committed. It performs these passes:

1. collect host and virtual semantic declarations with source locations;
2. resolve logical semantic parentage through transparent nodes and portals;
3. resolve typed relationships and accessible names/descriptions;
4. derive modal visibility, inertness, reading order, and collection metadata;
5. validate the complete candidate semantic tree;
6. diff it against the committed semantic tree; and
7. add its mutation stages to the same `ReactantCommit` as visual mutations.

Any validation failure aborts both visual and semantic work. Unity never sees a
new host tree paired with the previous semantic tree or the reverse.

The committed Rust semantic tree stores declarations and resolved canonical
values. It does not store Unity fallbacks. This allows reconnect or a future
Unity capability increase to reclassify the tree without rerendering the
application.

### Ordered Unity application

The Unity executor stages the complete response, validates references against the
post-commit object index, then applies these barriers:

1. deactivate semantic nodes that become hidden, inert, or removed;
2. move accessibility/input focus away from those nodes;
3. create and reparent visual hosts that new semantic nodes require;
4. update host properties and semantic nodes;
5. install navigation, controlled-action, and overlay policies;
6. execute focus and scroll actions;
7. submit supported layout, screen, value, and announcement notifications to
   `UnityEngine.Accessibility`; and
8. destroy visual hosts no longer retained by presence.

The response gate still prevents mutation during UI Toolkit propagation. Unity
accessibility callbacks enter through the same synchronous runtime dispatch used
by other UI events, and their response is admitted at that safe gate. The callback
returns “handled” when a live declared action route accepted the event, not when
the later visual render happens to change.

Motion may change geometry between Reactant commits. The Unity accessibility
manager therefore uses `AccessibilityNode.frameGetter` and marks
affected frames dirty in the Motion post-update player-loop phase. It coalesces
one layout-changed notification per semantic root per frame. Geometry animation
does not mutate semantic content.

### Reconnect

The reconnect snapshot includes the complete resolved semantic tree, action
policies, modal stack, current logical input focus, accessibility focus when
known, locale, direction, and unacknowledged assertive announcements.

Unity recreates visual hosts first, then semantic nodes with the same structured
IDs. It restores modal inertness and focus only after all referenced hosts exist.
If the previously focused node is no longer eligible, it runs the normal logical
restoration algorithm. A reconnect sends one screen-changed notification after
the complete tree is active; it does not announce every recreated node.

Unity node IDs and objects may change across reconstruction. Only
`AccessibilityId` is stable across reconnect. An incoming Unity callback resolves
through the current backend generation, so a callback from a disposed hierarchy
is rejected.

## Wire protocol

Accessibility is session-wide rather than nested under a `UiDocument`, because a
semantic tree may relate hosts across documents and external portal targets.

The initial `Snapshot` gains:

```rust
pub struct AccessibilitySnapshot {
    pub commit_sequence: u64,
    pub logical_roots: Vec<AccessibilityId>,
    pub presentation_roots: Vec<PresentationRootSnapshot>,
    pub nodes: Vec<AccessibilityNodeSnapshot>,
    pub relationship_sources: Vec<RelationshipSourceSnapshot>,
    pub policies: Vec<AccessibilityPolicySnapshot>,
    pub focus_scopes: Vec<FocusScopeSnapshot>,
    pub modal_stack: Vec<AccessibilityId>,
    pub locale: LocaleId,
    pub direction: LayoutDirection,
    pub input_focus: Option<AccessibilityId>,
    pub accessibility_focus: Option<AccessibilityId>,
}

pub struct AccessibilityNodeSnapshot {
    pub id: AccessibilityId,
    pub canonical_parent: Option<AccessibilityId>,
    pub canonical_children: Vec<AccessibilityId>,
    pub declared_exposure: DeclaredExposure,
    pub focus_host: Option<ObjectId>,
    pub role: SemanticRole,
    pub name: LocalizedText,
    pub description: Option<LocalizedText>,
    pub state: SemanticState,
    pub value: Option<SemanticValue>,
    pub relations: ResolvedSemanticRelations,
    pub collection: Option<CollectionItemInfo>,
    pub collection_window: Option<CollectionWindowInfo>,
    pub actions: ActionSet,
    pub geometry: ResolvedGeometrySource,
    pub fallback: FallbackPolicy,
    pub alias: Option<SemanticAlias>,
    pub content: SemanticContent,
}

pub struct RelationshipSourceSnapshot {
    pub id: AccessibilityId,
    pub text: LocalizedText,
}

pub enum DeclaredExposure {
    Eligible,
    InactiveModalBackground,
    AuthorInert,
}

pub enum PresentationExposure {
    Active,
    InactiveModalBackground,
    AuthorInert,
    RenderHidden,
    Detached,
}

pub struct PresentationRootSnapshot {
    pub id: AccessibilityId,
    pub policy: PresentationPolicy,
    pub activation_order: u64,
}

pub struct FocusScopeSnapshot {
    pub props: FocusScopeProps,
    pub active: bool,
    pub navigation_focus: Option<AccessibilityId>,
    pub resolved_restore_target: Option<AccessibilityId>,
}
```

The canonical semantic forest and nontraversal relationship sources cross the
wire separately. `NameSourceOnly` becomes a `RelationshipSourceSnapshot`; it is
never a root, child, focus target, or action target. This preserves labelled-by,
described-by, details, and error-message IDs for canonical inspection and future
Unity mappings. Unity v1 flattens the same source text under the explicit
fallback rule. `Hidden` declarations are absent.

Rust declares eligible, modal-background, or author-inert exposure. Unity
combines that with live attachment and computed UI Toolkit visibility to produce
`PresentationExposure`. Nodes remain in the Unity mirror while render-hidden or
detached so they can reactivate atomically. The Unity adapter publishes only
`Active` nodes; relationship-only source objects remain in the canonical mirror.

Incremental response bodies gain:

```rust
pub enum AccessibilityCommand {
    Apply(AccessibilityMutationBatch),
    ResolveProposal(ProposalResolution),
    Focus(AccessibilityFocusCommand),
    Notify(AccessibilityNotification),
    Announce(Announcement),
}

pub struct AccessibilityMutationBatch {
    pub commit_sequence: u64,
    pub removals: Vec<AccessibilityId>,
    pub upserts: Vec<AccessibilityNodeSnapshot>,
    pub source_removals: Vec<AccessibilityId>,
    pub source_upserts: Vec<RelationshipSourceSnapshot>,
    pub logical_roots: Option<Vec<AccessibilityId>>,
    pub presentation_roots: Option<Vec<PresentationRootSnapshot>>,
    pub policies: Vec<AccessibilityPolicyMutation>,
    pub focus_scopes: Vec<FocusScopeMutation>,
    pub modal_stack: Option<Vec<AccessibilityId>>,
}
```

`upserts` contain complete canonical nodes, not sparse Unity patches. The batch
itself is sparse, while each changed node is self-contained. This simplifies
hierarchy rebuilding, deterministic fixtures, and capability fallback. Removals are
children before parents; upserts are parents before children. Relationship source
upserts are available before dependent node upserts. A source removal follows all
node removals/updates that drop its last reference. The staged post-batch graph
must contain no dangling relationship ID.

Unity-to-Rust events gain:

```rust
pub struct AccessibilityEventEnvelope {
    pub backend_generation: u64,
    pub event: AccessibilityEvent,
}

pub enum AccessibilityEvent {
    Focused { target: AccessibilityId },
    FocusRequestFailed {
        target: AccessibilityId,
        reason: FocusFailure,
    },
    Action {
        target: AccessibilityId,
        action: AccessibilityAction,
        proposal: Option<Proposal>,
    },
    AnnouncementAcknowledged { announcement: AnnouncementId },
    BackendStatus(AccessibilityBackendStatus),
}

pub struct Proposal {
    pub id: ProposalId,
    pub target: AccessibilityId,
    pub action: AccessibilityActionKind,
    pub previous: ControlValue,
    pub proposed: ControlValue,
}

pub struct ProposalResolution {
    pub id: ProposalId,
    pub result: IntentResult,
    pub authoritative: ControlValue,
}
```

`FocusFailure` is stale generation, stale incarnation, inactive, disabled,
screen reader inactive, missing focus host, reveal rejected, reveal target
removed, reveal timeout, Unity focus timeout, superseded focus, or adapter
failure. `FocusScopeMutation` installs a complete scope snapshot or
removes it by owner. `PresentationRootSnapshot.activation_order` is assigned by
Rust commit sequence and never by physical portal order.

`ControlValue` is a closed tagged value: unit, boolean/mixed, selected semantic
key or key set, number, text with selection, expanded boolean, or virtual-window
range plus focus key. A proposal action and target determine its one valid value
variant. `ProposalId` contains `backend_generation` and a monotonically
increasing 64-bit sequence. Sequence wrap terminates and recreates the backend
generation.

The lifecycle counters have distinct meanings:

- `commit_sequence` is Rust's session-local accepted visual/semantic commit
  number. Incremental batches must be exactly previous plus one; a full snapshot
  resets the Unity mirror to the supplied value.
- `backend_generation` is Unity's activation count for the Unity accessibility
  adapter. It changes on reconnect, hierarchy replacement, or capability-set
  replacement and accompanies every callback envelope and proposal.
- `NodeIncarnation` is Rust's per-slot logical-lifetime token and is part of every
  `AccessibilityId`.

Actions are typed: activate, increment, decrement, set range value, toggle,
select, expand, collapse, dismiss, show help, scroll by direction/page, move to a
collection boundary, the complete text-edit variants above, and custom named
application action. A custom action must have localized text and an explicit Rust
handler. The Unity adapter must not turn an unsupported action into a click.

The protocol handshake advertises:

- Unity version, runtime platform, and whether `AssistiveSupport` is available;
- supported roles, states, relations, range and collection interfaces;
- custom-action, live-region, geometry, modal, and virtual-navigation support;
- screen-reader active status when available; and
- strict, degraded, or unavailable backend health.

Reactant may render on a degraded backend, but records each exercised fallback.
The Rust and Unity packages are built from the same generated schema. A payload
that contains an unknown field or enum variant fails the whole existing session
handshake instead of silently dropping semantic information. No accessibility
schema compatibility adapter is provided.

Rust protocol fixtures and C# DTOs are generated and reviewed together.

Capability enforcement has two inputs. `RequiredCapabilities` is optional static
player configuration checked once during handshake. It can require a particular
Unity capability for a product that cannot operate plausibly without it, but the
v1 default requires none and permits best-effort lowering. `FallbackPolicy` is
declared on nodes and checked on every candidate commit against Unity's reported
capability set. An explicitly forbidden exercised fallback rejects that complete
visual/semantic commit and leaves the previous complete commit active; it does
not deactivate the session.

`AccessibilityCoverage::Required` is separate. It rejects an actionable or
focusable descendant with no explicit semantic contract, regardless of backend
capability. Backend health is `Available`, `Degraded`, or `Unavailable`.
`Degraded` means at least one allowed fallback is active. `Unavailable` means no
Unity accessibility hierarchy is published. Strictness is policy applied to
those facts, not a fourth health state.

A generated pure fallback classifier is authoritative. Its inputs are backend
kind, Unity version, runtime platform, immutable capability set, and a resolved
canonical node; its output is the exact `AccessibilityNode` mapping plan and
`BTreeSet<FallbackClass>`. Rust runs the classifier before accepting a candidate
commit. Unity runs the same generated tables again before hierarchy mutation and
asserts identical fallback classes. A mismatch rejects the session as a
package/schema error.

The effective fallback policy is the nearest ancestor policy after canonical
parentage is resolved. The semantic root cannot use `Inherit`. `AllowAll` accepts
and diagnoses the classifier's set. `Forbid(classes)` rejects when the intersection
is nonempty; an empty set therefore means allow all while remaining explicit.

Capabilities are immutable within one backend generation. When the screen reader
turns off, Unity clears `AssistiveSupport.activeHierarchy`; Reactant marks the
generation inactive, cancels pending focus, reports `Unavailable` with a
screen-reader-off reason, and keeps the canonical mirror. When it turns on,
Reactant creates a new backend generation, rebuilds fresh `AccessibilityNode`
objects from that mirror, reassigns `AssistiveSupport.activeHierarchy`, and sends
a full status event without requiring a Rust rerender.

A changed capability set follows the same deactivation boundary but requires Rust
to reclassify the last committed tree before Unity republishes it. Missing static
required capabilities leave the adapter unavailable; node-level forbidden
fallback leaves the previous complete tree active and reports the failed
reactivation.

## Unity host architecture

The Unity package gains an `Accessibility` subsystem beside, not inside, the
existing UI element property adapters.

### `BattlementAccessibilityManager`

One manager per Reactant runtime owns:

- the canonical Unity semantic mirror;
- indexes by `AccessibilityId`, identity owner, and physical focus host;
- staged mutation validation and commit barriers;
- accessibility/input focus correlation;
- modality and focus-visible state;
- the overlay and restoration stack;
- navigation, typeahead, press, range, and collection policy adapters;
- frame invalidation and Unity notifications; and
- backend lifecycle and capability diagnostics.

The manager depends on `BattlementUiDocuments` and the live object index for
geometry and input focus. UI code may query semantic state for diagnostics, but
does not mutate it directly.

### Backend interface

```csharp
internal interface IAccessibilityBackend
{
    AccessibilityCapabilities Capabilities { get; }
    void Activate(AccessibilityTree tree);
    void Apply(AccessibilityBackendBatch batch);
    void RequestFocus(AccessibilityId? id, AccessibilityFocusReason reason);
    void Notify(AccessibilityNotification notification);
    void Announce(AccessibilityAnnouncement announcement);
    void Deactivate();
}
```

The v1 implementations are `UnityAccessibilityBackend`, which owns one
`AccessibilityHierarchy`, and `InspectorAccessibilityBackend`, which exposes no
assistive-technology nodes. The interface is an internal testing and lifecycle
boundary, not an extension point for native plugins or browser DOM backends.

Mutation, focus, notification, and action-dispatch methods run on Unity's main
thread. `UnityAccessibilityBackend` creates, updates, moves, and removes
`AccessibilityNode` objects and publishes its hierarchy through
`AssistiveSupport.activeHierarchy`. It uses Unity's notification dispatcher for
the layout, screen, and announcement operations Unity supports.

The adapter subscribes to `AssistiveSupport.screenReaderStatusChanged` before
activation. It never assumes its hierarchy remains assigned after a disabled
status event. Re-enabling reconstructs and assigns the complete current mirror as
described above; an incremental batch is never applied to an unassigned stale
hierarchy.

Unity node action and focus events dispatch directly on the main thread. If Unity
is inside semantic commit or synchronous runtime dispatch, recursive action
dispatch is rejected as busy; it is not queued after the callback returns.
Teardown, busy, or generation/incarnation mismatch returns `false` from the Unity
event when that event permits a handled result and creates no proposal. A runtime
budget failure rejects the response and applies no safe-gate mutation.

On the main thread, the manager validates the active generation and action route,
runs the synchronous Rust call, and decides handled status from the matching
`ProposalResolution` or noncontrolled handler admission. Safe-gate mutation may
follow, but handled status is no longer ambiguous. The adapter cannot retain a
mutable node reference across a generation change.

`AccessibilityBackendBatch` contains already resolved Unity mappings plus the
canonical node for diagnostics. Mapping is deterministic and side-effect-free;
the manager computes it before entering the adapter.

### UI Toolkit integration

UI Toolkit supplies rendering, geometry, input focus, and native-control local
draft mechanics. It is not the semantic source of truth. The class name or C#
type of a `VisualElement` does not infer a role or name.

For a host-owned semantic node, the manager resolves its screen-space frame from
the current `worldBound`, panel scaling, player viewport, and platform coordinate
origin. Clipping intersects the frame with all physical clipping ancestors even
though semantic ancestry is logical. A fully clipped interactive node validates
only when `FocusProps.scroll_route` names an attached scroll container and a
bounded reveal action. It remains in the tree with an offscreen frame; Unity
focus first executes the reveal route, waits for safe-gate layout, requests focus
through Unity's notification dispatcher, and waits for a confirming focus event.
Without that route, an actionable fully clipped node is invalid.

Virtual nodes choose one geometry source:

- an owning host's frame;
- the union of named child host refs;
- a live `ElementRef` frame; or
- no frame for non-interactive structural text.

Interactive nodes without a nonempty frame or valid scroll route are invalid.
Static offscreen virtual collection summaries may have no frame. A scroll route
is invalid if its container is hidden, outside the active presentation scope, or
cannot reveal the target owner.

Native controls continue to own text editing, selection, drag, scroll, and other
local interaction behavior. Behavior hooks install their semantic contract
explicitly so a native control and a custom visual host produce the same tree.
Reactant owns `AssistiveSupport.activeHierarchy` while its adapter is active and
does not attempt to merge its nodes with another application-authored Unity
hierarchy. UI Toolkit host types do not create additional Reactant nodes.

### Unity callback lifetime

A callback resolves `(backend_generation, AccessibilityId)` including node
incarnation against the active mirror, checks exposure, disabled/read-only/action
state, and only then submits the typed event. Increment, decrement, select,
toggle, and text-set callbacks use the same controlled proposal semantics as
Unity UI events.

A Unity focus callback enters the focus state machine. An already visible
target updates accessibility focus during `FocusNow`; a clipped target remains at
the old focus through `RevealPending`. Input focus changes only through declared
`FocusProps`. A stale, hidden, inactive, timed-out, or reveal-failed target returns
the defined failure and does not fall back to an ancestor action.

## Unity mapping and capability policy

V1 has one assistive-technology backend:
`UnityEngine.Accessibility`. “Full” below means Unity has a direct documented
field, role, state, event, or notification for the canonical concept. “Adapted”
means Reactant retains the canonical concept but publishes a simpler, plausibly
useful representation. “Unavailable” means Reactant keeps the concept for
inspection and future lowering but does not publish it through Unity.

The table describes the documented API surface of the repository's pinned Unity
6000.5.8f1 version in
[`ProjectSettings/ProjectVersion.txt`](../../ProjectSettings/ProjectVersion.txt).
Phase 0 verifies the observable VoiceOver and TalkBack result; an operating
system may still phrase or navigate the mapped node differently.

| Canonical capability | Unity v1 lowering |
| --- | --- |
| Name | Full: `AccessibilityNode.label` |
| Description or help text | Adapted: resolved text in `AccessibilityNode.hint` |
| Formatted value text | Full: `AccessibilityNode.value` |
| Unity role surface | Full: `Button`, `Container`, `Dropdown`, `Header`, `Image`, `KeyboardKey`, `ScrollView`, `SearchField`, `Slider`, `StaticText`, `TabBar`, `TabButton`, `TextField`, and `Toggle` |
| Other roles | Adapted to a non-misleading supported role or `None`; otherwise omitted |
| Disabled, expanded, and selected state | Full: corresponding `AccessibilityState` |
| Other state | Adapted into concise localized hint/value text when useful; otherwise omitted |
| Activate, increment, decrement, dismiss, and scroll | Full through `invoked`, `incremented`, `decremented`, `dismissed`, and `scrolled` events |
| Other and custom named actions | Unavailable; omitted with diagnostics |
| Label/description/error/details relationships | Adapted by resolving source text into label or hint |
| Other relationships and active descendant | Unavailable as relationships; retained canonically |
| Reading order and hierarchy | Full where representable by `AccessibilityHierarchy` node order and parentage |
| Collection position, size, table, grid, and tree metadata | Adapted into concise localized value/hint text when useful |
| Geometry | Full through `frame` or `frameGetter` |
| Accessibility focus | Adapted: requested through a layout/screen notification and confirmed by Unity focus events |
| Layout, screen, and page-scrolled notifications | Full through Unity's dispatcher |
| Announcement text | Adapted through `SendAnnouncement`; politeness is unavailable and diagnosed |
| Locale/language metadata | Unavailable; localized strings remain canonical |
| Screen-reader active signal | Full through `AssistiveSupport` |

The mapping table is generated and versioned against the pinned Unity API. It
may grow when a Unity upgrade adds roles, states, relationships, actions, or
notifications. Such growth does not require an application API migration because
the canonical Rust declarations already preserve that information.

### Supported Unity players

Unity documents `AssistiveSupport` for iOS and Android. On those targets,
Reactant constructs one `AccessibilityHierarchy`, fills it with
`AccessibilityNode` objects, assigns it to `AssistiveSupport.activeHierarchy`,
and listens to Unity events for focus, invoke, increment, decrement, dismiss, and
scroll. Reactant calls no UIKit or Android accessibility API directly.

The inspector backend always maintains the canonical Unity mirror and accepts
Ditto actions. It exposes no assistive-technology nodes. It is used in the Unity
Editor, headless tests, and players where `AssistiveSupport` is unavailable.

macOS, Windows, Linux, consoles, and WebGL therefore report `Unavailable` in v1.
Reactant does not ship NSAccessibility, UI Automation, Java/Kotlin, Objective-C,
Swift, or JavaScript accessibility integrations, and WebGL does not create DOM or
ARIA nodes beside the Unity canvas. Applications on those targets still receive
keyboard/controller interaction, semantic validation, inspector diagnostics, and
Ditto coverage, but the document does not claim screen-reader access.

### Best-effort fallback order

The adapter first selects an exact Unity role, state, value, and event. If Unity
lacks an exact role, it selects a supported role only when the result preserves
the control's primary meaning; otherwise it uses `AccessibilityRole.None` or
omits a structural node that would add noise. It never maps an unsupported
interactive role to an unrelated control merely to make it actionable.

Reactant preserves resolved name, description, and formatted value whenever the
corresponding Unity fields exist. Relationship sources may be flattened into the
label or hint in canonical order. Required, invalid, checked, expanded,
collection position, and similar unsupported metadata may be expressed as a
short localized hint/value phrase when that produces a clearer experience than
silence. Reactant does not concatenate application-authored sentences or invent
a name.

Activation remains available only when a matching Unity node event safely
represents the primary action. Unsupported secondary and custom actions are
omitted. Unsupported relations, state, and actions remain present in the
canonical inspector and Ditto snapshot so a Unity upgrade can expose them later
without changing application declarations.

Every loss or adaptation records a `FallbackClass` in the capability report,
editor inspector, logs, and Ditto snapshot. `AllowAll` is the recommended v1
root policy and provides best-effort output with diagnostics. Products may use
`Forbid` or `RequiredCapabilities` for individual experiences whose degraded
mapping would be unusable, but Reactant never responds by loading a custom native
or browser backend.

## Localization and layout direction

`AccessibilityContext` carries the resolved application locale, layout direction,
number formatter, and optional default strings for hook-authored validation. A
semantic commit records its locale for canonical inspection, navigation, and
fallback formatting. Unity v1 has no node-language field, so the adapter records
the resulting capability limitation when it materially affects a published node.

Hooks do not concatenate localized sentences. Application content supplies
complete localized names, descriptions, errors, and value text. Collection
position phrases used by a fallback come from a small Reactant Unity catalog
selected by locale. Role and action words are left to Unity and the operating
system. The adapter reports when the catalog lacks the requested locale.

Direction is inherited from the nearest Reactant language-direction context, not
from the physical portal target. An explicit direction on a semantic subtree may
override it. The same resolved direction configures visual host direction,
navigation policy, and typeahead collation.

Typeahead uses locale-aware case folding and grapheme boundaries. It searches
resolved accessible names, skips disabled or hidden items according to the
pattern policy, and resets after a declared timeout. It does not inspect visual
class names or raw serialized localization keys.

## Diagnostics and developer tooling

Accessibility declarations are developer contracts. Invalid trees panic before
commit in development and test builds, consistent with other impossible Reactant
states. In production, the render fails, the entire previous visual and semantic
commit remains active, and the error is reported. Reactant never advances the
visual tree while retaining older semantics. Product policy decides whether the
runtime continues from that previous complete commit or terminates the session.

Validation errors include:

- duplicate `AccessibilityId` or more than one exclusive semantic bundle on a
  host slot;
- a required interactive name resolving to empty;
- a dangling, cross-session, circular, or role-incompatible relationship;
- a relationship or explicit reading order that creates multiple parents or a
  cycle;
- invalid role/state pairs, such as mixed state on a switch or checked state on a
  button;
- an invalid range, step, current value, heading level, collection position, or
  set size;
- an option outside its listbox, tab outside its tab list, radio outside its
  group, or panel without exactly one owning tab;
- an active descendant outside the declared owned collection;
- an unnamed modal dialog, a modal scope with no eligible restoration policy, or
  intersecting modal scopes that are not nested;
- a focusable or actionable node hidden by semantic visibility or an inert
  ancestor;
- an interactive virtual node without usable geometry or a scroll-to-reveal
  route; and
- conflicting owners for a synchronous default action.

Warnings include:

- an exposed zero-opacity duplicate;
- ambiguous duplicate landmarks;
- an excessive unvirtualized collection;
- a description that duplicates the resolved name;
- live-region churn that is coalesced every frame;
- a Unity fallback used by a live node;
- a focus order that materially differs from reading order; and
- a visually clipped focused node without a scroll container.

Each message includes semantic ID, logical component/host path, hook source
location when available, the violated rule, and a concrete correction. It never
prints localized user content unless verbose accessibility diagnostics are
explicitly enabled.

The Unity editor gains an Accessibility inspector showing:

- canonical roots and logical parentage;
- role, resolved name/description, states, values, relations, and actions;
- identity owner, physical focus host and `UiDocument`, frame, clipping, and
  portal location;
- input, navigation, and accessibility focus;
- active modality, modal/inert scopes, and restoration targets;
- Unity node mapping and fallbacks; and
- recent Unity accessibility actions, notifications, and announcements.

The inspector reads the production mirror. It does not maintain a parallel debug
tree. Selecting a node highlights its UI Toolkit frame without changing either
focus kind.

## Compatibility and rollout boundaries

Accessibility protocol, Rust types, generated JSON fixtures, Unity DTOs, and the
Unity accessibility adapter land together. A stale Rust or Unity package does
not interoperate. There is no schema negotiation that drops unknown semantic
fields.

Existing Reactant applications continue to render and receive input, but no role
or name is inferred for custom elements or native controls. They become
assistive-technology accessible only when an application composes semantic
behavior. This deliberate boundary prevents duplicate or misleading trees.

Application code can migrate incrementally by screen. A root may declare
`AccessibilityCoverage::Required`; validation then rejects any focusable or
actionable descendant without explicit semantics. Shipping applications should
eventually enable this at the application root.

Public hook APIs may evolve as product needs change. Rust source compatibility is
not a goal for this project, but a stale player must still fail loudly rather than
misrepresent a new tree.

The Unity adapter activates only when the running platform implements
`AssistiveSupport`. Otherwise Reactant selects the inspector backend and reports
unavailable health. V1 does not probe for, load, or define an accessibility
plugin ABI.

## Test architecture

### Rust tests

The fake host records the resolved semantic tree and interaction policies beside
normal commands. Black-box tests render public hook examples and assert observable
canonical output rather than hook internals.

Required Rust coverage includes:

- accessible name and description precedence, whitespace, hidden references,
  localization, cycles, and duplicate sources;
- explicit accessibility-key conversion, namespace collision, and distinct
  domain-newtype identity;
- role/state/value/relationship validation;
- stable IDs through keyed reorder, conditional components, unchanged portal
  targets, and reconnect snapshots, plus new IDs after portal-target change;
- changed node incarnation after keyed removal/recreation in one backend
  generation;
- transparent nodes, explicit grouping, reading order, landmarks, headings, and
  collection positions;
- nested fixed and keyed virtual-node parentage, order, forward-reference, and
  removal behavior;
- controlled button, toggle, radio, slider, selection, combobox, and overlay
  intents;
- exact virtual semantic action targeting and capture/target/bubble propagation;
- complete text-edit action payloads and authoritative text/selection proposals;
- modality and focus-visible state transitions;
- LTR and RTL composite navigation policies;
- modal stacking, initial focus, dismissal, and logical restoration;
- dynamic validation, live-region coalescing, announcement acknowledgement, and
  reconnect replay rules;
- virtual windows and focus restoration by collection key; and
- semantic removal at presence exit start while the physical object remains.

Property tests generate valid semantic forests and assert that projection is
acyclic, deterministic, and stable under no-op reconciliation. Separate mutation
tests generate one invalid relationship, range, or composition and assert that
the visual commit is also rejected.

### Protocol tests

Rust and C# share golden fixtures for a full snapshot, incremental upsert/remove,
modal-stack update, action event, backend status, announcement, and reconnect.
Fixtures include unknown fields and enum variants to prove a loud failure.
Fixtures include nontraversal relationship sources, virtual parent IDs, text-edit
payloads, and semantic-key namespaces.

Round-trip tests assert structured IDs and locale strings exactly. Ordering tests
prove child-before-parent removals, parent-before-child upserts, generation
checks, and commit barrier placement relative to visual creation and destruction.
They also cover independent commit sequence, backend generation, node
incarnation, callback request, and proposal sequence values.

Cross-language fixtures compare semantic values after parsing. The fixture
normalizer sorts JSON object keys and arrays representing mathematical sets by
their structured ID/action key, preserves arrays whose order is semantic, emits
finite numbers in the repository's shortest round-trippable format, and preserves
strings exactly as Unicode scalar sequences. Raw object field order and
whitespace are not test requirements.

### Unity tests

EditMode tests construct the production semantic manager with a fake backend and
real UI Toolkit panels. They cover:

- frame conversion, scaling, clipping, physical portals, and virtual-node unions;
- staged batch validation and atomic failure;
- Unity callback generation rejection;
- same-generation stale-incarnation rejection before dispatch;
- input, navigation, accessibility focus, and focus-visible correlation;
- focus notification requests, exact-event confirmation, timeout, supersession,
  and screen-reader deactivation;
- admission-backed activate plus controlled drafts for toggle, selection, range,
  text, and dismiss;
- Tab trapping, nested overlays, logical restoration, and removed invokers;
- typeahead and RTL navigation;
- Motion frame invalidation and one notification per frame;
- reconnect activation and a single screen-change notification;
- screen-reader off/on reconstruction with fresh Unity nodes and complete
  hierarchy reassignment; and
- pinned Unity 6000.5 role/state/action mapping, announcement-politeness fallback,
  capability-generation replacement, focus reveal success/failure, and
  strict-capability policy.

Adapter unit tests map every canonical role, state, value, relation, and action to
an `AccessibilityNode` field/event or an explicit fallback class. Tests inspect
the resulting `AccessibilityHierarchy` rather than screen-reader speech. iOS and
Android IL2CPP player smoke tests prove hierarchy activation, Unity callback
dispatch, teardown, and stale-generation behavior.

### Ditto tests

Ditto adds production-backed steps and assertions:

```yaml
- accessibility_assert:
    target: { alias: music-volume }
    role: slider
    name: Music volume
    value_text: 75 percent
- accessibility_action:
    target: { alias: music-volume }
    action: increment
- accessibility_snapshot:
    matches: fixtures/settings-accessibility.json
```

Targets accept object ID, semantic alias, or a role/name query that must resolve
to exactly one node. Assertions cover role, name, description, state, value,
relationships, actions, order, position, set size, focus, modal/inert state,
backend fallback, and announcements.

`accessibility_action` enters through the same callback adapter as a Unity
`AccessibilityNode` action. It must not call Rust handlers directly. Existing click,
key, controller, drag, wait, and screenshot steps remain the way to test ordinary
input and visual focus styling.

Semantic settle requires no pending Reactant work, no pending safe-gate batch, no
unacknowledged inspector-backend notification, and two quiet frames. Snapshot
normalization removes Unity node IDs and frames unless a scenario asks
for geometry.

Ditto does not assert spoken phrases or replace manual assistive-technology
testing. It proves the canonical tree and action plumbing that the Unity adapter
consumes.

### Performance tests

Phase 0 checks in a machine manifest for one macOS and one Windows release runner.
It records CPU, memory, OS build, power mode, Unity build options, and command.
Benchmarks use a release player, fixed locale, screen reader off, and the inspector
backend so assistive-service latency is excluded.

The fixture contains 1,000 exposed nodes, 100 relationships, 50 controlled
actions, two portals, one modal scope, and a 100-item materialized window whose
declared set size is 10,000. After 200 warmup commits, the runner measures 2,000
commits each for no-op, one-node value update, 100-node reorder, portal-subtree
reorder under one target, and window replacement. It reports median, p95, maximum,
allocation count, and wire
bytes separately for Rust projection and Unity batch application.

The completion threshold is p95 below 1 ms for each side on both manifest
machines, no allocation growth across no-op commits, and mutation/wire work
proportional to the materialized window. Any machine or fixture change resets the
recorded baseline and requires explicit review rather than comparison to an
unidentified host.

## Acceptance scenarios

Every scenario below must pass exactly at the Rust fake-host level, through the
Unity canonical mirror, and through Ditto where applicable. In scenario prose,
“exposes” describes the canonical Reactant result unless a Unity node is named
explicitly. The iOS and Android adapter tests assert the direct or best-effort
mapping defined by the Unity capability table, including an explicit fallback
for every canonical field Unity cannot publish. Manual VoiceOver and TalkBack
testing verifies the resulting Unity-supported subset; it is not expected to
recover semantics absent from `UnityEngine.Accessibility`.

### Labeled controls

Given a custom-painted save button labelled by visible text and described by
hidden help text, the canonical tree exposes one button with name “Save changes”
and the help description. Unity publishes the name as `label`, the description as
`hint`, and the role as `Button`. Pointer, Space, Enter, controller submit, and
accessibility activate each produce one logical press. Disabled state keeps the
control readable and suppresses all activation.

The hidden help declaration is a nontraversal relationship source. The Unity
adapter flattens the resolved text into `hint`, reports the relation adaptation,
and does not expose a ghost text node.

A checkbox labelled by another logical node exposes checked, required, invalid,
and error-message relations independently. Reordering the visual label and
control through a portal does not change their relationship or reading order.

### Modal dialogs

Opening a portaled settings dialog promotes it as the active semantic screen,
makes the page below it inert, places focus at the declared first control, and
keeps Tab/controller navigation inside. Escape, controller cancel, Unity
accessibility dismiss, and an authored close button each request the same Rust
close intent once.

Closing removes dialog semantics before exit animation, then restores focus to
the logical invoker. A nested confirmation dialog restores to the settings dialog
first. Removing the invoker while open uses the documented fallback chain.

### Tabs

A labelled horizontal settings tab list exposes selected state, collection
position, and tab-to-panel relations. In LTR, Right advances; in RTL, Left
advances visually. Home and End choose declared first and last. Automatic mode
selects on focus; manual mode waits for activation. Only the selected panel is
exposed, including while a deselected panel animates out.

### Sliders

A custom music-volume slider exposes numeric range and localized percent text.
Arrows, Page keys, controller direction, accessibility increment/decrement,
pointer drag, and touch drag produce controlled value proposals. The Unity value
does not remain changed if Rust rejects a proposal. RTL reverses horizontal
spatial increment direction but not minimum, maximum, or value meaning.

### Input rebinding

Opening the rebind dialog announces its concise instructions and moves focus to
the capture control. The next eligible keyboard or controller input becomes a
binding proposal rather than activating another control. Escape/controller cancel
remains available. A conflict updates invalid state, description, and a polite
status announcement without moving focus. Successful binding updates the
button's accessible name with the new current key.

### Dynamic validation

Submitting a form with an invalid required text field exposes invalid and
error-message state, announces one concise summary, and focuses the first invalid
field by declared logical order. Correcting the value removes invalid state
without announcing the entire form. A repeated equivalent error is deduplicated.

### Announcements

Rapid music-playback status updates with the same polite key coalesce within one
frame. An assertive connection-loss message is delivered in commit order and
acknowledged. Unity receives the resolved text in submission order, while the
inspector reports that polite/assertive delivery was not representable. A
reconnect does not replay acknowledged speech and replays an unacknowledged
assertive message at most once.

### Portaled overlays

A combobox popup rendered into an external portal remains controlled by and
logically related to its field. Arrow navigation updates active descendant,
Enter selects, Escape closes, and accessibility focus returns to the field.
Physical event subscriptions still reach the logical handler chain.

### Virtualized collections

A listbox showing items 101 through 120 of 10,000 exposes one-based positions and
the full set size. Moving past item 120 requests the next window, applies it at
the safe gate, notifies layout change, and restores focus to the intended item by
key. Reorder or filtering never transfers selection or focus to a row merely
because it reused the same visual index.

### RTL interfaces

Arabic locale text retains language metadata through a portal. Horizontal tabs,
menus, listboxes with spatial navigation, and sliders mirror their directional
behavior. Vertical navigation, Home/End collection meaning, reading order, and
numeric value meaning remain stable. Typeahead matches locale-aware names.

### Reconnects

Disconnecting and reconnecting while a modal dialog and virtual collection are
open recreates stable canonical IDs, modal inertness, selection, and eligible
focus. Backend generations change, stale callbacks are rejected, and the adapter
emits one screen-change notification without replaying ordinary live-region
history.

Turning the screen reader off and on without a Rust render also changes the
backend generation. Reactant rebuilds fresh Unity nodes from the retained mirror,
reassigns one complete active hierarchy, rejects callbacks from the cleared
hierarchy, and preserves canonical IDs.

### Presence animation

Removing a focused tooltip, tab panel, menu, or dialog makes it semantic-inert in
the removal commit, chooses the documented focus destination, and suppresses
Unity accessibility actions. Its `VisualElement` and Motion animation may remain until exit
completion. Frame updates for still-present nodes remain synchronized without
semantic churn.

### Menus and disclosures

A menu trigger exposes expanded state and a controls relation. Opening a portaled
menu focuses the selected or first enabled item. Arrows, Home, End, localized
typeahead, Enter/submit, Escape/cancel, and Tab follow the declared menu policy.
An RTL submenu opens and closes in mirrored spatial directions. Closing the
deepest submenu restores its parent item; closing the stack restores the trigger.
Disabled menu items remain readable but cannot activate.

A disclosure exposes expanded state and controls exactly one region. Keyboard,
controller, pointer, and accessibility activation each toggle it once. Collapsing
while focus is inside moves focus to the disclosure before hiding the region.

### Text editing and combobox input

A labelled required text field exposes text value, selection/editability,
description, and validation without duplicating its visible placeholder as a
name. Read-only text remains focusable and selectable; disabled text does not
edit. UI Toolkit insert, delete, selection, and paste events produce correlated
controlled proposals. Ditto may exercise the canonical accessibility set-value
action. Unity v1 omits that unsupported action and records its fallback.

In a combobox, text input focus remains on the field while accessibility focus or
active descendant navigates the portaled listbox. Filtering that removes the
active option selects the nearest eligible keyed option without committing it.
Escape restores pre-open text; Enter/submit accepts the active option once.

### Tables, grids, and trees

A reading-only table exposes its caption/name, row and column count, headers, and
cell-to-header relationships without adding Tab stops. A grid with the same
visual hosts opts into two-dimensional navigation, has one Tab entry point, skips
hidden rows, and preserves focus by row/column keys through sorting.

A tree exposes level, expanded state, position, and set size. Right/Left expand,
collapse, or move between parent and child according to direction-independent
tree conventions; vertical arrows move by visible logical order. Collapsing an
ancestor containing focus moves focus to that ancestor before child semantics are
removed.

### Tooltips, progress, and custom actions

A keyboard-focused icon button gains a delayed tooltip description without
moving focus or creating an extra Tab stop. Escape hides it. Touch discovery uses
the declared help action. The tooltip cannot contain an actionable descendant.

A determinate progress indicator exposes its named numeric range and formatted
value without announcing every frame. An indeterminate indicator exposes busy
state and no false percentage. Explicit milestones announce once according to
their deduplication policy.

A custom action with a localized name remains available to canonical inspection
and Ditto but is omitted from Unity's v1 hierarchy with a custom-action fallback.
Invoking it through Ditto routes one typed action to its logical owner. Removing
or renaming the action invalidates stale test callbacks by generation.

### Focus-kind divergence

Screen-reader reading moves accessibility focus across a heading and static text
without moving UI Toolkit input focus. Activating the following button moves
input focus only because its focus props request correlation. Pointer focus does
not show a focus-visible ring; keyboard, controller, and accessibility focus do.
The diagnostics inspector shows all four focus/modality values throughout.

### Backend fallback and validation recovery

Against a pinned Unity capability fixture missing a rich role, an allowing node
uses the documented Unity fallback, preserves every supported name/hint/value
field, omits unsupported actions, and records the exact degraded diagnostics. The
same node with one of those fallback classes forbidden rejects the complete
candidate visual/semantic commit and leaves the previous complete screen active.

An invalid labelled-by cycle and an actionable clipped node without a scroll
route likewise reject the whole candidate. Correcting the declaration on the next
render commits both visual and semantic changes together and clears the active
error without restarting the session.

### Switch control and alternate actions

With iOS Switch Control or Android Switch Access, scanning follows the hierarchy
and order Unity publishes and skips nodes Reactant marks inactive. Select,
increment/decrement, and dismiss work when Unity exposes the corresponding node
event. Custom actions and active-member composite entry remain canonical-only and
produce fallbacks. Opening a modal publishes the updated active hierarchy;
closing it restores the previous keyed focus target when Unity permits it.

### Identity, capability, and reveal failures

Removing and recreating the same keyed virtual item in one backend generation
changes its node incarnation. A callback holding the removed ID is rejected and
cannot select the replacement.

Two actionable virtual items owned by one physical host route to different
slot/key handlers and retain their exact semantic target through capture and
bubble. Changing a portal target remounts its subtree with new host and semantic
IDs; reconnect rebinding with the target unchanged preserves them.

When the Unity capability set changes, Unity deactivates the active hierarchy, creates
a new backend generation, and asks Rust to reclassify the last complete commit.
An allowed mapping republishes once; a missing static requirement stays
unavailable; a newly forbidden fallback leaves the previous complete Reactant
commit intact and publishes no misleading Unity hierarchy.

Requesting accessibility focus on a clipped node with a valid reveal route keeps
old focus until geometry becomes visible and Unity reports an exact focus event
for the requested node. A rejected reveal, missing target, two-frame/250 ms reveal
deadline, one-second Unity-focus deadline, focus on another node, or
screen-reader deactivation preserves the last confirmed focus, returns the
documented failure, and creates no `Focused` event for the target.

Turning the screen reader off clears the active Unity hierarchy and cancels any
pending focus request. Turning it on creates a new backend generation, rebuilds
fresh Unity nodes from the last canonical mirror, and assigns the complete
hierarchy without requiring a Rust rerender. Canonical IDs remain stable, while
callbacks from the prior Unity generation are rejected.

### Scenario-to-platform coverage

Canonical automation runs in Rust, protocol tests, the Unity Editor, and Ditto.
The `UnityAccessibilityBackend` mapping suite runs against the pinned Unity API.
Only iOS and Android require player and manual assistive-technology evidence in
v1. Other player targets must report `Unavailable` and must not claim a semantic
tree was published.

| Scenario family | Canonical automation | Unity adapter | Manual AT |
| --- | --- | --- | --- |
| Labels, controls, validation | Required | Required mapping/fallback | iOS and Android |
| Dialogs, overlays, presence | Required | Required mapping/fallback | iOS and Android |
| Tabs, menus, disclosures | Required | Required mapping/fallback | iOS and Android |
| Sliders and progress | Required | Required mapping/fallback | iOS and Android |
| Text input, rebinding, combobox | Required | Required mapping/fallback | iOS and Android |
| Announcements, reconnect, and screen-reader reactivation | Required | Required mapping/fallback | iOS and Android |
| Virtual listbox, table, grid, tree | Required | Required mapping/fallback | iOS and Android |
| RTL and localization | Required | Required mapping/fallback | iOS and Android |
| Focus-kind divergence | Required | Required mapping/fallback | iOS and Android |
| Fallback and validation recovery | Required | Forced capability fixtures | Representative iOS and Android cases |
| Stale callback and off/on hierarchy replacement | Required | iOS and Android player smoke tests | iOS and Android |

The required evidence by test layer is:

| Scenario family | Rust | Protocol | Unity mirror | Ditto | Unity hierarchy | Manual AT |
| --- | --- | --- | --- | --- | --- | --- |
| Labels/control state | Required | Required | Required | Required | Required | Required |
| Dialog/portal/presence | Required | Required | Required | Required | Required | Required subset |
| Composite navigation | Required | Required | Required | Required | Required | Required subset |
| Range/text proposals | Required | Required | Required | Required | Required | Required subset |
| Collections/virtualization | Required | Required | Required | Required | Required fallback | Required subset |
| Live announcements | Required | Required | Required | Required | Required | Required |
| RTL/localization | Required | Required | Required | Required | Required fallback | Required subset |
| Validation/fallback | Required | Required | Required | Required | Required | Representative fallback |
| Identity/reconnect | Required | Required | Required | Required | Required | Required |
| Focus reveal/failure | Required | Required | Required | Required | Required | Required subset |
| Switch scanning/actions | Required | Required | Required | Required | Required subset | iOS and Android |

Every required cell produces a named test result or manual record linked from the
release checklist. A scenario may share setup with another, but no cell is
satisfied by inference from a different layer.

## Phased implementation plan

Each phase ends in an independently reviewable commit series during
implementation. Later phases may add mappings but must not redefine the canonical
semantics established in Phase 1.

### Phase 0: Unity accessibility fixtures and player spikes

Purpose: measure the pinned `UnityEngine.Accessibility` surface before its
best-effort mapping tables harden.

Tasks:

- Create a small canonical fixture covering a labelled button, checkbox, slider,
  tabs, modal dialog, listbox, live region, and one virtual item.
- Build disposable iOS and Android players that expose the fixture exclusively
  through `AccessibilityHierarchy` and `AccessibilityNode`.
- Verify label, hint, value, role, state, action events, focus, frame getters,
  notification, screen-reader status, replacement, and teardown behavior with
  Unity's inspector plus VoiceOver and TalkBack.
- Verify that layout and screen notifications request focus without synchronously
  confirming it, and record the exact focus callback that confirms the request.
- Verify that screen-reader off/on clears, reconstructs, and reassigns the active
  hierarchy without a Reactant rerender or reuse of old Unity nodes.
- Record the actual Unity mappings, minimum player requirements, callback-thread
  behavior, and unsupported concepts in checked-in mapping tables.
- Verify that macOS, Windows, Linux, console, and WebGL players select the
  inspector backend and report `Unavailable` without calling an unsupported
  `AssistiveSupport` API.
- Confirm that no target loads a custom accessibility plugin or creates a WebGL
  semantic DOM.
- Record exact benchmark hardware, OS, power mode, player build options, fixture
  generator, warmup, sample count, and invocation in a checked-in performance
  manifest.

Exit criteria:

- iOS and Android can expose and activate every fixture action Unity supports
  without duplicate nodes;
- Unity callback threading and hierarchy activation are proven;
- focus request/confirmation and screen-reader off/on reconstruction are proven;
- relationships, active descendant, virtual continuation, promoted modal roots,
  synchronous actions, live announcements, and animated geometry each have an
  explicit Unity mapping or fallback decision;
- unsupported player targets deterministically report `Unavailable`; and
- the reproducible performance manifest names every completion-gate machine and
  command.

### Phase 1: Canonical Rust model and validation

Purpose: establish semantics independent of Unity and individual widgets.

Tasks:

- Add IDs, roles, name/description sources, state, values, relations, actions,
  collection metadata, visibility, locale, direction, and source diagnostics.
- Add the complete declarations for content metadata, focus scopes, presentation
  promotion, fallback policy, geometry source, and finite interaction policies,
  even where runtime behavior lands in later phases.
- Add host and virtual-slot declaration APIs plus typed relationship refs.
- Project a candidate semantic tree from the logical Reactant tree, preserving
  portal ancestry and transparent nodes.
- Implement canonical name/description computation and validation.
- Add modal/inert projection, reading order, heading/landmark rules, and
  presence-removal semantics.
- Store the committed semantic tree and diff complete-node upserts/removals.
- Extend the fake host and add property, invalid-composition, portal, presence,
  and reconnect tests.

Exit criteria:

- the fake host produces a deterministic standalone semantic snapshot;
- every invalid identity, role/state/content, name, relation, visibility,
  ancestry, reading-order, and fallback-policy composition fails before visual
  commit;
- keyed nodes retain IDs through reorder and an unchanged portal target, while a
  portal-target change remounts with new IDs; and
- semantic presence removal and reconnect snapshot identity pass without Unity;
  no focus restoration is required in this phase.

### Phase 2: Protocol, Unity mirror, and inspector backend

Purpose: carry the canonical model into the existing atomic response lifecycle.

Tasks:

- Add snapshot, mutation, policy, focus, notification, announcement, event, and
  capability messages to Rust and C# protocol types.
- Add shared golden fixtures and unknown-schema failure tests.
- Generate the pure Rust/C# fallback classifier from Phase 0 mapping tables and
  implement immutable capability generations and reclassification.
- Implement `BattlementAccessibilityManager`, its indexes, generation checks,
  staged validation, commit barriers, and inspector backend.
- Resolve UI Toolkit host geometry, clipping, virtual geometry, and Motion frame
  invalidation.
- Integrate semantic deactivation with host destruction and presence retention.
- Enforce attachment, computed exposure, geometry-source, scroll-route, commit
  sequence, and node-incarnation validation.
- Add the editor Accessibility inspector and strict/degraded health reporting.
- Implement reconnect tree activation and announcement acknowledgement, leaving
  operational focus restoration to Phase 3.

Exit criteria:

- a real UI Toolkit panel is reflected exactly in the inspector backend;
- visual and semantic mutations fail or commit together;
- Motion, portal, reconnect reconstruction, and stale-callback Unity tests pass;
  and
- all protocol fixtures round-trip to semantic equality through the shared
  canonical fixture normalizer.

### Phase 3: Synchronous interaction and focus primitives

Purpose: create the reusable behavior layer before high-level pattern hooks.

Tasks:

- Implement modality and `use_focus_visible`.
- Implement `FocusProps`, focus correlation, queued focus commands, restoration,
  and focus-scope infrastructure.
- Implement layout/screen focus requests, exact Unity-event confirmation,
  supersession, one-second timeout, and screen-reader-deactivation failure.
- Implement Unity navigation policies for Tab, arrows, Home/End, Page keys,
  controller direction, submit, and cancel.
- Implement admission-backed `use_press` activation, controlled
  toggle/select/range proposals, hover, long press, drag, typeahead, and active
  descendant.
- Route accessibility actions through committed logical event ancestry.
- Integrate `input_disabled` and rebind capture policy.
- Complete reconnect input/accessibility focus restoration on top of the Phase 2
  tree activation.
- Enforce focus-scope nesting, autofocus arbitration, action ownership,
  proposal-value, callback cancellation, and reveal-state validation.
- Add LTR, RTL, duplicate-activation, rejected-proposal, and raw-event ownership
  tests.

Exit criteria:

- all standard defaults in the ownership table execute synchronously in Unity;
- Rust receives one typed intent per physical or supported Unity action;
- no public API implies late default cancellation; and
- focus-visible state agrees across pointer, touch, keyboard, controller,
  accessibility, and programmatic transitions.

### Phase 4: Core control hooks

Purpose: deliver common controls on top of the stable primitives.

Tasks:

- Implement state adapters and hooks for buttons, links, toggle buttons,
  checkboxes, switches, radios, sliders, text/search fields, validation, progress,
  headings, landmarks, images, groups, and separators.
- Implement native-control adapters without making native hosts mandatory.
- Add black-box Rust examples for a native host and a custom visual host producing
  equivalent semantics and behavior.
- Add localized range text, read-only/required/invalid behavior, and error
  relationships.
- Implement `use_live_region`, `use_announce`, custom-action admission, queue
  state/acknowledgement, deduplication, failure, and reconnect replay behavior.
- Add Ditto semantic assertion and action primitives backed by the inspector
  backend.

Exit criteria:

- labelled controls, sliders, text-field validation, progress, custom-action,
  announcement-queue, and primitive LTR/RTL scenarios pass in automated layers;
- a standalone rebind capture policy works without requiring a dialog, and
  semantic reconnect works without asserting overlay restoration;
- custom and native visual implementations produce equivalent canonical trees;
  and
- the public examples compile as documentation tests.

### Phase 5: Collections, composites, and overlays

Purpose: implement patterns whose correctness depends on relationships, focus
scopes, and keyed data.

Tasks:

- Implement collection state, item keys, selection state, roving focus,
  virtualization metadata, and continuation actions.
- Implement tabs, listboxes, comboboxes, menus/submenus, tooltips, disclosures,
  tables, grids, and trees.
- Implement dialogs, alert dialogs, overlay-stack inertness, initial focus,
  dismissal, nested restoration, and portaled relationships.
- Enforce tab/panel, group/item, active-descendant, table/header, tree-level,
  collection-window, overlay, and composite-member validation.
- Extend Ditto with semantic snapshots, role/name queries, focus, relation,
  modal/inert, collection, and announcement assertions.
- Add high-volume performance fixtures for collection diffing and geometry.

Exit criteria:

- dialog, tabs, menus, disclosures, combobox, input-rebinding dialog, tooltip,
  table, grid, tree, portaled overlay, presence, focus-divergence, and virtualized
  collection acceptance scenarios pass through Ditto;
- a 10,000-item virtual collection emits work proportional to the materialized
  window and changed metadata; and
- nested overlay closure restores focus correctly under invoker removal and
  presence animation.

### Phase 6: Unity accessibility backend

Purpose: expose the proven canonical model through Unity's supported
assistive-technology integration.

Tasks:

- Implement `UnityAccessibilityBackend` with `AccessibilityHierarchy`,
  `AccessibilityNode`, `AssistiveSupport`, and Unity's notification dispatcher.
- Add iOS and Android IL2CPP build, hierarchy activation, replacement, teardown,
  callback, and capability-health tests.
- Rebuild and reassign the complete hierarchy with fresh nodes after a
  screen-reader off/on transition, without requiring a Rust rerender.
- Implement the generated Unity mapping tables, best-effort fallbacks,
  notification coalescing, and strict-profile enforcement.
- Map the complete pinned Unity 6000.5 role, state, action, focus, and notification
  surface, including a diagnosed live-announcement fallback for lost politeness.
- Add unsupported-player tests proving deterministic inspector selection and
  `Unavailable` health without platform API calls.
- Validate Unity's minimum mobile OS versions and document deployment
  requirements.

Exit criteria:

- every cell marked Full or Adapted in the Unity capability matrix behaves as
  specified or records the exact documented fallback;
- the active Unity hierarchy contains no duplicate Reactant nodes;
- screen-reader reactivation publishes the retained canonical mirror through a
  new backend generation and rejects old-generation callbacks;
- Unity's hierarchy inspector matches the generated mapping fixture; and
- all Unity node action callbacks use the production controlled/default-action
  path.

### Phase 7: Product adoption and assistive-technology certification

Purpose: apply the subsystem to real Reactant surfaces and close gaps that only a
screen reader reveals.

Tasks:

- Migrate representative Battlement settings, input rebinding, overlays, tabs,
  sliders, validation, and dynamic status surfaces.
- Enable `AccessibilityCoverage::Required` at migrated roots.
- Run the Unity-supported acceptance subset with VoiceOver and TalkBack.
- Test keyboard-only and controller-only use with screen readers off and on.
- Test large text, bold text, reduced motion, high contrast where supported,
  orientation changes, locale changes, and RTL layouts.
- Record results by exact OS, player, and screen-reader version; file all
  divergences as product bugs rather than undocumented exceptions.
- Measure semantic commit, Unity hierarchy, geometry, and virtual-window latency
  under release builds.

Exit criteria:

- every applicable Unity-supported acceptance scenario passes manually on iOS
  and Android;
- no release-blocking inspector or screen-reader issue remains;
- required roots contain no unlabeled actionable nodes or undocumented fallback;
  and
- performance stays within the completion thresholds below.

## Delivery risks and mitigations

### Unity mappings differ by mobile platform and engine version

The same Unity role, state, or notification can produce different VoiceOver and
TalkBack behavior, and Unity may expand or change its mapping between engine
versions. Phase 0 measures the pinned version. Canonical tests remain stable,
while adapter fixtures and the manual mobile matrix record the observed Unity
behavior and every intentional fallback.

### Unity callbacks contend with Reactant dispatch

Unity accessibility events run on the engine thread and may arrive while other
UI work is settling. The adapter rejects recursive dispatch, validates generation
and incarnation before invoking Rust, and admits resulting mutations through the
existing safe gate. It does not add an off-thread native-provider bridge.

### Accessibility and input focus can diverge

Conflating them would make static reading move keyboard focus and make modal
restoration unpredictable. The manager stores both, applies explicit correlation
rules, and shows both in diagnostics. Acceptance tests cover their divergence.

### Controlled state can cause misleading assistive feedback

Unity or the operating-system service may announce a local value before Rust
accepts it. The controlled proposal
adapter restores committed state and sends a corrective value notification only
when needed. Tests cover accepted, rejected, delayed-safe-gate, and disconnected
responses.

### Geometry churn can overwhelm Unity accessibility

Motion and scroll can change many frames each update. Live getters, dirty-owner
tracking, one notification per root per frame, and windowed collections bound the
work. Release measurements gate completion.

### A rich canonical tree can hide degraded output

Capability negotiation, per-node fallback diagnostics, strict profiles, and
Unity hierarchy tests keep degradation visible. Fallback never changes the
Rust canonical snapshot, so tests can distinguish author errors from backend
limitations.

### Screen-reader and Unity behavior change outside the repository

Automation cannot guarantee spoken order, rotor behavior, gesture conventions,
or verbosity. The release matrix records exact external versions and manual
results. A supported Unity or operating-system version change requires targeted
recertification, not a claim that old results still apply.

## Rejected alternatives

### Copy DOM attributes into Rust

Strings such as `aria-*` would defer validation, invite impossible state/role
combinations, and pretend all targets use the browser accessibility model. Typed
semantic values and relations retain intent and allow richer Unity mappings as
the engine evolves.

### Port React Aria Components

A component layer would impose host structure and styling, conflict with native
UI Toolkit controls, and prevent arbitrary Battlement visuals from reusing the
behavior. Reactant adopts the lower-level separation of state and accessible
behavior instead.

### Infer semantics from UI Toolkit control types or visible text

Native control classes are implementation choices and custom hosts are common.
Visible text can be decorative, incomplete, duplicated, or in another portal.
Inference would produce unstable names and inconsistent custom/native output.

### Use the physical UI Toolkit tree

Portals and external targets intentionally change physical parentage without
changing logical ownership, context, or event propagation. Physical semantics
would disconnect labels, triggers, overlays, and focus restoration.

### Treat focus navigation as the accessibility tree

Headings, static text, groups, landmarks, descriptions, and disabled controls may
be readable without being in the Tab sequence. Conversely, a focusable canvas
proxy without a role and name is not accessible. The two projections integrate
through explicit contracts but remain distinct.

### Ask Rust to cancel every Unity default

The current event reaches Rust after UI Toolkit default behavior. A remote
prevent-default flag would be too late and race Unity accessibility callbacks.
Finite declared Unity policies make required defaults synchronous and observable.

### Keep exiting elements accessible until Motion completes

Logical removal means an action can no longer be handled reliably. Leaving an
exiting dialog, menu, or panel in the Unity hierarchy creates ghost focus and
stale actions. Semantic lifetime ends at logical removal while visual lifetime
may continue.

### Reduce the public model to Unity's current subset

Unity's current role/state surface cannot represent Reactant's relationships,
collections, rich widgets, or future targets. Reducing the canonical model to
that subset would discard author intent and force application migrations whenever
Unity adds capability. A rich model plus observable best-effort lowering keeps
applications future-ready without claiming that v1 publishes every declaration.

### Add custom native accessibility plugins in v1

Direct UIKit, Android provider, NSAccessibility, and UI Automation integrations
would multiply lifecycle, threading, build, and certification surfaces while
duplicating work Unity may later provide. V1 depends only on Unity's public
accessibility API. A future design may reconsider native plugins using measured
product requirements, but this protocol and implementation do not reserve or
load them.

### Create a WebGL DOM or ARIA mirror in v1

A browser semantic mirror would require DOM ownership, focus synchronization,
geometry overlays, input forwarding, hosting integration, and browser-specific
certification outside Unity's accessibility contract. V1 reports accessibility
as unavailable on WebGL and does not emit arbitrary DOM or ARIA nodes.

### Validate only with screenshots or semantic snapshots

Those tests cannot prove spoken output, gesture navigation, rotor behavior,
focus handoff, or Unity hierarchy behavior on a device. They are necessary
regression layers, not substitutes for assistive-technology validation.

## Completion criteria

The subsystem is complete when all of the following are true:

- public APIs preserve the five-layer separation and compile the button,
  checkbox, slider, tabs, and dialog examples in this design;
- every listed common pattern has typed semantics, interactions, focus policy,
  state integration, diagnostics, and developer-authored rendering examples;
- the canonical tree covers every role, name, description, state, value,
  relationship, order, grouping, collection, visibility, action, locale, and live
  behavior specified here;
- visual and semantic candidates validate and commit atomically;
- portals preserve logical semantics, presence removes semantics at logical exit,
  and reconnect restores stable IDs and valid focus;
- Unity executes all supported accessibility callback and standard navigation
  defaults synchronously from Rust-declared finite policies;
- Rust receives exactly one typed logical intent for every standard input and
  supported Unity accessibility action;
- the Unity backend passes mapping, hierarchy, lifecycle, and iOS/Android player
  tests, while unsupported players report `Unavailable`;
- Ditto can inspect and act on the production semantic mirror without calling
  Rust handlers directly;
- every acceptance scenario passes in Rust, Unity, protocol, and Ditto layers
  where applicable;
- every Unity-supported acceptance scenario has a recorded manual result on iOS
  and Android;
- a 10,000-item virtual collection performs semantic work proportional to the
  visible window, not the total item count;
- semantic projection and Unity batch application each remain below 1 ms at the
  95th percentile for 1,000 exposed nodes on supported release hardware, excluding
  assistive-service time;
- a Motion frame emits at most one coalesced layout notification per affected
  semantic root;
- strict roots contain no unlabeled actionable nodes, invalid compositions,
  unapproved fallback, duplicate Unity nodes, or unavailable required backend;
  and
- repository CI, target build smoke tests, and the manual matrix are green for
  the release candidate.

## Manual QA

Automation establishes the canonical contract but cannot certify the user
experience. Run these checks on release player builds, not only in the Unity
editor.

Record for every run:

- commit, Unity version, OS/device, backend kind, locale,
  layout direction, and assistive-technology name/version;
- whether an external keyboard, controller, touch, pointer, or switch-control
  device was used; and
- spoken output, focus order, action result, visual focus indicator, and any
  backend diagnostic.

Use this target matrix:

| Target | Assistive technology | Unity/platform inspection |
| --- | --- | --- |
| iOS device | VoiceOver | Accessibility Inspector |
| Android API 26+ device | TalkBack | Layout Inspector/Accessibility Scanner |

For each target:

1. Navigate the complete migrated surface using only the screen reader's next,
   previous, and Unity-exposed action mechanisms.
2. Repeat with keyboard only, controller only, and screen reader plus keyboard or
   controller.
3. Execute every applicable Unity-supported acceptance scenario plus
   representative fallback, rejection, and removed-target branches.
4. Confirm names are concise, descriptions are not repeated, role and state are
   understandable, and dynamic announcements are neither missing nor noisy.
5. Confirm visual focus matches keyboard/controller/accessibility modality and
   remains visible under clipping, scroll, scale, and Motion.
6. Enable the platform's largest practical text setting, bold text, reduced
   motion, and high contrast/increase contrast where supported.
7. Repeat the settings, tabs, slider, menu, and dialog paths in at least one RTL
   locale and one non-English LTR locale.
8. Reconnect during an open nested dialog, active rebind capture, slider change,
   and virtual collection navigation.
9. Leave a screen open through live updates for five minutes and verify that
   announcements remain meaningful and focus does not jump.
10. Inspect Unity's published hierarchy after every portal open/close and
    presence exit to confirm there are no duplicate, ghost, or stale nodes.

A mismatch between canonical inspection and Unity's supported spoken behavior is
an adapter bug even when automated tests pass. A mismatch between the intended
user behavior and the canonical tree is a Rust hook or application-composition
bug. Release is blocked until each mismatch has an owner, a regression test at
the lowest useful layer, and a passing manual retest.
