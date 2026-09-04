# 1. Gallery shell

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

Read the [gallery contract](../review-gallery.md) and verify the
[source prerequisites](../source-and-prerequisites.md) before implementation.

"Scrollable navigation selects one isolated demonstration; migrated mockup
content is intentionally not asserted."

**Visible result.** A 320-pixel scrollable navigation column lists all 40
numbered entries beside the centered design stage, with the padding and scale
specified under Review Gallery. Page 1 is selected; its heading and caption are
visible. Future entries show their heading, caption, and an explicit unavailable
specimen message until implemented. They must not display a fabricated mockup.

**Exercise.** Select an entry, scroll to entry 40, and reselect the current
entry. Exactly one navigation item indicates the current page; the content and
heading change, navigation reveals selection, and reset restores the heading
focus. Keyboard/controller focus is visibly distinguishable from selection.

**Deferred.** Mockup components belong to Tasks 2--40. The shell's own readable
navigation and focus presentation must already be complete.
