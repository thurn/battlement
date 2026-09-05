# Rendering policy for Tasks 10–40

[Plan and reading guide](../chess-ui-implementation-plan.md#reading-guide)

## End state

Use native UI behavior, reusable Battlement paint, and generated static artwork
in combination. Choose the simplest complete rendering path that meets visual,
interaction, scaling, and performance requirements. This policy governs Tasks
10–40 and their shared-component changes; it does not reopen Tasks 1–9.
References in those completed specs to Task 23 asset substitution identify
candidates for its audit, not a requirement to convert every control.

Layout, semantic names, focus, hit testing, controlled state, text input, and
animation remain live. A texture supplies artwork, never an entire interactive
screen or its behavior. Keep general-purpose paint capabilities used by other
components; remove unused sample recipes and redundant component rendering paths
once their replacements are verified. Do not delete a generator feature merely
because Unity supports that ingredient in isolation: a gradient can still be
needed inside a generated outlined, skewed wordmark.

Do not implement a temporary procedural equivalent solely to replace it with a
PNG later. Generated artwork may be introduced at its first owning page.
Reference recipes are inputs to inspect, not proof of visual correctness.

## Rendering choices and owners

| Treatment | Intended implementation | Owner |
| --- | --- | --- |
| Game logo and decorative Settings heading | Generated lettering, with native semantic text; preserve source gradient, outline, shadows, skew, and bounds | 10 |
| PLAY, SETTINGS, ABOUT, QUIT, RETURN decorative lettering | Generated label artwork on native buttons; keep arbitrary typed children live | 23 audits and completes the label family |
| Outer arcade frame | Prefer baked static frame artwork when it reduces complexity or rendering cost; keep pulse and collapse mechanisms separate | 23 evaluates; 30 and 34 verify animated use |
| Tabs, button backgrounds, checkboxes, dropdowns, sliders | Retain satisfactory procedural paint unless a focused comparison demonstrates a worthwhile baking benefit | 23 evaluates representative costly skins |
| Settings panel surround | Choose procedural paint or one baked background from evidence; rows and scrolling remain live | 24 |
| Controller glyphs and speaker-slash | Small prepared image/vector assets as appropriate; the CSS generator is optional | 24 and 32 |
| Binding text, ordinary labels, values, and user content | Live text | Each owning page |
| Shine, particles, grid motion, comets, heartbeat, route transitions | Runtime geometry, transforms, clipping, opacity, and clocks; small static textures may supply reusable effect artwork | 25–34 |

A generated background must preserve its content bounds, transparent padding,
corner geometry, and picking surface. Use nine-slicing only when its stretch
regions preserve the intended design. Do not stretch complex asymmetric frames
or bake moving content into a screenshot. Keep exactly one settled paint path
per treatment; temporary comparison variants stay in external QA fixtures.

## Text and scaling

Bake fixed decorative wording, not arbitrary labels or binding strings. Every
baked label retains the same native accessible name and action; decorative image
children must not introduce duplicate semantic targets. Preserve source typed
children instead of narrowing a button to a fixed image-only label API.

Task 10 prepares heading artwork with sufficient resolution for the source's
largest prescribed text scale. Task 22 verifies 100%, 150%, and 200% text and
both review sizes, including transparent margins and baseline alignment. Use
finite generated size variants when one texture cannot remain crisp within a
reasonable memory budget. Do not rasterize new text at runtime. Preserve the
source's state-only Language setting; do not invent translation behavior, but
keep ordinary text localizable rather than baking it into control backgrounds.

## Choosing by evidence

A simple native fill or gradient does not warrant a texture by default. Layered
shadows and elaborate lettering are stronger baking candidates. Custom paint is
reusable but has maintenance and rendering costs; PNGs have memory, loading,
filtering, and batching costs. Cached custom geometry avoids rebuilding every
frame, but the GPU still draws it. Never infer an FPS win from fewer source lines
or from using a PNG.

Task 23 compares the chosen procedural and baked variants on identical source
states, viewport, render scale, warmed assets, and hardware. Use a short native
Ditto scenario with idle and animated/selection phases. Record device/backend,
build mode, frame count, CPU/GPU frame-time medians and p95 where supported,
geometry/repaint work, draw calls, texture memory, and loading behavior. Mark
unavailable counters explicitly; don't substitute browser timing for native GPU
evidence. Repeat only when measurement noise prevents a decision.

Keep a procedural skin when the difference is negligible and its appearance is
accepted. Prefer baking when a reproducible cost reduction or materially simpler
implementation justifies its texture cost. Apply existing project performance
budgets; do not invent an unmeasured FPS claim. At Task 40, verify the assembled
app on macOS and the Android target used by the acceptance suite, recording the
same representative interaction path and any target-specific limitations.

## Visual acceptance

Preserve source composition, authored colors, typography, silhouettes, gradient
stops, clipping, and interaction states. Geometry remains within one logical
pixel at the canonical scale. Missing layers, wrong text, altered state, visible
seams, clipping, and obviously different glow extent fail acceptance.

Browser and Unity rasterization need not match every channel within 2/255.
For Tasks 10–40, compare aligned full crops and inspect them at intended display
size, retaining difference images and measurements. Small localized differences
in antialiasing, blur falloff, and compositing are acceptable when the complete
treatment remains perceptually faithful. Record their location and character in
the evidence; do not mask an entire effect or accept broad color/shape changes.
Uncertain or conspicuous differences are resolved before candidate submission.
The ordinary visual review covers these documented differences; no separate
approval is needed for each sampled pixel.

Use deterministic same-renderer Ditto baselines to catch regressions. A baseline
update proves neither source fidelity nor performance. Generated textures must
match their declared recipe under the recorded generator/browser inputs, and
runtime sampling must remain crisp and seam-free at required scales.
