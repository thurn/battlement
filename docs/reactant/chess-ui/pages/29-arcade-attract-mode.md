# 29. ArcadeAttractMode

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"ArcadeAttractMode animates seeded grid and particles deterministically;
menu controls and audio remain unasserted."

**Visible result.** The portrait frame encloses only the source attract
background: its layered grid and seeded ambient particles at the source
positions, colors, opacity, and clipping. At time zero it shows the defined
initial seed state, not an arbitrary screenshot of a running effect.

**Exercise.** Advance the controlled clock to ledger capture times; the grid
and particles move deterministically. Reset returns to the identical initial
seed/time image. Reduced motion follows the source's static/reduced alternative.

**Deferred.** No logo, menu buttons, music indicator, or audio belongs to
this background specimen; the assembled main menu is Task 39.

**Rendering.** Keep the effect's geometry, timing, interruption, and state
live under the [rendering policy](../rendering-policy.md). Static textures may
supply reusable artwork; they must not replace motion with a captured frame or
force a second settled control skin. Verify the selected static paint path
through this page's animated and reduced-motion states.
