# Accessible control labels

Named control hooks accept either `LocalizedText` or `AccessibleName`. Use
`text("Sound")` for literal text, `AccessibleName::LabelledBy(vec![label_ref])` for
one or more visible label hosts, or `AccessibleName::Contents` for eligible descendants.
References are concatenated in their authored order with normalized whitespace.
Name-source-only labels remain available for references and contents-derived
names without creating duplicate spoken nodes. `Contents` collects `StaticText`
descendants with `Exposed` or `NameSourceOnly` visibility, skipping hidden
subtrees and the contents of actionable descendants. Explicit names do not
inherit later label changes.

An activation behavior also provides `label_interaction(&control_ref)`.
Attach the ref to the control host and these interaction props to its visible
label or wrapping layout host. The binding focuses the current control and
invokes the same callback as direct or accessibility activation. It adds no
host, semantic node, or local control state.

```rust,ignore
let label_ref = element_ref::use_element_ref();
let input_ref = element_ref::use_element_ref();
let checkbox = accessibility::use_checkbox(ToggleOptions {
    name: AccessibleName::LabelledBy(vec![label_ref.clone()]),
    checked,
    is_disabled: false,
    on_change,
});
let label_click = checkbox.label_interaction(&input_ref);

View::new().interaction_props(label_click).child((
    Label::new("Sound").element_ref(label_ref).semantic(
        SemanticProps::new(SemanticRole::StaticText)
            .name(AccessibleName::text("Sound"))
            .visibility(SemanticVisibility::NameSourceOnly),
    ),
    Button::new("").element_ref(input_ref)
        .semantic(checkbox.semantic)
        .focus_props(checkbox.focus)
        .interaction_props(checkbox.interaction),
))
```

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

A popup trigger uses `accessibility_popup::use_popup_button(PopupButtonOptions {
name, popup: PopupKind::ListBox, expanded, is_disabled, on_press })`. It retains
canonical Button semantics and ordinary button focus and activation. Popup kind
requires explicit expansion state; it neither mounts a popup nor changes that
state. The parent supplies both expansion and value updates. Ordered label refs
can combine a field label and current value without extra spoken stops.

Unity preserves the canonical name in the semantic mirror and appends `listbox
popup` and `collapsed` or `expanded` in the native label. Expanded controls also
set Unity's Expanded state. Every update rebuilds this presentation from the
canonical name, so context never accumulates. Ordinary buttons carry neither
popup nor expansion context.
