# Chess UI motion requirements

[Plan and reading guide](../chess-ui-implementation-plan.md)

## Mockup translation coverage

The acceptance source is `~/Documents/mockups` at Git commit
`2451ea9cc6f76b356b1102ee37b82c478853122a`. The ledger below is a manually
reviewed requirements checklist. It does not create a source analyzer,
manifest, generated fixture, mirrored gallery, or automated coverage check.

Implementation reviews the pinned source directly and exercises the complex
animation families through focused Rust tests or the existing Reactant sample.
Simple declarations do not each require a separate test.

The acceptance criterion is API and behavior coverage. Pixel identity and
matching the browser's exact intermediate trajectory are not required. Values,
times, easing, repetition, presence policy, and interruption semantics are
preserved unless an entry explicitly names a paint approximation.

### Coverage ledger

- `BackgroundMusic.tsx:139`: audio-synchronized control heartbeat. Read the
  stable audio playback handle through `MotionTimeSource::Audio`, reproduce
  `heartbeatStrength` with serializable modulo, minimum, clamp, and exponential
  expression nodes, and derive shared scale, brightness, and glow motion
  values. Distribute those values through ordinary Reactant context. Paused,
  stalled, ended, and reduced-motion states resolve to zero pulse strength.

- `SettingsTabs.tsx:90`: tab host. Target `y = active ? 0 : 3`, hover
  `y = active ? 0 : -1`, tap `scale = 0.955`, spring stiffness `520`, damping
  `32`, mass `0.7`. Use `Button` target, hover, and tap builders. Reduced motion
  retains color/opacity feedback and removes spatial movement.

- `SettingsControls.tsx:245`: dropdown-button transform `90ms` cubic Bézier
  `(.2,.8,.2,1)` and filter `140ms ease`. Use typed pseudo styles and
  `StyleTransition`.

- `SettingsControls.tsx:268`: dropdown menu presence. Enter from opacity `0`,
  `y = -12`, `scale_y = .76` to `1, 0, 1` in `.2s` with
  `(.2,.8,.25,1)`; exit to `0, -7, .42` in `.26s` with
  `(.4,0,.75,.3)`. Use `AnimatePresence` and a Reactant host façade with
  inherent Motion builders.

- `SettingsControls.tsx:317`: dropdown option presence. Enter from opacity `0`
  and `x = -17` in `.18s ease-out`, delayed `index * .028s`; exit to opacity
  `0`, `x = 10`. Use keyed Motion children and per-item delay.

- `SettingsControls.tsx:377`: option host interaction. Translate its target and
  gesture declarations directly onto the option host; use the transition entry
  below for CSS-owned properties.

- `SettingsControls.tsx:417`: option transform `90ms`
  `cubic-bezier(.2,.8,.2,1)`, box shadow and filter `140ms ease`. Use typed
  pseudo styles and `StyleTransition`.

- `SettingsControls.tsx:423`: selected-option flash. Opacity `.9 -> 0`, scale
  `.96 -> 1.035`, `.38s ease-out`, with `.01s` reduced-motion duration. Use a
  keyed child under `AnimatePresence`.

- `SettingsControls.tsx:478`: toggle label transform `140ms ease`. Use typed
  pseudo styles and `StyleTransition`.

- `SettingsControls.tsx:588`: checkbox transform `90ms`
  `cubic-bezier(.2,.8,.2,1)`, filter `90ms ease`, border `140ms ease`, and box
  shadow `140ms ease`. Use a multi-property `StyleTransition`.

- `SettingsControls.tsx:664`: control transform `90ms`
  `cubic-bezier(.2,.8,.2,1)`. Use `StyleTransition`.

- `SoundSettings.tsx:179`: slider transform `90ms`
  `cubic-bezier(.2,.8,.2,1)`, filter `90ms ease`, and box shadow `140ms ease`.
  Use a multi-property `StyleTransition`.

- `SoundSettings.tsx:240`: transform `90ms`
  `cubic-bezier(.2,.8,.2,1)` and filter `140ms ease`. Use typed pseudo styles
  and `StyleTransition`.

- `InputSettings.tsx:237`: binding blink opacity `[1, 1, .08, .08, 1]`, `1.05s`
  linear, infinite. Use a Motion keyframe target with `Repeat::Forever`.
  Reduced motion leaves the indicator at its readable static value.

- `InputSettings.tsx:334`: transform `90ms`
  `cubic-bezier(.2,.8,.2,1)` and filter `140ms ease`. Use
  `StyleTransition`.

- `ActionButton.tsx:87`: transform `90ms`
  `cubic-bezier(.2,.8,.2,1)`, filter `140ms ease`, and background `140ms ease`.
  The tap gesture layer owns the pressed transform. Typed hover and focus
  pseudo-styles own filter and background, so no property is driven by two
  layers for the same interaction.

- `ControlInteraction.tsx:41`: `control-shine-sweep`, `720ms ease-out`, one
  iteration, `Both` fill. Use a decoration CSS `Animation`; reduced motion
  installs no animation.

- `ArcadeAttractMode.tsx:125`: perspective grid breathing. Preserve the source
  keyframes, duration, alternate direction, and infinite iterations in a CSS
  `Animation` on grid chrome.

- `ArcadeAttractMode.tsx:169`: 48 particle loops. Preserve each seeded size,
  color, position, drift, duration, and negative phase delay. Use keyed
  decorations and `AnimationIterations::Forever`; reduced motion is static.

- `ArcadeFramePulse.tsx:111`: two border comets. Preserve the `6.5s` linear
  infinite nine-frame left/top/rotation path and settings cutout mask. Use two
  decorations sharing one `Keyframes` value.

- `ArcadeMenuTransition.tsx:88`: routed screen presence. Preserve source
  variants, `.3s` duration, `.17s` delay, and `(.16,1,.3,1)` easing. Use `Sync`
  normally and `Wait` for the source's conservative backend branch.

- `ArcadeMenuTransition.tsx:138`: contained beam. Opacity
  `[0,.72,0]`, scale-x `[.15,1,.72]`, `.3s`, times `[0,.48,1]`. Use a
  decoration Motion target.

- `ArcadeMenuTransition.tsx:177`: reveal scan. Clip inset
  `[49.7%,46%,0%]`, opacity `[0,.48,0]`, times `[0,.44,1]`, and
  `(.65,0,.35,1)`. Use rectangular clip and opacity channels.

- `ArcadeMenuTransition.tsx:208`: transition beam. Preserve its literal source
  target, keyframe, duration, time, and easing values in a decoration case.

- `ArcadeTabTransition.tsx:77`: directional panel. Use typed direction custom
  data, named enter/center/exit variants, `AnimatePresence::custom`, and
  `PopLayout`; preserve the variant-local transition values at lines `16-25`.

- `ArcadeTabTransition.tsx:108`: directional light sweep. X starts at `-90` or
  `940` and crosses to the other value; opacity `[0,.68,.68,0]`, `.34s`, times
  `[0,.22,.72,1]`, easing `(.4,0,.2,1)`. Skew applies to custom sweep
  geometry, not the live panel subtree.

- `ArcadeTabTransition.tsx:140`: scan line. Y `-12 -> 1000`, opacity
  `[0,.38,.22,0]`, `.42s` linear, times `[0,.1,.72,1]`. Use a decoration
  Motion target.

- `ArcadeModal.tsx:82`: backdrop presence. Opacity `0 -> 1 -> 0`, `.2s`, or
  `.01s` under reduced motion. Use `AnimatePresence`.

- `ArcadeModal.tsx:104`: modal panel. Preserve all source opacity, scale-x,
  scale-y, x, skew-x, and filter frames, `.42s` entry or `.3s` exit, and
  `ease-out`. The three-frame exit uses even per-property spacing when the
  four-value times array is incompatible. Skew deforms panel chrome while live
  content remains undeformed.

- `ArcadeModal.tsx:168`: modal shine. X `-115% -> 115%`, `1.8s` linear,
  infinite Motion repetition with `1.2s` repeat delay. Use a decoration target;
  reduced motion omits it.

- `ArcadeCheckboxEffect.tsx:33`: checked burst root. Key by activation; retain
  with presence and exit opacity in `.04s`.

- `ArcadeCheckboxEffect.tsx:40`: checkbox ring. Preserve its source opacity,
  scale, rotation, duration, and easing in a decoration target.

- `ArcadeCheckboxEffect.tsx:53`: checkbox flash. Preserve its source opacity,
  scale, duration, and easing in a decoration target.

- `ArcadeCheckboxEffect.tsx:69`: checkbox beam. Preserve its source transform,
  opacity, duration, and easing in a decoration target.

- `ArcadeCheckboxEffect.tsx:87`: checkbox sparks. Preserve per-spark opacity,
  x, y, rotation, scale-x, duration, delay, and easing on keyed decorations.

- `ArcadeButtonEffect.tsx:32`: button burst root. Key by activation and preserve
  the root presentation values.

- `ArcadeButtonEffect.tsx:46`: button ring. Preserve source opacity, scale,
  rotation, duration, and easing on a decoration.

- `ArcadeButtonEffect.tsx:61`: button beam. Preserve source scale-x, opacity,
  duration, and easing on a decoration.

- `ArcadeButtonEffect.tsx:79`: button particles. Preserve seeded particle
  opacity, x, y, rotation, scale-x, duration multiplier, `index * .008s` delay,
  and `(.2,.82,.32,1)` easing.

- `ArcadeSliderEffect.tsx:24`: slider burst root. Key by activation and keep
  opacity at `1`.

- `ArcadeSliderEffect.tsx:38`: slider ring. Opacity `[.9,.65,0]`, scale
  `[.72,1.3,1.6]`, rotate `-16 -> 12`, base duration `.66s`, easing
  `(.16,.8,.35,1)`.

- `ArcadeSliderEffect.tsx:52`: slider particles. Preserve opacity, x, y,
  rotation, scale-x, duration multiplier, `index * .01s` delay, and
  `(.2,.85,.35,1)` easing.

- `ArcadeExitSequence.tsx:29`: exit flash. Opacity `[0,.7,.25,0]`, `.36s`,
  times `[0,.25,.65,1]`, ease-out.

- `ArcadeExitSequence.tsx:42`: expanding beam. Opacity `[0,.9,0]`, scale-y
  `[.2,1,.08]`, `.43s`, times `[0,.34,1]`, easing `(.2,.8,.2,1)`.

- `ArcadeExitSequence.tsx:57`: top line. Top `[7%,50%,50%]`, opacity
  `[0,.78,0]`, `.5s`, times `[0,.72,1]`, easing `(.7,0,.3,1)`.

- `ArcadeExitSequence.tsx:72`: bottom line. Bottom `[7%,50%,50%]`, opacity
  `[0,.78,0]`, `.5s`, times `[0,.72,1]`, easing `(.7,0,.3,1)`.

- `ArcadeExitSequence.tsx:87`: central collapse. Opacity `[0,0,1,.92,0]`,
  scale-x `[.08,.08,1,.32,.01]`, scale-y `[.5,.5,1.9,.5,.1]`, shared exit
  duration, times `[0,.52,.72,.87,1]`, ease-out.

- `MainMenu.tsx:101`: main content exit. Preserve the five-frame clip, filter,
  opacity, scale, and x targets, times `[0,.14,.38,.73,1]`, easing
  `(.65,0,.35,1)`, and shared exit duration. Reduced motion fades in `.08s`.

- `ScreenFrame.tsx:44`: frame exit. Preserve the five-frame clip, filter,
  opacity, scale, and x targets with the same clock as the main content. Its
  independent x values remain literal. Reduced motion fades in `.08s`.

Every source `useReducedMotion` branch is part of the checklist. Inherited
`ReducedMotion::User` supplies the normal mapping; a component whose source has
a custom static value uses `use_reduced_motion` and builds that value directly.
Manual review exercises each relevant pattern with motion enabled and reduced
motion forced.

Static gradients, glows, masks, and shadows may use ordinary styles, prepared
textures, custom UI Toolkit geometry, or a shader. These paint choices cannot
alter timing, presence, gesture, or reduced-motion behavior.
