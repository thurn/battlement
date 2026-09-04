# 15. SelectControl keyboard and controller behavior

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"SelectControl supports arrows, Home, End, typeahead, Escape, restoration,
and listbox semantics through handlers and queued ref focus; animation
remains unasserted."

**Visible result.** The same listbox now visibly distinguishes the
keyboard/controller active option from the committed selected option. Source
focus paint follows the active option, while the check mark continues to
identify the selected value until commitment.

**Exercise.** Open from the trigger, use arrows, Home, End, and typeahead,
commit Windowed, then reopen and Escape without changing it. The trigger
regains focus and the correct modality treatment. Reset is closed Borderless.

**Deferred.** This page adds no new decorative shell. Dropdown presence,
stagger, selection flash, and interruption animation remain Task 26.
