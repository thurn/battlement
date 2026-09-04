# 31. BackgroundMusicProvider

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"BackgroundMusic loops audio, applies effective volume and background mute,
and exposes playback context; heartbeat remains unasserted."

**Visible result.** A harness shows the existing Master Volume and Music
Volume sliders at 80 and 65, background mute off, and external playback,
visibility, and mute status. Activating playback advances the displayed
playhead; sliders retain their approved paint. Status is harness UI outside
the reference crop, not an invented source music player.

**Exercise.** Change volumes and simulate hidden/visible state. Effective
volume reflects .8 × .65 initially; background mute affects hidden playback
without pausing the playhead. Reset stops playback at zero and restores values.
Unavailable playback is explicit in harness status.

**Deferred.** The source music indicator and visible heartbeat are Task 32.
No waveform, equalizer, or new product audio controls are introduced.
