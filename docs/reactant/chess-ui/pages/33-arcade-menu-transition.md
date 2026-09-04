# 33. ArcadeMenuTransition

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"ArcadeMenuTransition swaps keyed screens with beam and reveal effects;
complete routed screens remain unasserted."

**Visible result.** A keyed screen-transition harness opens on a clearly
labeled Main specimen. A harness action replaces it with a Settings specimen
using the source beam, reveal, overlay, and outgoing/incoming motion. The
specimens occupy the source screen bounds inside the completed frame; they
are simple distinguishable contents, not prematurely assembled screens.

**Exercise.** Transition in both directions and interrupt with a replacement.
Capture ledger-defined intermediate layers and the settled destination.
Reduced motion follows the source alternative. Reset restores Main with no
beam or outgoing screen visible.

**Deferred.** First-navigation policy and route ownership are Task 40;
complete MainMenu and SettingsScreen are Tasks 38--39.
