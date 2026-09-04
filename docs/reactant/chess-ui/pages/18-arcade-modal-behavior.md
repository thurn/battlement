# 18. ArcadeModal behavior

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"ArcadeModal traps focus, dismisses safely, restores its opener, and
exposes dialog semantics on its modal wrapper; animation remains
unasserted."

**Visible result.** An external opener shows a closed-dialog state. Open
it to display the source erase-confirmation specimen: “Erase Saved Data?”, its
source warning sentence, Cancel, and Erase. The stage behind it is darkened
and blurred; the centered clipped panel has the source cyan border, layered
dark gradients, inset/outer glow, title/body typography, and danger button.
The panel is at its fully open size and opacity immediately.

**Exercise.** Open, traverse contained focus, cancel with Escape, reopen,
confirm, and dismiss through the source backdrop behavior. Each close
restores opener focus; confirmation records an event without deleting data.
Reset closes the dialog.

**Deferred.** Opening/closing transforms and looping shine are Task 28.
The real EraseControl row and its composition belong to Task 35; no complete
Gameplay panel is needed here. Static modal paint is required now.
