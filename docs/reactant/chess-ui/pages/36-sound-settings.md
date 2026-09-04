# 36. SoundSettings

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"SoundSettings composes three sliders and background mute against shared
audio state; Input settings remain unasserted."

**Visible result.** The Sound panel contains Master Volume 80, Music
Volume 65, Effects Volume 75, and unchecked Mute in Background in source order,
with the source multiline labels, spacing, and approved slider/checkbox paint.
Its panel surround and large-text layout match earlier specimens.

**Exercise.** Drag all three sliders, toggle background mute, and simulate
hidden/visible playback. Fill, thumb, and numeral agree; master/music affect
the shared audio state. Effects volume changes its visible value only. Reset
restores all four defaults, scrolling, and the audio lifecycle contract.

**Deferred.** Input composition is Task 37 and cross-tab settings ownership
Task 38. This page does not add menu chrome or a new effects sound.
