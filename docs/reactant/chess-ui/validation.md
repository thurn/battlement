# Validation and manual QA

[Plan and reading guide](../chess-ui-implementation-plan.md)

## Automated Validation

Tests should describe player-visible behavior rather than private Rust
structure. A **fake host** is Reactant's deterministic non-Unity renderer used
to inspect committed UI output. Add focused unit tests only for genuinely
complex algorithms.

Pages 2--4, 10, 23, and 24 are static for capture purposes. Every other page is
interactive or time-varying and supplies named initial, changed, and reset
states. All pages receive Ditto smoke and reset coverage; pages with a
meaningful interaction also receive a changed-state scenario.

Every task supplies validation appropriate to its page:

- Fake-host render, initial-state, interaction, reset, and semantic assertions
- An explicit `N/A` changed state for static pages
- Pointer, keyboard, controller, dismissal, focus-restoration, and modality
  matrices for interactive controls
- Controlled clocks and deterministic seeds for Motion behavior
- Numeric animation assertions copied from the exact rows of the pinned
  animation coverage ledger
- Asset-address uniqueness, canvas, slices, fonts, audio, and provenance checks
- Ditto initial, changed, and reset scenarios for each applicable page
- Smoke and reset checks for every previously registered page
- Targeted screenshot recapture whenever a shared component changes
- Unity-backend assertions for roles, names, states, relationships, listbox and
  table semantics, landmarks, current-page state, dialog isolation, live
  announcements, and external links
- At Task 40, a VoiceOver path through the macOS player and a TalkBack path
  through an Android player: launch, open Settings, change one tab and control,
  open and close each dialog, activate Return, and dismiss the full app
- A Task 40 audit assigning every source line a terminal disposition; each
  resulting Reactant follow-up refreshes its affected audit entries before
  candidate submission
- A complete architectural challenge over the assembled application, including
  earlier components; prior per-page approval does not exempt accumulated glue,
  repeated associations, or inconsistent control APIs from redesign

The per-page smoke check opens the registered entry, finds its semantic target,
and asserts that no error or warning was emitted. The reset check mutates every
state domain owned by that page, reselects the same entry, and asserts the
documented default values, zero scroll, closed overlays, reset clock and audio,
and expected focus target.

Task 40 receives the project's single independent-review pass before candidate
submission. The required post-promotion port-ergonomics reviewer remains a
separate review and may produce one final Reactant follow-up. That follow-up
cannot promote until its affected source-coverage and correspondence entries
are current and the complete audit remains terminal.

## Manual QA

1. Launch `chess-ui`. Count 40 entries, read every description, navigate the
   full list with pointer, keyboard, and controller, and reselect entries. Pass
   when the named navigation and region, current-page state, selection, focus,
   explicit scrolling, and every reset value match the gallery contract.
2. Compare every visually applicable initial, changed, and reset state with its
   unchanged source crop at 1024x1536. Then capture the 2560x1440 integration
   view. Pass when geometry and pixel evidence meet the documented tolerances.
3. Exercise hover, press, release, pointer cancellation, focus-visible changes,
   D-pad, left stick, Submit, ignored shoulder buttons, and Cancel. Pass when
   controller and keyboard actions match and pointer focus never gains a
   keyboard-only ring.
4. Tab and Shift-Tab across all four settings tabs, then use every arrow,
   Home, and End. Test dropdown outside-click dismissal, Escape, typeahead, and
   focus restoration. Pass when focus and selection follow the defined order
   and the dropdown publishes one listbox with the expected options.
5. Drag sliders and use arrows, Page Up, Page Down, Home, and End. Capture a new
   shortcut, reject a conflict, cancel capture, and reset the binding. Pass when
   values, announcements, modal state, and display-only controller cells match
   the source.
6. Switch among 100%, 150%, and 200% text. Scroll the sticky input table and
   focus its final row. Pass when rows reflow, headings remain visible, focused
   content is revealed, and no text or control is clipped.
7. With VoiceOver and TalkBack, follow the Task 40 accessibility path. Activate
   Privacy Policy through a test host. Pass when roles, names, states,
   relationships, listbox and table semantics, link activation, dialog
   isolation, announcements, exact URL, and focus restoration are correct.
8. Observe each animation normally, with reduced motion, and while interrupted.
   Toggle sound, simulate unavailable playback, change visibility, background
   mute, and volume. Pass when timing follows the ledger and heartbeat, mute,
   enable, zero-volume restoration, and reset follow the audio contract.
9. Open the complete router. Verify every setting, Return, inert About, erase
   no-op, state-only controls, and identical Play and Quit exits. Pass when
   neither exit invokes gameplay or host shutdown and both finish on black.
10. Confirm the full-screen app contains no gallery chrome. After dismissal,
    press an otherwise unhandled Escape or controller Cancel. Pass when the
    app layer makes gallery content inert without publishing a dialog, the
    launcher regains focus, the review shell returns with Page 40 reset, and no
    player-visible sample control was added.
