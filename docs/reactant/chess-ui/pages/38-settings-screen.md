# 38. SettingsScreen

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"SettingsScreen composes tabs, panels, Return, and both dialogs; main menu
and route transition remain unasserted."

**Visible result.** The complete settings screen opens on Gameplay:
finished arcade frame and pulse/cutout, Settings heading, four tabs at left
68/top 233, the 887-pixel panel below the tab strip, complete active panel
contents, and Return at its source position. No dialog or dropdown is open.
Every visible component includes the paint, icons, effects, scaling, and
behavior completed earlier.

**Exercise.** Visit all four tabs with animated panel replacement; edit values,
return to panels and verify retained state; open help, erase confirmation, and
rebinding. Erase confirms and closes without deleting anything. Return emits a
main-route request to the harness. Reset restores Gameplay and every default.

**Deferred.** Return need not reveal a real main menu here: that composition
is Task 39 and integrated routing Task 40. No settings chrome or static paint
is deferred beyond this page.
