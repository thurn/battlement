# 8. ActionButton

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"ActionButton renders typed children and invokes clicks; interaction states,
particles, and navigation remain unasserted."

**Visible result.** A source-size 760x140 ActionButton shows PLAY with
cut corners, multicolor border, dark inset, source glow, and the gradient,
stroked, shadowed label. Separate specimens cover custom typed children and a
disabled button. Include ReturnButton at its source rectangle: left 328, top
1358, width 368, height 120, with its dark backing and RETURN label.

**Exercise.** Enabled buttons update an external activation counter. Disabled
buttons do not. Return requests its callback without navigating a screen.
Reset clears counters. Preserve the source's actual disabled appearance; do
not invent a gray treatment.

**Deferred.** Hover/press states are Task 11, focus paint Task 12, generated
frames/labels Task 23, shine/bursts Task 25, and route/exit integration Tasks
34 and 38--40.
