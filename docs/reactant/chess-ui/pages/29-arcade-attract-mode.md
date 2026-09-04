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
