# 3. ScreenFrame and ConceptFrame

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"Arcade frame and clipped interior match their resting paint; pulses,
exits, generated textures, and controls remain unasserted."

**Visible result.** An empty arcade frame fills the portrait stage. Match
`styles.ts`'s `frameClip` polygon, the 21-pixel outer inset, 111-pixel bottom
inset, 8-pixel metallic border, cyan/blue/violet/pink border gradient and glow,
and `ScreenFrame`'s clipped dark radial interior. The interior is empty, not a
main-menu screenshot or placeholder control stack.

**Exercise.** Capture the static frame and its reset at both review sizes.
The corners, notches, border, and interior gradient must match the source.

**Deferred.** Generated frame substitution is Task 23, moving border comets
and the corrected Return cutout are Task 30, frame collapse is Task 34, and
screen contents are Tasks 35--40. The resting frame's shape and gradient are
required now.
