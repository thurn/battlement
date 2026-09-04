# 28. ArcadeModal animation

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"ArcadeModal animates backdrop, panel, and shine with reduced-motion
alternatives; screen composition remains unasserted."

**Visible result.** The finished erase, help, and rebinding modal
specimens gain backdrop fades, panel reveal/collapse, skew/brightness changes,
and looping panel shine as specified in the ledger. Their fully open text,
buttons, borders, and dimensions remain those already approved.

**Exercise.** Open and close each variant, interrupt an entrance with close,
and reopen during exit. Capture entrance, open shine, and exit. Exiting content
is inert and cannot retain focus. Reset closes all dialogs and clears shine;
reduced motion uses the source's short fades without the large transforms.

**Deferred.** No new modal content or settings-screen composition is added.
