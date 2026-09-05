# 10. ScreenHeader

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"ScreenHeader renders generated heading artwork; text scaling and complete
screen composition remain unasserted."

**Visible result.** Provide separate game and settings heading specimens
at their source positions. The game heading reads CHESS CHESS on its first
line and REVOLUTION on its second; the other reads Settings. Match the
Barlow Condensed 800 italic letters, gradient fill, stroke, colored offset
shadows, skew/stretch, and the blue left and pink right clipped stripe bars.
At 100%, heading containers are left 84, width 854, with top/height 103/330
for game and 74/122 for settings. Use the source's distinct text transforms.

**Rendering.** Generate the logo and decorative Settings lettering now,
including their text effects. Keep the stripe bars procedural unless baking
simplifies the complete treatment. Retain native semantic headings without
duplicating image text in the semantic tree. Follow the
[text and scaling contract](../rendering-policy.md#text-and-scaling); inspect
existing recipes against the pinned source before reuse.

**Exercise.** Capture both static variants and their reset. Verify generated
asset preparation, loading, subject bounds, and the required resolution.

**Deferred.** Font-scale behavior and its full size matrix are Task 22;
surrounding screen composition belongs to Tasks 38--39. No title animation is introduced.
