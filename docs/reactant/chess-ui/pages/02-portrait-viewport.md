# 2. PortraitViewport

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"Fixed stage scales to fit available space; responsive content reflow is not
asserted."

**Visible result.** One empty 1024x1536 portrait stage fits inside the
gallery without cropping or stretching. Its aspect ratio stays 2:3; shrinking
the window scales the entire stage uniformly according to the gallery formula.
Use a visible stage boundary or measurement markers outside the source crop to
make its edges reviewable.

**Exercise.** Capture the canonical and integration window sizes. The stage
remains centered, navigation scrolls independently, and no outer scrollbar is
introduced. Reset yields the same empty stage.

**Deferred.** The arcade frame is Task 3; no frame, logo, controls, responsive
content rearrangement, animation, or audio is required here.
