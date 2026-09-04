# 17. SettingsTabs navigation

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"SettingsTabs preserves four Tab stops and adds directional selection
with visible focus; animated content panels remain unasserted."

**Visible result.** The completed tab strip opens on Gameplay. Arrow,
Home/End, and controller selection now move both the active tab paint and
keyboard/controller focus treatment. All four labels remain sequential Tab
stops; focused and selected are distinct states when ordinary Tab moves focus.

**Exercise.** Wrap Input to Gameplay and back, select first/last, and traverse
all four tabs with Tab/Shift-Tab. Reset selects Gameplay and restores gallery
heading focus. Selection still renders without a content-panel animation.

**Deferred.** Scaled labels are Task 22 and panel transition Task 27. This
page requires no settings content panel and makes no new resting-skin change.
