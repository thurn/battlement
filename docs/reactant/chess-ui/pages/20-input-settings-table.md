# 20. Input settings table

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"InputSettings scrolls bindings beneath a sticky header; rebinding,
conflicts, and visual icons remain unasserted."

**Visible result.** A source-size 839-pixel-wide input table shows Action,
Keyboard, Controller headings and seven rows: Left, Right, Up, Down, Move
Piece, Pause, Restart. Initial bindings match Behavioral Acceptance. Match
row height, column widths, separator paint, header background,
text, and scroll viewport. Keyboard/controller cells deliberately show binding
names as text; they do not show placeholder squares or guessed glyphs.

**Exercise.** Scroll until Restart is visible. The header stays fixed and
aligned above its columns; reset returns to the top and default bindings.

**Deferred.** Rebinding/dialog/conflict states are Task 21 and icon artwork
Task 24. The full settings panel surround is not required.
