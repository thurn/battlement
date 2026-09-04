# 6. SelectControl closed state

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"SelectControl renders changing controlled values and its caret; opening,
options, focus, and animation remain unasserted."

**Visible result.** A Display Mode row opens with Borderless in the closed
select trigger. Match the source's 396x106 trigger at 100%, clipped corners,
3-pixel inset, cyan-to-pink border gradient, dark interior, glow, Barlow Condensed
value text and shadow, and downward caret. The trigger remains closed.

**Exercise.** Harness controls set Borderless, Fullscreen, and Windowed;
only the displayed controlled value changes. Reset restores Borderless.

**Deferred.** Hover/press feedback is Task 11, focus paint Task 12, opening
and the option list Task 14, keyboard list navigation Task 15, generated frame
substitution Task 23, and popover animation Task 26.
