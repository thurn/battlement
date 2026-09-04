# 13. ToggleControl accessibility

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"ToggleControl exposes labeled checkbox semantics and help description;
effects, help modal, and composition remain unasserted."

**Visible result.** A labeled Upload Crash Reports checkbox row opens
checked and has the same finished paint and feedback as the earlier checkbox.
Its crash-report description is available to assistive technology without
adding a visible paragraph to the source row. Include the `aria_label`
override variant in the harness.

**Exercise.** Activate through pointer, keyboard, controller, and semantic
Activate; the check mark and checked state agree. Reset returns to checked.
The description reads “We upload crash reports to Unity Diagnostics.”

**Deferred.** The clickable info badge and visible help modal are Task 19.
No help panel or screen composition is required on this page.
