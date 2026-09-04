# Chess UI Reactant Port Implementation Plan

This plan defines a complete Rust port of the player-visible `mockups` React
application using **Reactant**, Battlement's declarative Rust component runtime.
The result is a standalone `chess-ui` sample whose appearance and behavior
match the pinned TypeScript source at desktop resolution.

The sample is a deliberate challenge to Reactant's API and architecture. Every
migration must strongly question why Reactant cannot express the source as
directly as React can. Consider major public API changes, replacement of
existing abstractions, and changes to runtime responsibilities when they make
application code simpler and preserve the source's behavior. The current
Reactant API is an implementation to improve, not a constraint to defend.

An audit that finds only naming, formatting, or small local cleanups while
leaving unexplained architectural differences has not fulfilled this plan.
Visual parity and passing tests are necessary, but insufficient: the sample
must demonstrate that ordinary application authors can express the same
component boundaries, controlled props, inline composition, labels, controls,
focus, accessibility, assets, and motion without unnecessary framework wiring.

The port is developed through exactly 40 selectable review pages. Each page
isolates one responsibility, states what must work, and states what that page
does not yet assert. The last page composes the pieces into the complete app.

## Reading guide

Start here and open the selected page in the migration order below. The shared
contracts and page specs together are the authoritative plan. Read applicable
sections rather than loading every page or repeating shared rules in each spec.

For each task, create a compact requirement-to-evidence checklist outside tracked
source. Include the page's visible result, exercise, deferred scope, applicable
shared contracts, and links to their authoritative sections. A checklist helps
execution; it cannot narrow the specification or waive a requirement.

| When | Read |
| --- | --- |
| Before the first migration; when a prerequisite or source input changes | [Source, pinned revision, dependencies, and research](chess-ui/source-and-prerequisites.md) |
| Every implementation | The selected page; [port principles](chess-ui/port-contract.md#port-contract); [workflow](chess-ui/workflow.md); [visual fidelity and ownership](chess-ui/visual-fidelity.md); applicable [validation](chess-ui/validation.md) |
| Authoring or changing a component | Its entries in [component correspondence](chess-ui/port-contract.md#component-correspondence) and [platform substitutions](chess-ui/port-contract.md#platform-substitutions); the [architectural challenge](chess-ui/review-protocol.md#mandatory-architectural-challenge) |
| Building or resetting a page harness | [Gallery layout, semantics, and reset](chess-ui/review-gallery.md) |
| Choosing defaults, settings, dialogs, audio, routes, or input behavior | Relevant [behavioral acceptance](chess-ui/behavior.md) sections, including prototype behavior and keyboard/controller rules |
| Preparing a candidate or reviewing a promoted page | [Reviewer inputs and protocol](chess-ui/review-protocol.md), alongside that page and its evidence |

Shared rules have one authoritative home. Page-specific requirements and named
deferrals stay in the page spec. A short gallery caption is not a substitute
for either. Reuse unchanged source snapshots and verified evidence according to
the workflow; reread the affected contract when its inputs or scope change.

## Migration order

Implement pages in numeric order. Each page starts from the certified release
containing all earlier migrations and their required reviewer follow-ups.
Dependencies within a page are stated in its spec; a named later task marks a
deferral, not permission to omit paint or behavior owned by the current page.

The page order begins with horizontal layout and controlled props, then adds
interaction, focus, accessibility, assets, motion, audio, and composition.

Three pages intentionally group closely coupled work: Task 19 validates one
help dialog including its link, Task 23 validates one generated-asset batch, and
Task 35 validates the two state-only settings panels. These remain one review
boundary because splitting them would not expose an independently meaningful
player interaction. The approximate 500-line task target still applies.

### Layout and shared controls

1. [Gallery shell](chess-ui/pages/01-gallery-shell.md)
2. [PortraitViewport](chess-ui/pages/02-portrait-viewport.md)
3. [ScreenFrame and ConceptFrame](chess-ui/pages/03-screen-frame-and-concept-frame.md)
4. [SettingRow](chess-ui/pages/04-setting-row.md)
5. [ToggleControl layout and state](chess-ui/pages/05-toggle-control-layout-and-state.md)
6. [SelectControl closed state](chess-ui/pages/06-select-control-closed-state.md)
7. [VolumeControl layout](chess-ui/pages/07-volume-control-layout.md)
8. [ActionButton](chess-ui/pages/08-action-button.md)
9. [SettingsTabs layout](chess-ui/pages/09-settings-tabs-layout.md)
10. [ScreenHeader](chess-ui/pages/10-screen-header.md)

### Interaction, focus, accessibility, and input

11. [useInteraction](chess-ui/pages/11-use-interaction.md)
12. [Focus-visible behavior](chess-ui/pages/12-focus-visible-behavior.md)
13. [ToggleControl accessibility](chess-ui/pages/13-toggle-control-accessibility.md)
14. [SelectControl pointer popover](chess-ui/pages/14-select-control-pointer-popover.md)
15. [SelectControl keyboard and controller behavior](chess-ui/pages/15-select-control-keyboard-and-controller-behavior.md)
16. [VolumeControl input](chess-ui/pages/16-volume-control-input.md)
17. [SettingsTabs navigation](chess-ui/pages/17-settings-tabs-navigation.md)
18. [ArcadeModal behavior](chess-ui/pages/18-arcade-modal-behavior.md)
19. [InfoBadge and Privacy Policy](chess-ui/pages/19-info-badge-and-privacy-policy.md)
20. [Input settings table](chess-ui/pages/20-input-settings-table.md)
21. [Keyboard rebinding](chess-ui/pages/21-keyboard-rebinding.md)
22. [FontScale](chess-ui/pages/22-font-scale.md)

### Assets, effects, animation, and audio

23. [Generated control skin integration](chess-ui/pages/23-generated-control-skin-integration.md)
24. [Input icons and settings panel skin](chess-ui/pages/24-input-icons-and-settings-panel-skin.md)
25. [Control shine and release bursts](chess-ui/pages/25-control-shine-and-release-bursts.md)
26. [Dropdown animation](chess-ui/pages/26-dropdown-animation.md)
27. [ArcadeTabTransition](chess-ui/pages/27-arcade-tab-transition.md)
28. [ArcadeModal animation](chess-ui/pages/28-arcade-modal-animation.md)
29. [ArcadeAttractMode](chess-ui/pages/29-arcade-attract-mode.md)
30. [ArcadeFramePulse](chess-ui/pages/30-arcade-frame-pulse.md)
31. [BackgroundMusicProvider](chess-ui/pages/31-background-music-provider.md)
32. [Music indicator and heartbeat](chess-ui/pages/32-music-indicator-and-heartbeat.md)
33. [ArcadeMenuTransition](chess-ui/pages/33-arcade-menu-transition.md)
34. [ArcadeExitSequence](chess-ui/pages/34-arcade-exit-sequence.md)

### Screen composition

35. [Gameplay and Graphics settings](chess-ui/pages/35-gameplay-and-graphics-settings.md)
36. [SoundSettings](chess-ui/pages/36-sound-settings.md)
37. [InputSettings composition](chess-ui/pages/37-input-settings-composition.md)
38. [SettingsScreen](chess-ui/pages/38-settings-screen.md)
39. [MainMenu](chess-ui/pages/39-main-menu.md)
40. [ArcadeScreenRouter](chess-ui/pages/40-arcade-screen-router.md)
