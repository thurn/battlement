# Shared Reactant components

Reactant controls include their native interaction, focus, semantic, and motion
contracts. Applications should author controls from `prelude::*`; accessibility
is an invariant of those components rather than an optional hook layer.

```rust
Button::new(ls("Save"))
  .disabled(saving)
  .on_press(|game: &mut Game| game.save())
```

Actionable components use type-state builders. `Button`, `Checkbox`, `Switch`,
`Radio`, `Tabs`, `Slider`, and similar controls implement `Render` only after
their authoritative callback is supplied. `Tabs::on_select` is the single
selection callback for every descendant `Tab`; tabs and panels derive selected
state from the controlled index on their nearest `Tabs`. Text buttons derive
their semantic name from the same localized value rendered by Unity. Composed
button content must call `semantic_name(...)` before it can render.

The shared layer includes:

- `Button`, `PopupButton`, `Link`, and `Disclosure`;
- `Checkbox`, `Switch`, `RadioGroup`, and `Radio`;
- `Tabs`, `Tab`, and `TabPanel`;
- `Slider`, `Progress`, and `ScrollArea`;
- list-box, table, landmark, heading, image, and text components; and
- named modal dialog behavior through `Overlay::modal`.

Controlled state is authored once on the component. `disabled(...)` governs
native enabled state, focusability, semantic state, activation gating, and
disabled pseudo-styling together. Checked, selected, expanded, current-page,
popup, and range values likewise remain controlled application values.

Native UI Toolkit controls provide pointer, touch, keyboard, and controller
behavior. Direct assistive actions reach the same stored callback. Hover,
active, focus-visible, and disabled visuals are native Motion pseudo-states and
do not require Reactant state renders.

## Advanced custom controls

Custom controls may use `host::*Host` façades with `ControlBehavior`. Raw hosts
are intentionally absent from the prelude. `ControlBehavior` atomically carries
semantic, focus, interaction, and motion declarations and does not expose
presentation state.

Semantic data uses `SemanticName`, `SemanticDescription`, `SemanticRange`,
`SemanticProps`, `SemanticRole`, `SemanticState`, and `SemanticVisibility`.
`AccessibilityAction` and snapshot types retain their platform-boundary names.

Visible external labels can use `use_control_label` with an advanced behavior.
Names derived from live hosts continue to resolve after localization updates,
through transparent logical hosts, and across portals.

Collection components own their roles and validate logical ancestry before a
commit. `RadioGroup` and `Tabs` also own runtime membership references and pass
them through Reactant context, so callers never coordinate group refs.

## Modal overlays

`Overlay::modal(target, name)` installs dialog semantics and the existing focus
trap/restoration contract. Add `on_dismiss(...)` only when the application can
close the modal in response to a direct dismiss action.

```rust
Overlay::modal(overlay_target, ls("Settings"))
  .on_dismiss(|game: &mut Game| game.settings_open = false)
  .child(settings_panel())
```
