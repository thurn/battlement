# 35. Gameplay and Graphics settings

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"Gameplay and Graphics settings compose matching controls and props; other
tabs and final transitions remain unasserted."

**Visible result.** Separate Gameplay and Graphics specimens contain
complete, source-painted row contents within the approved panel surround.
Gameplay opens with Language English, Text Size 100%, Reduce Motion off,
Increase Move Duration on, Upload Crash Reports on with its info badge, and
the fully painted red ERASE row. Graphics opens with Resolution 1920 × 1080,
Max Framerate 144 FPS, Display Mode Borderless, Screenshake on, and VSync on.
Match source row order, offsets, line breaks, spacing, and panel scrolling.

**Exercise.** Change every control and compare all text sizes. Selecting
150% or 200% reflows the specimen. Harness callbacks may demonstrate the already
completed help/erase dialogs, but are outside cross-screen routing. Reset
restores all listed values and closes overlays.

**Deferred.** Sound and Input composition are Tasks 36--37; the shared tab
strip, header, Return, and integrated dialog ownership are Task 38. No platform
locale, graphics, saved-data, or gameplay effects are added.
