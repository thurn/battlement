# 4. SettingRow

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"SettingRow aligns label and child horizontally; responsive reflow and
interactive controls are not asserted."

**Visible result.** Source-width rows show a label in the 422-pixel left
column and a plain child specimen in the right column. Include a first row,
a normal separated row, a multiline label, and an explicit-height variant.
At 100%, default minimum height is 159 pixels; labels use 61-pixel Bebas Neue,
uppercase treatment, source horizontal stretch and shadows. The normal row
has the source 2-pixel translucent separator; the first row does not.

**Exercise.** Capture each static variant; reset reproduces it exactly.

**Deferred.** The child is a size-marked specimen, not an unfinished control.
Actual controls begin at Task 5. Stacked large-text rows belong to Task 22.
