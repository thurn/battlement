# 10. ScreenHeader

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"ScreenHeader matches both painted heading variants; generated textures,
text scaling, and surrounding screen composition remain unasserted."

**Visible result.** Provide separate game and settings heading specimens
at their source positions. The game heading reads CHESS CHESS on its first
line and REVOLUTION on its second; the other reads Settings. Match the
Barlow Condensed 800 italic letters, gradient fill, stroke, colored offset
shadows, skew/stretch, and the blue left and pink right clipped stripe bars.
At 100%, heading containers are left 84, width 854, with top/height 103/330
for game and 74/122 for settings. Use the source's distinct text transforms.

**Exercise.** Capture both static variants and their reset.

**Deferred.** The generated logo is Task 23; its absence does not permit plain
unpainted heading text. Font scaling is Task 22 and surrounding screen
composition Tasks 38--39. No title animation is introduced.
