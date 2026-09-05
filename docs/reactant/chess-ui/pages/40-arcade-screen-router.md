# 40. ArcadeScreenRouter

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"ArcadeScreenRouter composes every accessible mockup behavior; no
player-visible behavior remains outside this page's scope."

**Visible result.** The gallery initially shows only this page's heading,
caption, and launcher. Launching displays the entire source-matching app in a
full-screen layer with no gallery navigation, counters, specimen selectors,
clock controls, or other review UI. It opens on the complete main menu.

**Exercise.** Settings opens the complete Gameplay settings screen; switch
all tabs, edit controls, open each dialog, change text size, and Return to main.
Preserve the source first-navigation rule and animate later route replacements.
Play/Quit finish on black. An otherwise unconsumed Escape/controller Cancel
closes the app layer and restores launcher focus. Relaunch/reselect resets all
values, bindings, scroll, overlays, route state, effects, and audio lifecycle.

**Deferred.** Nothing player-visible remains deferred. Only the documented
platform substitutions, corrected Return cutout, and intentional prototype
behaviors are exceptions to source parity. No browser render-mode diagnostics
or sample controls appear inside the app.

Before this candidate, run the project's single permitted independent
review and final source-coverage audit over the complete port. After
promotion, run the ordinary required port-ergonomics reviewer. If that
review produces a Reactant follow-up, refresh every affected source-coverage
and correspondence entry before the follow-up candidate. The port is not
complete until the final promoted follow-up, or the no-follow-up rationale,
retains a complete terminal audit.

**Final rendering acceptance.** Apply the [rendering policy](../rendering-policy.md)
to the assembled app. Verify source fidelity, all text scales, semantic names,
asset loading, and the representative performance path on macOS and Android.
Record frame-time and texture costs, resolve budget failures, and confirm no
unused sample recipes or duplicate settled rendering paths remain. Localized
rasterization differences allowed by the policy are not missing source features;
all other player-visible requirements remain in scope.
