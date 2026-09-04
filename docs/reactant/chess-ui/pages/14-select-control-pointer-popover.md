# 14. SelectControl pointer popover

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"SelectControl opens one anchored listbox, selects options, and dismisses
outside; keyboard behavior remains unasserted."

**Visible result.** Display Mode opens as the fully styled closed
Borderless trigger. Clicking opens one source-styled list directly below it,
with the same width and a 6-pixel gap. Show Borderless, Fullscreen, and
Windowed, the selected check mark, hovered option background, clipped gradient
frame, dark interior, and shadows. The caret points up while open.

**Exercise.** Open the list, hover Windowed, select it, reopen, and dismiss by
clicking outside. The trigger reads Windowed after selection; reset closes
the list and restores Borderless. Open/close and caret reversal are immediate.

**Deferred.** Keyboard/controller list navigation is Task 15. Presence,
stagger, caret rotation timing, and selection-flash animation are Task 26.
The fully open list's shape, gradients, text, and selected mark are required now.
