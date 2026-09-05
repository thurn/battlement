# 32. Music indicator and heartbeat

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"MusicPlaybackIndicator mutes or enables sound while controls pulse from
audio time; complete menu composition is not asserted."

**Visible result.** The source indicator reads “Playing with sound” and
“is recommended!” on two centered lines, with its source position, font, and
shadow. Active sound has no crossed-out speaker below it; muted/unavailable
sound shows the source gray speaker-slash icon. Representative completed
controls pulse in scale/brightness from audio time, using the specified two-hit
heartbeat rather than an unrelated animation clock.

**Exercise.** Mute and enable without rewinding, restore zero volumes through
enable, simulate unavailable playback, and compare normal/reduced motion.
Capture sound-on, muted, and ledger-timed pulse states. Reset stops the
playhead and clears the pulse until playback is activated again.

**Deferred.** The surrounding main menu is Task 39. Audio status diagnostics
remain outside the source crop.

**Rendering.** Keep the two-line recommendation live text. A small prepared
speaker-slash image or vector is appropriate. Heartbeat remains driven by audio
time and animates the chosen control skin without regenerating static artwork.
