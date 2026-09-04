# 30. ArcadeFramePulse

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"ArcadeFramePulse animates border comets around the restored Return cutout;
exits and route effects remain unasserted."

**Visible result.** The finished empty arcade frame gains moving border
comets. A harness switches between main and settings frame contexts. The
settings specimen includes Return at its source location and the corrected
bottom-center cutout, so comets follow the remaining frame rather than crossing
through Return. Main has the source main-frame path.

**Exercise.** Capture both contexts at pinned pulse times and verify the
cutout geometry below. Reset restores the main context at time zero; reduced
motion follows the ledger.

**Deferred.** Frame collapse is Task 34 and complete routed settings Task
38. Only the documented settings cutout is a parity correction.

This is an approved parity exception. The source tests
`usePathname() === "/settings"`, but routing now stays at the root URL, so
that condition never succeeds. Reactant applies the cutout when
`active_screen == ArcadeScreen::Settings`. The source's existing
`frame-pulse-right-edge-fixed.png` and the following mask geometry are the
visual authority for the corrected state:

```text
position: 0 0, 0 100%, 100% 100%
size:     100% 1329px, 297px 75px, 297px 75px
repeat:   no-repeat
layers:   three linear-gradient(#000 0 0) masks
```
