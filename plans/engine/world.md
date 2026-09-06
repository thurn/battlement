# Compose world objects, layouts, and input in Rust

World components describe native Unity objects using the same component system
as UI. Rust selects children, assets, layout, and interaction. Unity loads and
renders generic sprites, meshes, text, hit regions, and effects.

Read this when adding world hosts, card visuals, layout, or input routing.
Related pages: [architecture](architecture.md), [identity](identity.md),
[animation](motion.md), and the playable [Hearts sample](hearts.md).

## Build a card from independent assets

Provide builders for `WorldGroup`, `WorldSprite`, `WorldMesh`, `WorldText`,
`BoxHitRegion`, and `Anchor`, plus cameras, lights, and opaque prefab visuals.
New or migrated Reactant builders use argument-free `new()` and setters.

For example, a card's artwork, text, and hit region are separate children:

```rust
WorldGroup::new().sort_order(view.sort_order).children((
    WorldSprite::new().sprite(view.art).layer(0),
    WorldSprite::new().sprite(assets.frame).layer(1),
    WorldText::new().text(&view.rules).font(assets.rules_font).layer(2),
    BoxHitRegion::new().size(view.hit_size).center(view.hit_center),
))
```

A group establishes transform and optional render sorting. Mixed sprite/text
ordering must work where glyphs overlap sprites. World text supports font
selection, rich text, wrapping, alignment, tint, and opacity. Meshes use
prepared mesh/material assets with explicit scaling and orientation.

Face changes are ordinary conditional children. A card can show normal, compact
table, hidden, or UI representations from the same view data. Badges, outlines,
outcome previews, and conditional action buttons are additional components.
Nested card layouts are normal children, not native prefab behavior.

The hit region is independent of the artwork. For example, a compact table face
can use a smaller collider selected by Rust props. Geometry and input must
change in the same visible commit.

Hearts uses face/back textures on Rust-created surfaces. Chess can retain opaque
piece prefabs, but no sample may use prefab child lookup as a substitute for the
required Rust card-composition example.

## Animate material properties without changing shared assets

Typed material parameters are validated against prepared materials. Per-instance
overrides must not mutate the shared asset or another card using it.

A shared Motion value can drive dissolve on several sprites, while a separate
opacity track fades text:

```rust
WorldSprite::new().sprite(view.art)
    .material(assets.dissolve)
    .parameter(CardShader::Clip, dissolve_progress.clone())
```

Apply the same value to the frame and other affected sprites. A sequence drives
that value, fades text, and schedules a sound or particle burst at a label. Exit
retention keeps the necessary native objects alive until it finishes. Reverse
dissolve uses the same properties with reversed targets. See
[effects](motion.md#combine-sounds-particles-and-material-effects).

## Author attachment points instead of looking inside prefabs

An **anchor** is a transform created by Rust to identify an attachment point.
Typed refs connect effects to it without searching native object names.

For example, a card exposes separate projectile and trail origins:

```rust
WorldGroup::new().children((
    Anchor::new().reference(projectile_origin).position((0.0, 1.0, 0.0)),
    Anchor::new().reference(trail_origin.clone()).position(skin.trail_position),
    Trail::new().asset(assets.trail).attach_to(trail_origin),
))
```

A target explicitly chooses to follow a live anchor or capture its position at
start. Validate required refs and native kinds during preparation. Refs carry
the mounted lifetime ID, so retained effects continue targeting the original
object if the same UUID is later mounted again.

## Layout uses stable sizes in an explicit plane

UI keeps its native pixel-space layout. World Flexbox and Grid use Taffy in
Rust; fan, pile, and arc arrangements use pure Rust functions. Both domains use
familiar layout concepts while retaining their own units.

A world layout declares an origin, orthogonal X/Y basis vectors, available
extent, ordered child IDs and layout boxes, and algorithm parameters:

```rust
WorldFlex::new()
    .plane(table_plane)
    .extent((12.0, 3.0))
    .gap(0.1)
    .children(cards)
```

Layout boxes use explicit sizes or authored bounds and pivots measured at rest.
Do not feed animated bounds back into layout. Enlarging a card for hover must
not push its neighbors away unless game code deliberately changes its layout box
too.

If a layout needs native measurement, wait for a cached, identified rest
measurement before committing the dependent pose. Reject stale measurements.
Changed measurements invalidate dependent ancestors, not unrelated layouts.
Preserve mesh facing and authored geometry unless scaling or alignment is
explicitly requested.

Custom layout functions consume those inputs and return target transforms.
Expose each child's latest target to animation through a live layout reference.
See [automatic movement](motion.md#movement-works-without-configuration).

## Displayed placement need not change the rules location

A browser or prompt can rearrange cards locally without moving them between
rules zones. Rust combines the game view and local interaction state to select
the displayed layout.

For example, opening a deck browser can show the same deck in a grid:

```rust
if browser.is_open() {
    CardGrid::new().cards(view.deck()).into_node()
} else {
    CardPile::new().cards(view.deck()).into_node()
}
```

Keep the same presentation UUIDs when these are alternative locations for the
same live objects. A simultaneous inspection copy gets different UUIDs. Test
browsing, deck-order selection, shop/draft arrangements, and drag return without
introducing changes to authoritative rules state solely for placement.

## Move between UI and world space

A transfer between a UI rectangle and a world object needs an explicit
projection: which camera and world plane apply, which source and destination
rectangles correspond, and how their units relate. Never equate pixel and world
coordinates.

For example, a UI card thumbnail may expand into a world-space inspection card:

```text
read the thumbnail's currently rendered screen rectangle
project its corners through the chosen camera onto the inspection plane
prepare the destination card and a compatible transition visual
move/resize continuously to the destination rectangle
```

Commit logical identity and context changes once. Retained source visuals do not
retain hooks. Begin with an orthographic scene of known dimensions, then verify
the same screen-space continuity on the perspective tabletop. Missing required
projection data fails preparation before visible replacement.

## Route input through the visible component tree

World objects expose click, hover, press, and drag callbacks. Unity performs
geometric hit testing and pointer capture. Rust decides eligibility, actions,
and capture/bubble propagation along logical ancestry, including portals.

The following ordering resolves overlapping targets:

- Native UI blocks world targets unless the surface explicitly permits
  passthrough.
- The top applicable modal admits only targets within its declared scope.
- World candidates are ordered by interaction layer, visible depth, then stable
  sibling order for exact ties.

For example, opening a menu over a card prevents a click from playing that card,
even if the card's collider is geometrically under the pointer. A decorative UI
overlay can explicitly permit passthrough.

Capture remains attached to the same live target until release or removal.
Reparenting is not removal. Removal emits capture loss; stale commit or lifetime
IDs cannot deliver old drag callbacks to a new mount. Native default prevention
stays synchronous without reentrant engine calls.

Gameplay input answers the current presented prompt or starts an action at a
completed-action boundary. Inspection, settings, and menus remain usable while
rules or required animation are pending.

## Keyboard, controller, touch, and resizing

World hit regions expose focus and semantic activation to the logical tree.
Provide directional navigation using eligibility and displayed geometry; do not
implement controller activation by synthesizing mouse coordinates. Focus needs a
visible world indication and returns to its invoker after closing a modal.

Touch uses stable pointer IDs and the same modal/capture rules as mouse input.
An invalid drop or lost capture returns a dragged card to its latest valid
layout without dispatching a play. Separate touch IDs do not share capture.

Viewport changes update affected extents and projection, then retarget existing
motion. Hit regions and focus must agree with the committed transforms during
resize or reorientation. Preserve existing sample input mappings.

## Manual QA

Inspect a rich card with overlapping text and sprites, change its face and hit
region, and compare two instances with different material overrides. Resize a
fan while a card is moving. Move between UI and world views with orthographic
and perspective cameras. Exercise overlapping UI, passthrough, nested modals,
keyboard/controller focus, and removal during a captured touch drag.
