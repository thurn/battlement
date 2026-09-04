# Control labels and semantic names

Shared text controls derive their semantic name from the same localized value
rendered by Unity. This keeps localization updates visible and spoken in one
atomic render.

```rust
Button::new(ls("Save"))
  .on_press(|game: &mut Game| game.save())
```

Composed and icon-only buttons must supply a name explicitly:

```rust
Button::content(icon())
  .semantic_name(SemanticName::Text(ls("Save")))
  .on_press(|game: &mut Game| game.save())
```

`SemanticName::LabelledBy` resolves text from live logical hosts in authored
order. `SemanticName::Contents` gathers eligible static-text descendants while
skipping hidden subtrees and nested controls. `Text::name_source` publishes
visible text for naming without adding another spoken node.

Advanced custom controls can allocate an external label and control reference
with `use_control_label()`. Its `bind_with` closure receives the derived
`SemanticName` and returns atomic `associated_label` and `associated_control`
properties. Label activation focuses the live control and follows the same
callback and disabled gating as direct activation.

```rust,ignore
let (label, control) = use_control_label().bind_with(|name| {
  control_behavior::checkbox(name, None, checked, disabled, on_change)
});

View::new().child((
  View::new()
    .associated_label(label)
    .child(Text::name_source(ls("Sound"))),
  ToggleHost::new()
    .value(checked)
    .associated_control(control),
))
```

Descriptions use `SemanticDescription::Text` or a live `DescribedBy` reference.
Host query identity is independent: shared components use `host_name(...)`,
while raw hosts retain their native `name(...)` property.
