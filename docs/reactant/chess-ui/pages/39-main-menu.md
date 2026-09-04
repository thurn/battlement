# 39. MainMenu

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"MainMenu composes background, header, buttons, music, and exit behavior;
the complete router remains unasserted."

**Visible result.** The complete main menu shows the arcade frame and
pulse, attract background, CHESS CHESS REVOLUTION heading, Play, Settings,
About, Quit in their source stack, and the music recommendation/indicator.
At 100%, buttons occupy left 132, width 760, starting top 476, with 140-pixel
heights and 24-pixel gaps. Playback and heartbeat follow the audio contract.
No settings panel is shown initially.

**Exercise.** About leaves the view unchanged. Settings emits a navigation
request. Play and Quit each run the same exit sequence and finish on black;
neither starts gameplay nor shuts down the host. Exercise sound mute/enable.
Reset restores the complete menu, initial state, and playback lifecycle.

**Deferred.** Settings-to-main route integration and first-navigation
transition policy are Task 40. No main-menu paint or exit behavior is deferred.
