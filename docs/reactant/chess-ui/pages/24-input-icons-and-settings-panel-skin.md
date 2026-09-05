# 24. Input icons and settings panel skin

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"InputBindingIcons and the settings panel render precisely; rebinding
behavior and full composition remain unasserted."

**Visible result.** Two static specimens are required. First, display the
input table with actual source keycaps, directional arrows, D-pad marks, green
A, gray menu, and yellow Y glyphs in place of the text-only binding substitutes.
Second, display the 887x1021 settings panel surround with its clipped bottom
corners, thin blue/purple gradient border, layered dark interior, inset shadow,
and 18/24/32-pixel top/horizontal/bottom content padding. Use a plain interior
specimen to show the padding without assembling settings content.

**Exercise.** Capture default and long/custom binding variants at each text
size; reset reproduces them. Earlier input pages now render icons while
retaining rebinding and scrolling. Check the final row beneath the sticky header.

**Deferred.** Full Input composition is Task 37 and tabs/panels/Return
integration Task 38. No new rebinding policy or controller editing is added.

**Rendering.** Small prepared PNG or vector glyphs are both valid; the CSS
asset generator is not mandatory for icons. Keep user-defined binding text live.
Choose one panel-background implementation using the rendering policy and
Task 23 evidence; preserve live rows, scrolling, and the stated padding.
Do not bake the whole table or panel contents into an image.
