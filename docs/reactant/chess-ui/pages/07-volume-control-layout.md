# 7. VolumeControl layout

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"VolumeControl renders track, fill, thumb, value, and controlled changes;
rich input and effects remain unasserted."

**Visible result.** A Master Volume row opens at 80 with the source's
284-pixel track, proportional colored fill, tick marks, clipped metallic thumb,
and numeric value. Match the track's dark gradient, fill gradient, thumb shape,
shadows and glow; retain the source thumb overhang at both endpoints.

**Exercise.** Parent controls set 0, 50, and 100. Fill width, thumb position,
and numeral agree at every value. Reset restores 80.

**Deferred.** Complete drag, keyboard, and controller input is Task 16;
generated slider parts Task 23; release effects Task 25; audio behavior Task
31; heartbeat Task 32. A generic native slider is not a visual substitute.
