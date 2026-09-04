# 12. Focus-visible behavior

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"Keyboard and controller focus-visible states render correctly while
pointer focus hides the keyboard-only ring; complete controls remain
unasserted."

**Visible result.** The Task 11 specimens also show their source
keyboard/controller focus treatment: yellow/gold borders or outlines, white
and yellow glow, and the appropriate focused gradient. Moving focus moves
that treatment to one control; checked and selected states remain legible.
Pointer focus retains ordinary pointer/hover paint without the keyboard ring.

**Exercise.** Compare pointer click, Tab, Shift-Tab, and controller focus on
each specimen. Reset restores the gallery heading focus using the current
physical input modality, so a keyboard ring must not remain on an old control.

**Deferred.** Full arrow selection for tabs belongs to Task 17; listbox and
dialog navigation belong to Tasks 15 and 18. This page does not add panels,
shine, particles, or heartbeat.
