# 25. Control shine and release bursts

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"Buttons, checkboxes, and sliders play shine and keyed release bursts;
ambient and route effects remain unasserted."

**Visible result.** Existing action/Return buttons, tabs, select triggers
and options, checkboxes, and sliders retain their finished paint and gain
the source shine or keyed effect layers where their source uses them.
Action-button highlight shows its moving shine; successful releases produce
the source compact or full particle burst. Checkbox state changes and slider
release use their own source effect shapes and anchors.

**Exercise.** Trigger each effect, capture its ledger-defined intermediate
state and settled result, retrigger before completion, and cancel a press.
Effects must originate at the control and leave no residue. Reset clears all
particles and keys; reduced motion follows each source branch.

**Deferred.** Dropdown presence/selection flash is Task 26; modal shine Task
28; ambient effects Task 29; heartbeat Task 32. No effect can excuse a
resting-paint mismatch. EraseControl reuses its source-prescribed compact
button burst when that control is introduced in Task 35; modal buttons do
not gain a burst absent from their source.

**Rendering.** Keep the effect's geometry, timing, interruption, and state
live under the [rendering policy](../rendering-policy.md). Static textures may
supply reusable artwork; they must not replace motion with a captured frame or
force a second settled control skin. Verify the selected static paint path
through this page's animated and reduced-motion states.
