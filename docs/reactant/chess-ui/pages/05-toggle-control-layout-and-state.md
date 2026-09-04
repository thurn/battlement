# 5. ToggleControl layout and state

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"ToggleControl renders label, checkbox, and controlled toggling; focus,
animation, and help remain unasserted."

**Visible result.** A source-styled labeled checkbox row opens checked.
The 77x77 checkbox has its 4-pixel blue border, 11-pixel corner radius, dark
vertical gradient, inset shadow, blue glow, and cyan clipped check mark. Show
an unchecked specimen and the supported row-height/offset variants as well.
Preserve the source's treatment of `first`; do not remove a separator when the
source component ignores that prop.

**Exercise.** Clicking toggles the controlled check mark on and off; changing
`checked` from the harness changes the same specimen. Reset returns to checked.

**Deferred.** Hover/press feedback is Task 11, keyboard-only focus paint Task
12, help description semantics Task 13, the info badge/dialog Task 19, generated
checkbox parts Task 23, and animated checkbox effects Task 25.
