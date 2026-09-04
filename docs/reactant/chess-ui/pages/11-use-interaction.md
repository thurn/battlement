# 11. useInteraction

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"useInteraction drives hover, press, release, and cancellation visuals;
focus modality and particles remain unasserted."

**Visible result.** A specimen selector presents the existing checkbox,
closed select, slider, action/Return button, and tabs with their completed
resting paint. Show source hover, held-press, release, and canceled-press
states, including brightness, border color, scale, and tab vertical offset.
For example, an inactive hovered tab rises to y=-1 and a pressed tab scales
to .955; the source's feedback transitions and reduced-motion branches apply.

**Exercise.** Enter, press, release, drag out/cancel, and leave each specimen.
Successful activation changes controlled state where applicable; cancellation
clears pressed presentation without a successful activation. Reset restores
resting paint and clears counts.

**Deferred.** Keyboard-only focus appearance is Task 12. Shine sweeps,
particles, and release bursts are Task 25; audio-driven pulse is Task 32.
Popover, panel, and route transitions retain their later owners.
