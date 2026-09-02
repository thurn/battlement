# Collection and landmark accessibility

Reactant exposes host-backed single-selection listboxes, tables, links,
navigation landmarks, content regions, and current-page state. These compose
[semantic declarations](accessibility-technical-design.md) with ordinary focus
and interaction properties. Application handlers own navigation and selection.

## Declarations and validation

`accessibility_collections` contains the structural and actionable hooks.
All structural hooks return `SemanticProps`; they add no input focus or actions.

| Hook | Required meaning | Logical semantic children |
| --- | --- | --- |
| `use_listbox(name)` | Named single-selection list | Options |
| `use_option(options)` | Named, selected, optionally disabled choice | Ordinary contents |
| `use_table(name)` | Named table | Rows |
| `use_row()` | Table row | Cells and scoped headers |
| `use_column_header(name)` | Header scoped to its column | Ordinary contents |
| `use_row_header(name)` | Header scoped to its row | Ordinary contents |
| `use_cell(name)` | Named data cell | Ordinary contents |
| `use_link(options)` | Named activation target | Ordinary contents |
| `use_navigation(name)` | Named navigation landmark | Ordinary contents |
| `use_region(name)` | Named content landmark | Ordinary contents |

Each option belongs to a listbox, each row belongs to a table, and each cell or
header belongs to a row. Transparent layout hosts do not change relationships;
logical semantic ancestry remains authoritative across portals. Listboxes permit
zero or one selected option. Disabled options remain readable and retain their
actions, but activation has no effect.

`use_option` accepts the existing `ChoiceOptions` controlled props and returns
`AccessibleBehavior<G, bool>`. Arrow, Home, End, typeahead, and queued ref focus
remain application handlers. It installs no roving focus or selection-on-focus
policy. `use_link` accepts `ButtonOptions`; its application callback owns URL
activation. The semantic layer does not open external URLs.

`SemanticState.current` accepts only `CurrentPage::Page`, only on a button or
link. At most one descendant of a navigation landmark may be current. The
application determines whether a navigation has a current page. Current-page
state is independent of selection and input focus.

Invalid declarations panic before committing visual or semantic changes.
Semantic snapshots carry exact roles, state, and host ancestry to Unity and
Ditto. No virtual semantic identities or independent focus state are created.

## Unity presentation

The pinned Unity 6000.5.8f1 accessibility API supports macOS and Windows players
as well as iOS and Android. The backend enables all four player platforms and
publishes only while a screen reader is enabled.

Unity's role enum has no listbox, option, table, scoped cell, link, or landmark
roles. It also has no current-page state. The canonical mirror preserves those
concepts exactly; native presentation uses the documented unsupported-role
mapping below. Native collection-specific navigation, table-coordinate APIs,
and a native link trait are not supplied by Unity's public API.

| Canonical concept | Unity presentation |
| --- | --- |
| Listbox, table, row, navigation, region | Container, logical descendants, role label |
| Option, cell, scoped header, link | None role, role label, declared callbacks |
| Option selected/disabled | Native selected/disabled state |
| Current page | Label suffix `current page`, independent of selected state |

Native labels append role information and current-page state to the canonical
name, so that meaning remains audible with spoken hints disabled. Data-cell
labels include their row and column header text. Authored descriptions remain
separate hints. Canonical names remain unchanged for application targeting.
Rows and cells retain their hierarchy and reading order. Roles without an exact
native trait remain identified in their label, as recommended by Unity.
This presentation does not claim native table or landmark navigation support.

- [Unity accessibility platforms](https://docs.unity3d.com/6000.5/Documentation/Manual/accessibility/module-intro.html)
- [Unity role mapping guidance](https://docs.unity3d.com/6000.5/Documentation/ScriptReference/Accessibility.AccessibilityRole.html)

## Validation

Public runtime tests inspect complete committed snapshots, controlled option
activation, disabled rejection, link callbacks, and invalid structures.
Unity fixtures pin role/label mapping and current-page transport. The Layout
Gallery's collection specimen exercises public hooks through the production
host. Ditto semantic assertions can check selected, disabled, current-page,
and parent role/name in addition to a target's role/name.

Screen-reader validation must inspect the real player: traverse each collection,
activate an option and link, verify selected and disabled options, read scoped
headers and cells, find both landmarks, and distinguish the current page from
an ordinary navigation button. Canonical assertions alone do not establish
what VoiceOver or TalkBack announce.
