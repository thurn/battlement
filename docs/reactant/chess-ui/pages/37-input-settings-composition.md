# 37. InputSettings composition

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"InputSettings composes bindings, icons, scrolling, rebinding, and its
modal; cross-tab integration is not asserted."

**Visible result.** The Input panel combines the approved surround,
sticky Action/Keyboard/Controller header, seven binding rows, real icons,
scrolling, and the animated Change Shortcut modal. At default size the first
rows appear at the source scroll position; scrolling reveals Restart beneath
the fixed header. Larger text follows Task 22.

**Exercise.** Rebind Move Piece, show a conflict, cancel, reset one binding,
and scroll/focus the last row. Updated key text/icon matches the binding.
Gallery reset closes capture, restores all default bindings, and scrolls to top.

**Deferred.** The tab strip, settings title, Return, and state integration
with the other settings panels are Task 38. Controller cells remain display-only.
