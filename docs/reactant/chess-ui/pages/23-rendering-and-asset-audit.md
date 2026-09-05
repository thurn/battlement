# 23. Rendering and asset audit

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"Rendering choices preserve source appearance; generated catalog coverage,
scaling, and measured rendering costs are verified across controls."

**Visible result.** A static specimen selector displays each retained rendering
treatment at its real runtime size: active/inactive tabs, checkbox states,
slider endpoints, all decorative action labels, both headings, outer frame,
and an isolated settings panel background. It shows the chosen implementation,
not a user-facing renderer toggle. Earlier pages retain their behavior and
approved appearance. Use the title and caption above in gallery registration.

**Exercise.** Apply the [rendering policy](../rendering-policy.md). Complete the
fixed decorative action-label family as generated artwork on native buttons,
preserving arbitrary typed children and accessible names. Compare procedural
and baked variants of representative expensive skins, including glowing tabs
and the outer frame, using the prescribed native measurements. Adopt baking
only when quality, cost, or implementation simplicity warrants it. Keep useful
shared paint code and satisfactory procedural controls.

Audit every sample asset declaration: retain, correct, or remove it based on
actual runtime use. The existing 18 declarations are a starting inventory,
not a required count or a mandate to use every PNG. Verify recipe fidelity,
symbol-to-hash mapping, generated manifest, Addressables loading, filtering,
transparent padding, and required text scales. Remove unused sample declarations
and duplicate settled paint paths; do not delete shared generator ingredients
needed by retained compound artwork. Do not create a shared sample asset crate.

Audit the generator's native-only rejection rule against these decisions. If it
rejects a justified baked treatment solely because runtime paint can express
its ingredients, implement a generalized eligibility correction with public
validation tests and updated generator documentation. Preserve the closed typed
syntax and rejection of unsupported properties; add no sample-name exception
or arbitrary rendering escape hatch.

Recapture changed earlier pages and compare the selected result against the
pinned source, then refresh only intentional baselines. Verify no missing
textures, seams, stretched corners, duplicated paint, semantic changes, or
behavioral regressions. Preserve the performance comparison and rendering
choice with the candidate evidence; no second production path is retained for
benchmarking.

**Deferred.** Input glyphs and assembled panel surround are Task 24. No new
hover, focus, burst, or transition behavior is introduced. Existing procedural
paint need not be replaced, and asset count is not an acceptance metric.
