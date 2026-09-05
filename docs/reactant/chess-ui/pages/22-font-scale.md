# 22. FontScale

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"FontScale reflows rows and scales text and controls; persistence and
complete screens remain unasserted."

**Visible result.** A text-size harness opens at 100% and offers 150% and
200%. It presents representative rows of every existing control, the four-tab
strip, both headings, action/Return buttons, dialogs, and the input table.
At larger sizes, SettingRow labels stack above controls with the source gaps,
padding, and height formulas; scroll containers reveal focused controls. Text
and controls grow by their own source formulas, not one uniform scale.

Tab widths remain 264/212/205/200 with 2-pixel gaps. Tab text multiplies its
55/51-pixel base by `1 + (fontScale - 1) * .25`; Gameplay and Graphics also
multiply by .92 above 100%. No abbreviated labels are introduced. Input columns
change from 310/310/remainder to 260/340/remainder. Headings, controls, navigation
labels, and Return's text cap follow `FontScale.tsx` and their component formulas.

**Exercise.** Compare all three sizes, open a list/dialog at 200%, scroll to
and focus the final input row, then reset to 100%. No text or focused control
is clipped. Use separate specimens instead of squeezing everything into one
stage.

**Deferred.** Binding icons are Task 24; complete screens Tasks 35--40.
Window narrowing still scales the portrait stage; it does not trigger reflow.

**Rendering acceptance.** Verify live text and generated decorative lettering
under the [text and scaling contract](../rendering-policy.md#text-and-scaling).
Check native semantic names, image resolution, baseline alignment, and memory
cost at all three sizes. Add finite baked variants only where needed; ordinary
labels, values, and custom bindings remain live text.
