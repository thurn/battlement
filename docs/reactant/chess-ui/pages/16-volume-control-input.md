# 16. VolumeControl input

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"VolumeControl supports drag, keyboard steps, endpoints, pages, and
controller input; release effects remain unasserted."

**Visible result.** The finished Master Volume slider opens at 80.
Dragging visibly moves its fill, thumb, and numeral together, with existing
hover, press, and focus paint. Values remain integers between 0 and 100.

**Exercise.** Drag to both endpoints; use arrows, Page Up/Down, Home/End, and
controller actions. Match the source's step sizes, clamping, and touch padding.
Cancel a captured pointer and confirm pressed paint clears. Reset restores 80.

**Deferred.** Release bursts are Task 25 and playback integration Task 31. Do not
add an audio visualization or redesign the already approved slider paint.
