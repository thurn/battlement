# 23. Generated control skin integration

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"Generated assets replace matching frame and label paint; catalog integration
and preserved appearance are asserted across earlier pages."

**Visible result.** A static specimen selector displays every generated
frame, label, logo, checkbox state, and slider part integrated at its real
runtime size. Earlier pages retain the same approved shapes, gradients,
shadows, text placement, and controlled resting states when generated artwork
replaces their paint. Include active/inactive tabs, checked/unchecked boxes,
slider endpoints, all action labels, both frame assets, and the game logo.
The settings panel frame is an isolated asset specimen here, not a composed
panel with rows or tabs.

**Exercise.** Capture the static variants and every affected earlier page.
Verify asset loading from the generated catalog with no missing-texture blocks,
transparent holes, seams, stretch artifacts, or duplicated overlaid paint.
The source CSS remains the visual target; reconcile a recipe mismatch rather
than approving a changed appearance merely because generation succeeded.

**Deferred.** Input glyphs and the assembled panel surround are Task 24.
No new hover, focus, burst, or transition behavior is introduced; retain
completed interaction behavior on earlier pages. Generated asset integration
must not be reported as the first completion of their static appearance.

Copy these existing declarations into `chess-ui`; do not create a shared
asset crate:

- `ARCADE_SCREEN_FRAME`: 1024x1536
- `SETTINGS_PANEL_FRAME`: 887x1021
- `ACTION_BUTTON_FRAME`: 760x140, slices 24/26/24/26
- `SMALL_CONTROL_FRAME`: 396x106, slices 15/15/15/15
- `SETTINGS_TAB_ACTIVE`: 288x154, slices 30/42/18/42
- `SETTINGS_TAB_INACTIVE`: 288x154, slices 30/42/18/42
- `GAME_LOGO`: 900x360
- `ACTION_LABEL_PLAY`, `SETTINGS`, `ABOUT`, `QUIT`, and `RETURN`:
  480x146 each
- `CHECKBOX_UNCHECKED` and `CHECKBOX_CHECK`: 101x101 each
- `VOLUME_SLIDER_TRACK`: 308x88, slices 18/18/18/18
- `VOLUME_SLIDER_FILL`: 278x20
- `VOLUME_SLIDER_TICKS`: 284x10
- `VOLUME_SLIDER_HANDLE`: 68x88

Generate all 18 assets through the ordinary asset command. The generator's
`manifest.json` is authoritative; each runtime address has the form
`battlement-reactant/generated/<request-hash>.png`. Validate the declaration
count, symbol-to-hash mapping, and linked runtime catalog.
