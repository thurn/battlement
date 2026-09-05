# Visual fidelity

[Plan and reading guide](../chess-ui-implementation-plan.md)

## Visual Fidelity

The authoritative visual target is the mockup's CSS desktop rendering. The
canonical environment is:

- Inner design stage: exactly 1024x1536 logical pixels
- Device scale: 1
- Screenshot crop: the inner design stage only
- Initial route: main menu
- Motion preference: normal
- Safe-area insets: zero
- Fonts: Bebas Neue, Barlow Condensed 700, and Barlow Condensed 800 italic
- Secondary integration capture: 2560x1440 desktop with the source's 0.75
  outer scale

Run the pinned source unchanged with its existing development command. A
partial page uses the corresponding state in the complete source application,
cropped to the component under review. Transient effects are captured through
normal source interactions. Computed DOM and CSS values supplement the crop
where the source does not expose an isolated specimen. Do not add a temporary
React harness to manufacture references.

### What must look finished at each step

For numeric/style details not repeated in a task paragraph, the named component
and its imported style helpers at the pinned source revision supply the exact
values. The paragraphs define which state and layers are visible; the source
supplies those layers' complete rendering recipe. Do not approximate a source
value because this document describes its color or shape in words.

**Static appearance is required when a component first appears.** A task named
"layout", "closed state", or "behavior" is not permission to draw a wireframe.
Unless its Deferred paragraph explicitly names an exception, the component must
already match the pinned CSS reference in silhouette, clipping, corner cuts,
border thickness, every gradient stop, solid colors, inset and outer shadows,
glow, opacity, text paint, font, size, weight, spacing, alignment, and resting
transforms. These requirements include active, inactive, checked, unchecked,
disabled, and other settled states that the task exposes. Plain rectangular
borders, solid-color substitutes for gradients, omitted shadows, generic system
fonts, and default native control chrome are not acceptable placeholders.

Tasks 10–40 follow the [rendering policy](rendering-policy.md). Use generated
artwork at its first owning page when it is the intended implementation; do not
build a procedural substitute solely to defer asset use until Task 23. Task 23
audits rendering choices, catalog completeness, and performance rather than
requiring every control to become a texture. No browser render-mode toggle is
added to the sample.

Interactions and time-varying effects are separate requirements. Before the task
that owns an effect, render its settled state without that effect: changing a
tab changes its active paint immediately; opening a dropdown or dialog shows
its fully open appearance immediately. Do not leave an entering element at
zero opacity, half scale, or another animation start value. Task 11 owns hover,
press, and cancellation presentation and its short feedback transitions; Task
12 owns focus-visible paint; Task 25 owns shine and release bursts; Tasks
26--30 and 33--34 own their named animations; Task 32 owns audio-driven pulses.
A new control introduced after one of these tasks must reuse the applicable
completed behavior. Animation alone may not be used to defer static paint.

Only these visual substitutions are permitted before their named owner:

| Visible feature | Required before its owner | First task requiring the final feature |
| --- | --- | --- |
| Generated frame and label textures | Use the chosen rendering path at first appearance; preserve complete source treatment | 10: headings; 23: catalog and rendering audit; 24: icons and panel |
| Keyboard and controller binding icons | Actual binding names as readable text in correctly sized cells | 24: source keycaps, arrows, and controller glyphs |
| Settings panel frame | No panel required on isolated control pages; behavior harnesses may use a plain dark backdrop | 24: complete panel specimen |
| Text-size-dependent layout | Source appearance at 100%; no guessed large-text layout | 22: 100%, 150%, and 200% specimens |
| Full settings and main-menu composition | Only the components and fixtures listed for the selected page | 35--40: the named assembled panels and screens |

In particular, **Task 9 must show the source's clipped tabs, multicolor active
border, gray inactive border gradient, dark inner gradients, glow, and text
shadows.** Task 17 adds directional navigation, Task 22 adds scaled tab labels,
Task 25 adds release bursts, and Task 27 adds content-panel transitions. None
of those tasks is the owner of Task 9's resting tab paint.

### Specimens, captures, and reset

Each Visible result lists the required specimens, not permission to compose the
whole source screen early. Render one source-sized specimen at a time when
multiple variants cannot fit without rescaling. Put variant selectors, parent
state controls, event counters, and clock controls in a clearly separate harness
area outside the reference crop. They must not replace source content or change
its layout. Use the source component's own colors on a plain dark stage when
its surrounding screen is not yet in scope; omit absent neighboring components
rather than filling their space with invented product UI.

For every page, selecting or reselecting its gallery entry returns to the listed
opening state, clears transient effects, and applies the gallery focus/reset
contract. Unless specified otherwise, 100% text, normal motion, and the defaults
in [Behavioral Acceptance](behavior.md#behavioral-acceptance) apply. "Reset" below means this gallery operation,
not an extra reset button inside the mockup. Static pages have an identical
reset capture and an `N/A` changed state. Variant captures are still required
when a static page lists multiple specimens.

Compare all paint belonging to the current and earlier tasks, including paint
implemented without generated assets. Mask only the explicitly deferred visual
features above or in the selected task's Deferred paragraph; record each mask
and its owner. Task 23 recaptures any earlier pages whose rendering path changes
to prove that the selected implementation preserves appearance. Task 24 recaptures binding pages when text substitutes become icons.

Static pages may record their changed-state capture as `N/A`. Interactive pages
require initial, changed, and reset captures.

Geometry is measured in the unscaled 1024x1536 coordinate system and remains
within one logical pixel. Screenshots use sRGB and align by stage bounds.
Device scale 1 maps one logical pixel to one captured pixel. For Tasks 10–40,
[visual acceptance](rendering-policy.md#visual-acceptance) governs source
comparison: perceptual fidelity with documented localized rasterization
differences, plus deterministic same-renderer regression checks. These tasks do
not require browser-to-Unity 2/255 channel equality. The policy does not reopen
completed Tasks 1–9 or weaken their behavioral contracts.
