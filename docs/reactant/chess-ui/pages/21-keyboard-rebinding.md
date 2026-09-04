# 21. Keyboard rebinding

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"InputSettings captures keyboard bindings, rejects conflicts, resets
defaults, and announces status; icons and controller rebinding are not
asserted."

**Visible result.** The Task 20 table gains interactive keyboard cells.
Opening Move Piece displays the finished “Change Shortcut” modal, “Press a key
for Move Piece”, a cyan waiting marker, Cancel, and Reset. A conflicting key
shows “Already used by <action>” in the source pink/red status styling; the
modal stays open. The waiting marker's source blink is owned here.

**Exercise.** Assign an unused key and see the cell text update, reject a
conflicting key, cancel another capture, then reset a binding. Escape can be
captured as a key. Gallery reset closes capture and restores all defaults.

**Deferred.** Cells remain text-only until Task 24; controller cells remain
display-only permanently. The modal's entrance/exit and shine are Task 28.
