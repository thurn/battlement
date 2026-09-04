# Review gallery

[Plan and reading guide](../chess-ui-implementation-plan.md)

## Review Gallery

The gallery contains exactly 40 registered entries. Page 1 demonstrates the
gallery shell itself. Every entry displays its title and a 10--20 word
description of the behavior asserted on that page.

The per-task **Visible result**, **Exercise**, and **Deferred** paragraphs in the [page specs](../chess-ui-implementation-plan.md#migration-order)
are the visual acceptance contract. The short quoted descriptions are gallery
captions, not exhaustive scope definitions. Source-line ownership records must
follow this contract; they cannot independently defer a required visual feature.

Each contract describes the page immediately after its task is complete. Earlier
pages mount current shared components and acquire later improvements; they do
not preserve historical incomplete renderings. A later capability may therefore
appear on an earlier page without transferring ownership of its acceptance
evidence. Do not add feature switches solely to freeze an earlier appearance.

Each registration supplies:

```text
number, title, description, render_harness, reset_generation,
semantic_target, capture_states
```

`capture_states` is either `static` or an ordered list of named initial,
changed, and reset states. The container is a navigation landmark named
`Chess UI review pages`. Each item is a button named `<number>. <title>`; the
selected button exposes the extension's current-page state. The content is a
region labelled by its visible page heading. Selection recreates the harness
and queues focus on that heading. Directional controller input uses native
navigation to move focus among navigation buttons without selecting; Submit
activates the focused button.

The gallery navigation:

- Is vertically scrollable and remains outside the 1024x1536 design stage.
- Uses the navigation ScrollView ref's explicit `scroll_to` action to reveal the
  selected entry.
- Exposes current-page state on the selected review-page button.
- Supports pointer, keyboard, and controller activation.
- Mounts the current shared component implementations, so earlier pages improve
  as shared components and Reactant evolve.

At desktop sizes, the gallery uses a 320-pixel navigation column, a 24-pixel
gap, and 24-pixel outer padding. The design stage is centered in the remaining
space and uses this scale, capped at 1:

```text
min((viewport_width - 392) / 1024, (viewport_height - 48) / 1536)
```

The navigation scrolls independently. The design stage never causes outer-page
scrolling. Below a 1280x800 review window the same formula continues to shrink
the stage; mockup content does not reflow merely because the gallery is narrow.

Selecting any entry, including the currently selected entry, increments a mount
generation and fully recreates the page harness. Re-entry, application reload,
and relaunch reset:

- Component and provider state
- Focus target
- Dropdowns, dialogs, input capture, and overlays
- Audio playback and playhead
- Animation clocks and keyed effect generations
- Scroll positions
- Application-visibility simulation
- Final-router dismissal state

On reload or relaunch, Page 1 is selected and its heading is focused. After any
selection or reselection reset, the selected page heading is focused, no pointer
is captured, the host is visible and focused, the application is on the main
route, all overlays are closed, all page scroll offsets are zero, animation
time and heartbeat phase are zero, and audio is stopped at time zero. Audio
begins only when the page or full app explicitly activates its playback
behavior.

Focus-visible presentation always follows the panel's current physical input
modality. A pointer selection hides the heading's focus-visible treatment;
keyboard or controller selection retains it. Resetting a page does not clear or
replace that panel-local modality.

For the review shell, an action is **unconsumed** when the focused control,
input-capture policy, open listbox, active dialog, and application router all
return it as unhandled.

Page 40 first displays its title, description, and a launcher. Activating the
launcher opens the complete app in an unanchored full-screen `Overlay::layer`
without gallery chrome or sample-only controls. The review shell authors its
gallery-content root inert while the app is open and queues focus on the app's
initial heading. The layer is a logical sibling of that inert root, so portal
ancestry does not make the app inert. This layer is not a dialog and must not
use `Overlay::modal`. An unconsumed Escape or controller Cancel closes the
layer, removes authored inertness, and restores launcher focus. Pointer-only
exit is intentionally absent; application reload remains available.

The pages remain committed until the user explicitly accepts Task 40's final
parity candidate and every resulting Reactant follow-up has been promoted.
Their possible deletion is a separate, explicitly authorized task and is not
part of this plan.
