# 27. ArcadeTabTransition

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"ArcadeTabTransition enters, exits, and sweeps by direction; complete tab
contents and routing remain unasserted."

**Visible result.** The completed tab strip sits above a source-sized
panel viewport containing a simple labeled content specimen for each category.
Gameplay is initial. Switching categories produces the source directional
enter/exit motion and sweep; after settling exactly one correctly labeled
specimen remains visible. The tab strip's own shape and paint do not change.

**Exercise.** Switch right and left, wrap, and interrupt a transition with
another selection. Capture both directions and the reduced-motion result.
Reset restores Gameplay with no outgoing content or sweep remaining.

**Deferred.** The specimens are not real Gameplay/Graphics/Sound/Input
contents; those belong to Tasks 35--37. Full SettingsScreen is Task 38.
