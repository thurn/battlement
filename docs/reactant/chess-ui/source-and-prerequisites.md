# Source and prerequisites

[Plan and reading guide](../chess-ui-implementation-plan.md)

## Related Information

The authoritative TypeScript source is
`git@github.com:thurn/mockups.git` at commit
`2451ea9cc6f76b356b1102ee37b82c478853122a`. The existing checkout is
`/Users/dthurn/Documents/mockups`. That checkout remains unchanged while
reference states and screenshots are captured.

The reference uses Node 22.13 or newer, React 19.2.6, TypeScript 5.9.3,
Framer Motion 13.1.1, and Vinext 1.0.0-beta.3. Before the first capture, run:

```text
npm install
npm run format:check
npm run build
npm run dev
```

The pinned commit must build without source changes. Dependency installation
and generated build output do not become port inputs.

Implementation depends on the certified implementations of these designs:

- [Layout and stacking](../layout-and-stacking.md)
- [Focus and navigation](../focus-and-navigation.md)
- [Shared components](../shared-components.md)
- [Shared components](../shared-components.md)
- [Events and default actions](../events-and-default-actions.md)
- [Asset generator](../asset-generator.md#authoring-api)
- [Mockup animation coverage](../animations.md#mockup-translation-coverage)

The events and default-actions work is a transitive prerequisite even though it
was not part of the original requested document list. Sliders, listboxes,
modals, tabs, and input rebinding cannot preserve native default behavior
without it.

The current accessibility design deliberately stops before listboxes, tables,
links, landmarks, and current-page state. The chess UI demonstrates a product
need for those semantics. Before Task 1 begins, a focused accessibility
extension must be designed, implemented, and certified. It adds only these
host-backed patterns:

- listbox and option semantics for `SelectControl`;
- table, row, header, and cell semantics for the input bindings;
- link semantics whose activation uses the existing external-URL request;
- navigation and region landmarks for the review gallery; and
- current-page state for its selected review-page button.

The extension composes the existing `SemanticProps`, `InteractionProps`, and
ordinary `FocusProps` contracts. It does not add virtual semantic nodes,
programmatic accessibility focus, a roving-focus engine, or a second input
navigation model. It is prerequisite work rather than a forty-first review
page.

The separate extension design may choose its public type names, but it cannot
weaken these behavior contracts:

- A listbox is one host-backed named container. Each logical option descendant
  is a host-backed named option with application-owned selected and disabled
  state. Option activation requests selection through its target-default
  handler. Arrow, Home, End, typeahead, and focus movement remain
  `SelectControl` handlers using queued ref actions.
- A table is one host-backed named container whose logical children are rows.
  Header cells identify their column or row scope, data cells remain in their
  containing row, and logical ancestry supplies the relationships. These nodes
  have no input-focus behavior unless the rendered host is independently
  interactive.
- A link is one named, ordinarily focusable host with an Activate action. The
  Privacy Policy handler issues the existing typed external-URL request; the
  semantic layer does not open URLs itself.
- Navigation and region landmarks are named host-backed containers with no
  actions and no implied input focus.
- Current-page state has the exact value `Page`, is valid on a button or link,
  and appears on exactly one review-page button in the gallery navigation.
- Rust validation, Unity mapping, **Ditto** black-box sample inspection, and
  VoiceOver and TalkBack evidence cover every added role, relationship, state,
  and action. Ditto is the repository's sample scenario runner.

The declarations in `samples/reactant/rules/src/assets.rs` are the source for
the 18 existing generated assets. The
[animation coverage ledger](../animations.md#coverage-ledger) is the source for
motion timing, easing, direction, interruption, seed, and reduced-motion
requirements. This plan uses those as pinned starting evidence instead of
duplicating or rediscovering either design.

**Tollgate** is the repository's validation and exact-promotion service. A
**candidate** is one immutable source commit submitted to it. A **certified
release** is the repository's `release` ref after Tollgate has validated and
promoted that exact candidate. Implementation begins from the first certified
release containing every prerequisite named above. Each later task records its
exact starting release commit in its handoff, because those future commit
identifiers do not exist while this plan is written.

## Research Basis

Three independent research passes audited the mockup, Reactant, and the
dependency order for a port. They reached these conclusions:

- Visual fidelity is feasible after the prerequisite designs are implemented.
- Focus, default actions, accessibility, controller input, and application
  lifecycle are more significant obstacles than static layout.
- Existing generated assets should be copied into the new sample instead of
  being shared with or referenced from the Reactant rules sample.
- Application visibility, external links, and stable semantic test targeting
  are prerequisite Reactant capabilities. Callback props, behavior-bundle
  forwarding, and asset families may still expose authoring friction during the
  port.
- Browser-only diagnostics should not be ported unless they expose a
  generalized requirement that also applies to Unity applications.
- The source contains intentional prototype behavior, including inert actions
  and settings that modify only local state. Fidelity requires preserving those
  behaviors rather than completing the implied product features.

Before the first migration, compile and runtime probes verify that the certified
release supplies:

- Flex, grid, stack, scroll, sticky, overlay, and anchored-popover layout
- Programmatic focus, authored inertness, modal focus containment and
  restoration, focus-visible modality, and native directional navigation
- Accessible custom buttons, checkboxes, sliders, tabs, listboxes, dialogs,
  tables, links, navigation landmarks, regions, and current-page state
- Pointer capture, committed default actions, and input-capture policies
- Keyboard and normalized controller actions
- Presence, keyed animation, interruption, reduced motion, and audio time
- Audio playback and application-focus or visibility observation
- External-URL activation
- Stable semantic targeting for black-box tests

A later missing capability that blocks a page is implemented as a generalized
Reactant feature in that page's base change. The known accessibility extension
above must already be certified before Task 1. Sample-specific framework
adapters are not permitted.
