# Accessible control labels

Named control hooks accept either `LocalizedString` or `AccessibleName`. Use
`tx("Sound", "Accessible name for the sound control.")` for authored text, `AccessibleName::LabelledBy(vec![label_ref])` for
one or more visible label hosts, or `AccessibleName::Contents` for eligible descendants.
References are concatenated in their authored order with normalized whitespace.
Name-source-only labels remain available for references and contents-derived
names without creating duplicate spoken nodes. `Contents` collects `StaticText`
descendants with `Exposed` or `NameSourceOnly` visibility, skipping hidden
subtrees and the contents of actionable descendants. Explicit names do not
inherit later label changes.

`use_control_label()` allocates the label and control references as one
association. Its `bind_with` method passes the generated accessible name into a
closure and returns both bound properties. Attach them to explicitly selected
hosts with `associated_label` and `associated_control`. The association adds no
host, semantic node, or local control state.

```rust,ignore
let (label, checkbox) = use_control_label().bind_with(|name| {
    accessibility::use_checkbox(
        ToggleOptions::new()
            .name(name)
            .description(AccessibleDescription::text(trox::tx(
                "Controls game audio",
                "Description of the sound control.",
            )))
            .checked(checked)
            .on_change(on_change),
    )
});

View::new().child((
    View::new()
        .associated_label(label)
        .child(accessibility::name_source_text(trox::tx(
            "Sound",
            "Visible label for the sound control.",
        ))),
    Button::new(trox::tx("", "User-facing copy in this example.")).associated_control(checkbox),
))
```

Activation labels focus the current control and invoke the same callback as
direct or accessibility activation. Non-activation behaviors such as sliders
receive focus without proposing a value. `LabelBinding`, `ElementRef`, and
`label_interaction` remain available for layouts that need to place those
properties separately.

Control clicks, including clicks originating in their descendants, mark the
current event as activated. An associated wrapper then leaves it alone. Nested
activation controls also own their clicks. Ordinary ancestor handlers still
receive bubbling events. Default-prevented clicks, disabled controls, and labels
whose control ref is detached neither focus nor propose a value.

Checkbox and switch values remain controlled: each activation proposes the
inverse of the latest rendered value. A parent may accept or reject the proposal;
an external value update does not invoke the callback. Attach the same behavior's
interaction props and ref to exactly one control host. Labels are intended for
pointer interaction and do not gain keyboard focus or accessibility actions.

Public runtime tests in `accessibility_labels.rs` exercise committed semantic
state, nested clicks, focus commands, parent updates, rejection, cancellation,
disabled state, detached refs, and ancestor propagation. `accessibility.rs`
covers referenced and contents-based names through the public hook options.

`Button` accepts logical child content through `child` and `children`. Use an
empty caption when the children provide the complete visible label. Descendant
clicks follow the ordinary logical route to the button, and updating text
preserves the child hosts and references.

A popup trigger uses
`accessibility_popup::use_popup_button(PopupButtonOptions::new().name(name).popup(PopupKind::ListBox).expanded(expanded).on_press(on_press))`.
It retains canonical Button semantics and ordinary button focus and activation.
Popup kind requires explicit expansion state; it neither mounts a popup nor
changes that state. The parent supplies both expansion and value updates.
Ordered label refs can combine a field label and current value without extra
spoken stops. All accessible control option types use the same `#[builder]`
pattern: required semantic and callback props are enforced at compile time,
while descriptions and disabled state retain their defaults.

Unity preserves the canonical name in the semantic mirror and appends `listbox
popup` and `collapsed` or `expanded` in the native label. Expanded controls also
set Unity's Expanded state. Every update rebuilds this presentation from the
canonical name, so context never accumulates. Ordinary buttons carry neither
popup nor expansion context.
