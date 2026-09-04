# 26. Dropdown animation

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"Dropdown and options animate presence, stagger, selection flash, and
interruption; settings composition remains unasserted."

**Visible result.** The finished select now animates from its closed
state into the fully painted list: the panel reveals below the trigger,
options enter with source stagger, the caret rotates, and selection flashes
before the list closes. Intermediate scale/translation/opacity match the
pinned ledger; settled open and closed appearances match Tasks 14--15.

**Exercise.** Open, select Windowed, reopen during closing, and dismiss
outside/Escape. Capture opening, selection flash, interrupted replacement, and
settled states. Reset returns to closed Borderless with no lingering overlay.
Reduced motion removes the source-disallowed movement.

**Deferred.** No settings screen composition or tab-panel transition is added.
