# 34. ArcadeExitSequence

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"ArcadeExitSequence and frame collapse synchronize dismissal; gameplay,
quitting, and routed composition remain unasserted."

**Visible result.** A completed frame with a representative painted
button/content specimen starts intact. An external trigger runs the source
exit overlay and synchronized frame brightness, distortion, and collapse,
ending on an entirely black stage. Gallery navigation remains outside the
stage; no invented game, farewell message, or quit dialog appears.

**Exercise.** Capture the intact, ledger-timed collapsing, and black states;
repeat with reduced motion. Once exiting, controls become inert and focus
clears. Reset restores the intact initial frame and specimen.

**Deferred.** Play/Quit wiring and the complete menu are Task 39; full-app
review-layer dismissal is Task 40. Neither gameplay nor host shutdown will
be added by those tasks.

**Rendering.** Keep the effect's geometry, timing, interruption, and state
live under the [rendering policy](../rendering-policy.md). Static textures may
supply reusable artwork; they must not replace motion with a captured frame or
force a second settled control skin. Verify the selected static paint path
through this page's animated and reduced-motion states.
