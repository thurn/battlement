# World primitives, assets, layout, and input

Read this for world hosts, layout algorithms, interaction routing, or UI/world
transfers. See [identity](identity.md), [motion](motion.md), and
[Hearts](hearts.md).

## Rust-composed objects

Provide WorldGroup, WorldSprite, WorldMesh, WorldText, BoxHitRegion, Anchor, and
an opaque visual prefab component. Camera and lights are generic world hosts as
well. Components own composition; Unity loads independent assets and renders the
described hierarchy.

A group establishes transform and optional render sorting. Sprites and world
text share explicit relative ordering; rich text, wrapping, font choice,
alignment, and text opacity are available without game-specific C#. WorldMesh
accepts prepared mesh/material assets and explicit orientation/scale. Geometry
authoring never depends on reading an unprepared prefab child.

~~~rust
WorldGroup::new().children((
    WorldSprite::new().sprite(face).layer(0),
    WorldText::new().text(label).font(font).layer(1),
    BoxHitRegion::new().size(hit_size),
    Anchor::new().reference(spark_origin).position(offset),
))
~~~

Material property names come from typed declarations validated against prepared
materials. Per-instance overrides cannot mutate the shared material asset.
Motion values may drive several renderers and a separate text-opacity track.
Refs expose typed Rust-created anchors with incarnation-aware lifetimes.

Hearts uses face/back textures on Rust-created card surfaces. Chess can load
opaque model prefabs. A prefab does not expose typed child parts or become a
shortcut around the Hearts primitive demonstration.

## World layout

Use Taffy for world Flexbox and Grid, with authored world-unit sizes and a
declared plane: origin, orthogonal X/Y basis, and available extent. UI keeps its
native pixel-space layout; do not run Taffy over UI Toolkit trees.

A layout input contains ordered presentation identities, stable layout boxes and
pivots, and algorithm parameters. Custom fan, pile, and arc layouts are pure
functions returning target transforms. Explicit sizes or authored rest bounds
are the default; never measure the animated current bounding box as layout
input.

If runtime measurement is required, cache a correlated host rest measurement and
defer the dependent pose commit until available. Changed measurements invalidate
dependent ancestors. Preserve mesh facing and authored geometry unless
scaling/orientation is explicitly requested.

Game Rust maps rules locations and local interaction state to displayed layout
membership. A browser or inspection view can change placement without changing
the rules zone. Separate inspection copies have distinct presentation UUIDs.

## UI/world projection

For cross-domain transfers, require a projection specifying camera, destination
plane, source/destination rectangle correspondence, and units. Project the
current rendered source geometry to a temporary compatible visual
representation; prepare the destination host and move/resize through that
representation. Commit identity/context changes once; retained source visuals do
not own hooks.

Use an orthographic fixture first with known dimensions, then a perspective
table fixture. Verify screen-space continuity by rendered geometry, not raw
equality between pixels and world units.

## Input arbitration

Hit testing and pointer capture are host capabilities. Rust controls game
eligibility, logical event propagation, actions, and display interaction state.

- Native UI blocks world hits unless its surface explicitly permits passthrough.
- The top applicable modal admits only its declared interaction scope.
- Order world candidates by interaction layer, visible depth, then stable
  sibling order for exact ties.
- Capture and bubbling use committed logical ancestry, including portals.
- Captured pointers retain their target until release or removal; removal emits
  capture loss. Reparenting alone is not removal.
- Reject events from stale commit generations or mounted incarnations.
- Keep default prevention synchronous and avoid reentrant engine calls.

At a prompt, gameplay input can only answer that prompt. At an accepted
completed-action boundary, a new action may begin. Settings, menus, and
inspection remain responsive while presentation or rules work is pending.

## Keyboard, controller, and touch

Reuse the existing focus/navigation behavior where applicable. World hit regions
expose an explicit focusable/activation surface to the logical tree. Provide
directional navigation policy and semantic activation without simulating mouse
coordinates. Focus indication must be visible in the rendered world.

Touch uses stable pointer IDs and capture. A drag that loses capture returns to
the latest valid layout without dispatching a play. World and UI modal
arbitration is identical for touch and mouse.

Viewport changes invalidate affected layout extents/projection and retarget
existing motion. UI and world input continue to reference the committed
generation; a resized hit region must not be paired with stale transforms.

## Manual QA

Overlap UI and world targets, toggle passthrough, then open nested modals. Drag
a card across a portal/layout change and remove it while captured.
Resize/reorient during motion and confirm visible targets, focus, and hit
regions agree. Inspect two cards with independent material overrides.
