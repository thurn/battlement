# 9. SettingsTabs layout

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"SettingsTabs selects controlled tabs horizontally; directional focus,
panel transitions, and responsive labels remain unasserted."

**Visible result.** Show only the horizontal SettingsTabs strip, opening
with Gameplay selected. Columns are 264, 212, 205, and 200 pixels with 2-pixel
gaps in an 887x129 layout slot. Labels read Gameplay, Graphics, Sound, Input.
The active tab is 130 pixels high; inactive tabs are 127 pixels high with the
source's 3-pixel downward resting translation. Preserve bottom alignment and
visible overflow rather than forcing all painted tops into the slot.

The shape is mandatory: `tabOuterClip` cuts both top corners by 18 pixels,
keeps bottom corners square, and encloses a 4-pixel inset whose top cuts are
15 pixels. The active border is the 112-degree gradient with stops #72f5ff,
#53afff at 44%, #9a83ff at 68%, and #ff4ed3. Inactive borders use the source's
110-degree #657287 / #454f64 at 52% / #6f6577 gradient. Active and inactive
interiors retain their distinct dark vertical gradients and inset shadows;
the active tab has blue outer glow and a magenta inner bottom edge. Labels
use Barlow Condensed 700, 55 pixels active and 51 inactive, 1-pixel tracking,
#f7f7fb text, and the source text shadows. Rectangular solid borders fail this
page even if their bounding boxes match.

**Exercise.** Click each tab and show the active appearance moving to the
chosen label, with exactly one selected tab. A parent control can select Sound;
reactivating the current tab still emits its selection request. Reset selects
Gameplay. Changes are immediate until feedback motion is added.

**Deferred.** No content panel is required. Hover/press feedback is Task 11,
focus-visible paint Task 12, arrows/Home/End/controller selection Task 17,
scaled labels Task 22, generated frame substitution Task 23, release bursts
Task 25, and content-panel transitions Task 27. Tab shape and all resting
border/interior/text paint are required now.
