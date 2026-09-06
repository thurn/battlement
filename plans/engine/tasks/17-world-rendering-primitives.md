# 17. Add sprites, meshes, world text, and material overrides

Rust can compose neutral card visuals from independently prepared assets with
predictable sorting and per-instance properties.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [World objects and input](../world.md)
- [Presentation timing](../presentation.md)
- [Identity and state](../identity.md)

**Prerequisite:** [Task 16: Add stable snapshot selectors and queued display
stores](16-snapshot-selectors-stores.md) and all its required follow-ups must be
integrated.

**Starting code:** World adapter; object protocol/builders; Unity world
creation; prepared assets; fake asset catalog.

## Example

Build the visual hierarchy from independent assets in Rust:

```rust
WorldGroup::new().children((
    WorldSprite::new().sprite(art).layer(0),
    WorldText::new().text(rules).font(font).layer(1),
))
```

## Implementation

1. Add typed sprite, mesh, and world text descriptors/builders and their
   Rust/Unity/fake implementations. Extend asset preparation for
   mesh/font/material dependencies as needed.

2. Implement group-relative mixed sprite/text ordering, rich text, wrapping,
   alignment, tint/opacity, explicit geometry scale/orientation, and
   independently selectable front/back children.

3. Validate typed material parameter declarations against prepared assets and
   apply per-instance overrides without mutating the shared material. Expose
   these properties to later Motion adapters.

4. Create an early composed-card test scene with two cards using one shared
   material but different static parameters and hit-independent visual geometry.

## Acceptance

- Sprites and rich text render in the declared order, including overlapping
  glyph/sprite regions in a native capture.

- Changing one card's material parameter does not change the other card or the
  source material.

- Missing required assets fail preparation before visible replacement; prepared
  incompatible children replace without a partial frame.

- Mesh scale/orientation and text wrapping match explicit props in fake
  observations and native geometry.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Hit regions/anchors are task 18; animated material values are task 21; full
rich-card laboratory coverage is task 44.

## Manual QA

Inspect both cards natively, change face/text/overrides, and verify sorting,
wrapping, and independent appearance.
