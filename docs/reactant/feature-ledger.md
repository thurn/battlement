# Reactant V1 Feature Ledger

This ledger maps every public `battlement-reactant` module to the sample
screen that teaches its purpose and to black-box tests that prove its public
behavior. The release test reads the crate module declarations and this table,
so an added public module cannot ship without a screen and test mapping.

The screen is a focused specimen, not an exhaustive API catalog. The listed
tests remain the authoritative behavior proof when a capability is broader
than the specimen.

## Shipped modules

| Public module | Lab screen | Black-box proof |
|---|---|---|
| `accessibility` | Layout Gallery | `accessibility.rs::complete_snapshot_resolves_contents_and_prunes_hidden_subtrees`, `accessibility.rs::accessibility_activation_uses_the_ordinary_logical_event_path` |
| `accessibility_collections` | Layout Gallery | `accessibility.rs::collections_preserve_roles_ancestry_current_page_and_controlled_selection`, `accessibility.rs::invalid_collection_relationships_and_page_states_fail_before_commit` |
| `accessibility_popup` | Chess UI: SelectControl | `accessibility_popup.rs::popup_button_keeps_one_host_and_controlled_context_across_updates_and_activation`, `accessibility_popup.rs::malformed_popup_declarations_fail_as_developer_errors` |
| `animation_controls` | Values, Time & Controls | `motion.rs::typed_motion_values_controls_and_scopes_lower_closed_native_contract` |
| `announcement` | Layout Gallery | `accessibility.rs::complete_snapshot_resolves_contents_and_prunes_hidden_subtrees` |
| `app` | Composition | `app_composition.rs::scene_only_app_connects_and_reconnects_without_creating_a_ui_document`, `app_lifecycle.rs::generated_application_snapshot_and_mixed_callbacks_work_without_a_custom_engine`, `app_lifecycle.rs::reconnect_policy_controls_remounts_and_drop_runs_cleanup_once`, `app_assets.rs::consecutive_responses_wait_for_preparation_and_keep_prior_dependencies` |
| `app_context` | Layout Gallery | `app_lifecycle.rs::reconnect_policy_controls_remounts_and_drop_runs_cleanup_once`, `composition.rs::sample_recomposes_when_the_viewport_crosses_the_compact_breakpoint` |
| `builder_support` | Composition | `composition.rs::composition_action_reorders_and_restores_the_badges` |
| `callback` | Layout Gallery | `app_lifecycle.rs::generated_application_snapshot_and_mixed_callbacks_work_without_a_custom_engine`, `app_lifecycle.rs::ui_disposition_is_synchronous_and_old_session_events_are_rejected` |
| `cooperative_executor` | Resources & Boundaries | `cooperative_executor.rs::self_waking_work_is_bounded_and_cancellation_prevents_further_polls` |
| `resource_control` | Resources & Boundaries | `app_resources.rs::pending_resources_wake_refetch_and_cancel_without_an_author_executor` |
| `application` | Layout Gallery | `application.rs::lifecycle_context_updates_memoized_consumers_and_preserves_preview_overrides` |
| `asset_generator` | Assets | `composition.rs::assets_screen_prepares_mockup_paint_and_resizes_then_restores_the_action_frame`, `generated_assets.rs::generated_image_lowers_to_exactly_one_native_image_host` |
| `component` | Composition | `composition.rs::composition_action_reorders_and_restores_the_badges`, `primitives.rs` |
| `element_behavior` | Chess UI: Gallery shell | `chess_gallery.rs::gallery_selection_recreates_each_harness_and_restores_heading_focus` |
| `label_binding` | Chess UI: ToggleControl | `chess_gallery.rs::checkbox_accepts_one_proposal_and_parent_updates_reset_authoritatively`, `chess_gallery.rs::closed_selection_uses_parent_value_and_resets_without_proposals` |
| `scale_to_fit` | Chess UI: PortraitViewport | `app_composition.rs::fitted_content_uses_the_area_inside_viewport_decoration_without_remounting` |
| `context` | Context & Memo | `composition.rs::context_screen_overrides_only_the_nested_descendant_and_restores`, `refs_context.rs` |
| `element_ref` | Refs & Geometry | `composition.rs::refs_screen_samples_world_geometry_and_restores_an_unavailable_target`, `element_refs.rs` |
| `error_boundary` | Resources & Boundaries | `composition.rs::resources_screen_catches_resets_and_restores`, `error_boundaries.rs` |
| `event` | Events & Portals | `composition.rs::events_screen_runs_and_restores_one_logical_event_path`, `event_catalog.rs`, `propagation.rs` |
| `executor` | Resources & Boundaries | `composition.rs::resources_screen_catches_resets_and_restores`, `resources.rs` |
| `external_store` | Effects & Stores | `composition.rs::effects_store_swaps_updates_and_restores_its_external_snapshot`, `external_stores.rs` |
| `focus` | Layout Gallery | `focus_protocol_tests.rs::complete_tree_accepts_one_auto_focus_and_inertness`, `BattlementFocusCoordinatorTests.cs` |
| `geometry` | Refs & Geometry | `composition.rs::refs_screen_samples_world_geometry_and_restores_an_unavailable_target`, `geometry.rs`, `geometry_effects.rs` |
| `gesture` | Gestures & Drag | `motion.rs::gesture_drag_scroll_and_viewport_props_lower_native_contract` |
| `hooks` | State & Identity | `composition.rs::state_screen_batches_updates_preserves_keyed_state_and_restores`, `hook_scheduling.rs`, `state.rs`, `use_id.rs` |
| `key` | State & Identity | `composition.rs::state_screen_batches_updates_preserves_keyed_state_and_restores`, `identity.rs`, `moves.rs` |
| `layout` | Layout & Reorder | `motion.rs::layout_projection_shared_handoff_and_reorder_lower_native_contract`, `composition.rs::layout_gallery_preserves_state_routes_portals_and_authors_modal_focus`, `composition.rs::layout_performance_builds_the_exact_mixed_workload`, `LayoutProjectionTests.cs` |
| `motion` | Targets & Timelines | `motion.rs::host_methods_interleave_without_restarting_or_adding_a_host`, `motion.rs::public_targets_serialize_keyframes_overrides_repeats_and_transition_end`, `motion.rs::forwarding_component_collects_complete_props_without_a_wrapper_host` |
| `motion_config` | Composed Effects | `motion.rs::motion_config_inherits_transition_and_reduced_motion_without_a_host` |
| `motion_value` | Values, Time & Controls | `motion.rs::typed_motion_values_controls_and_scopes_lower_closed_native_contract` |
| `overlay` | Events & Portals | `composition.rs::events_screen_runs_and_restores_one_logical_event_path`, `composition.rs::layout_gallery_preserves_state_routes_portals_and_authors_modal_focus`, `primitives.rs::overlay_helpers_resolve_first_mount_refs_and_wrap_fragment_children` |
| `paint` | Chess UI: ScreenFrame | `paint.rs::static_paint_updates_and_removal_preserve_the_host_and_gesture_generation`, `paint.rs::static_only_paint_has_no_animation_slots_or_entrance_target` |
| `portal` | Events & Portals | `composition.rs::events_screen_runs_and_restores_one_logical_event_path`, `portals.rs`, `external_portals.rs` |
| `prelude` | Composition | `composition.rs::sample_opens_on_an_accessible_composition_screen`, `primitives.rs` |
| `presence` | Presence & Lifecycle | `presence.rs::automatic_exit_retains_hooks_until_exact_generation_completion`, `presence.rs::manual_hold_reconnect_and_rapid_reopen_preserve_one_mount` |
| `host` | Composition | `composition.rs::sample_opens_on_an_accessible_composition_screen`, `primitives.rs` |
| `props` | Composition | `composition.rs::composition_action_reorders_and_restores_the_badges`, `required_props.rs` |
| `render` | Composition | `composition.rs::composition_action_reorders_and_restores_the_badges`, `runtime.rs`, `identity.rs` |
| `resource` | Resources & Boundaries | `composition.rs::resources_screen_catches_resets_and_restores`, `resources.rs` |
| `runtime` | Effects & Stores | `composition.rs::effects_screen_defers_connection_until_poll_and_restores`, `runtime.rs`, `lifecycle.rs` |
| `semantics` | Layout Gallery | `accessibility.rs::complete_snapshot_resolves_contents_and_prunes_hidden_subtrees`, `accessibility.rs::protocol_round_trips_direct_actions_and_complete_snapshot` |
| `suspense` | Resources & Boundaries | `composition.rs::resources_screen_catches_resets_and_restores`, `resources.rs` |

The remaining focused screens exercise the same public surface from another
angle: Context & Memo covers memoized values and callbacks, Effects & Stores
covers passive effects, Refs & Geometry covers queued host actions and coherent
geometry effects, Assets covers generated advanced paint, and Physical Motion
covers springs, inertia, velocity handoff, and terminal playback outcomes.
Styles & Decorations covers pseudo
precedence, CSS playback, keyed decoration identity, composition, and advanced
paint. Variants & Orchestration covers typed maps, custom-data snapshots,
ordered targets, logical propagation, opt-out, and bidirectional stagger.
Public physical, CSS, and variant authoring are checked
in `crates/battlement-reactant/tests/motion.rs`; controlled native behavior is
checked in `Packages/com.battlement.client/Tests/Editor/PhysicalMotionTests.cs`
and the adjacent CSS and variant motion tests.
Presence & Lifecycle demonstrates retained exits, interruption, Wait mode,
manual removal holds, reconnects, and callback boundaries. Its logical
lifecycle proof lives in `crates/battlement-reactant/tests/presence.rs`, while
the native Ditto scenario records the rendered transition and event trace.
Values, Time & Controls demonstrates shared Unity-local graphs, typed ranges,
springs, explicit checkpoints, controlled and audio time, broadcast controls,
selector snapshots, and ordered sequences. Native dirty propagation, clock,
coalescing, and discontinuity behavior is checked in
`Packages/com.battlement.client/Tests/Editor/MotionWorldTests.cs`.
Gestures & Drag demonstrates device-aware hover, focus, tap and pan boundaries,
constrained momentum, external drag controls, native motion-value outputs,
scroll progress, and viewport state. Deterministic pointer and geometry behavior
is checked in `Packages/com.battlement.client/Tests/Editor/GestureMotionTests.cs`.
Layout & Reorder demonstrates position and size projection, nested scale
correction, shared handoffs, projection-aware scrolling, drag reorder, and
presence-owned `PopLayout` removal. Controlled geometry behavior is checked in
`Packages/com.battlement.client/Tests/Editor/LayoutProjectionTests.cs`.
Composed Effects combines dropdown, modal, directional route, keyed control
burst, pseudo-state, ambient, audio-time, reduced-motion, and reconnect
specimens. `MotionConfig`, the platform preference bridge, and reconnect phase
adoption are checked by the Reactant Motion and controlled Unity tests.
Motion Performance instantiates the fixed `transform-200`, `mixed-200`, and
mixed interaction workloads from public builders. Its fast black-box proof
checks the emitted host, graph, subscription, and timeline structure; native
and WebGL Release profiling remains in the on-demand release lane.

### Pinned settings mockup animation coverage

The pinned settings mockup at commit
`2451ea9cc6f76b356b1102ee37b82c478853122a` was reviewed directly. Every
animation ledger entry maps to a public Reactant API and a named sample
specimen; several related declarations intentionally share one composition.

| Pinned source | Public Reactant API | Named specimen |
|---|---|---|
| `BackgroundMusic.tsx:139` | `AudioPlayback`, `use_motion_time`, `use_transform` | `composed-audio-pulse` |
| `SettingsTabs.tsx:90` | `while_hover`, `while_tap`, `Transition::spring` | `composed-routes-specimen` |
| `SettingsControls.tsx:245` | `hover_style`, `active_style`, `StyleTransition` | `composed-dropdown-specimen` |
| `SettingsControls.tsx:268` | `AnimatePresence`, `initial`, `animate`, `exit` | `composed-dropdown-specimen` |
| `SettingsControls.tsx:317` | `key`, `Transition::delay_secs` | `composed-dropdown-specimen` |
| `SettingsControls.tsx:377` | `MotionStyle`, gesture target builders | `composed-dropdown-specimen` |
| `SettingsControls.tsx:417` | `StyleTransition`, `MotionFilter`, `MotionShadow` | `composed-dropdown-specimen` |
| `SettingsControls.tsx:423` | `AnimatePresence`, `key`, `MotionStyle` | `selection-flash` |
| `SettingsControls.tsx:478` | `StyleTransition::property` | `composed-interactions-specimen` |
| `SettingsControls.tsx:588` | `StyleTransition::all`, typed pseudo styles | `composed-checkbox` |
| `SettingsControls.tsx:664` | `active_style`, `StyleTransition` | `composed-interactions-specimen` |
| `SoundSettings.tsx:179` | `MotionStyle`, `StyleTransition` | `composed-slider` |
| `SoundSettings.tsx:240` | typed pseudo styles, `MotionFilter` | `composed-slider` |
| `InputSettings.tsx:237` | `Animation`, `Keyframes`, `AnimationIterations::Forever` | `composed-binding` |
| `InputSettings.tsx:334` | `StyleTransition` | `composed-interactions-specimen` |
| `ActionButton.tsx:87` | `hover_style`, `focus_style`, `while_tap` | `composed-checkbox` |
| `ControlInteraction.tsx:41` | `Decoration`, `Animation`, `AnimationFill::Both` | `modal-shine` |
| `ArcadeAttractMode.tsx:125` | `Animation`, `AnimationDirection::Alternate` | `composed-grid` |
| `ArcadeAttractMode.tsx:169` | keyed `Decoration`, negative `delay_secs` | `composed-particle` |
| `ArcadeFramePulse.tsx:111` | shared `Keyframes`, keyed `Decoration` | `composed-comet` |
| `ArcadeMenuTransition.tsx:88` | `AnimatePresence`, `PresenceMode` | `composed-routes-specimen` |
| `ArcadeMenuTransition.tsx:138` | `Decoration`, `MotionStyle` | `route-beam` |
| `ArcadeMenuTransition.tsx:177` | `clip_inset`, opacity keyframes | `composed-routes-specimen` |
| `ArcadeMenuTransition.tsx:208` | `Decoration`, `Keyframes` | `route-beam` |
| `ArcadeTabTransition.tsx:77` | `LayoutGroup`, `AnimatePresence`, `PresenceMode::PopLayout` | `composed-routes-specimen` |
| `ArcadeTabTransition.tsx:108` | directional `MotionStyle`, keyed `Decoration` | `route-beam` |
| `ArcadeTabTransition.tsx:140` | `Decoration`, `Keyframes` | `composed-routes-specimen` |
| `ArcadeModal.tsx:82` | `AnimatePresence`, `ReducedMotion` | `composed-modal-specimen` |
| `ArcadeModal.tsx:104` | `MotionStyle`, `MotionFilter`, keyframes | `composed-modal-specimen` |
| `ArcadeModal.tsx:168` | `Decoration`, `AnimationIterations::Forever` | `modal-shine` |
| `ArcadeCheckboxEffect.tsx:33` | keyed presence root | `composed-checkbox` |
| `ArcadeCheckboxEffect.tsx:40` | keyed `Decoration`, transform keyframes | `composed-checkbox` |
| `ArcadeCheckboxEffect.tsx:53` | keyed `Decoration`, opacity keyframes | `selection-flash` |
| `ArcadeCheckboxEffect.tsx:69` | keyed `Decoration`, transform and opacity | `composed-checkbox` |
| `ArcadeCheckboxEffect.tsx:87` | `after_all`, keyed particle decorations | `composed-checkbox` |
| `ArcadeButtonEffect.tsx:32` | keyed burst generation | `composed-interactions-specimen` |
| `ArcadeButtonEffect.tsx:46` | keyed `Decoration`, rotation and scale | `composed-interactions-specimen` |
| `ArcadeButtonEffect.tsx:61` | keyed `Decoration`, scale and opacity | `composed-interactions-specimen` |
| `ArcadeButtonEffect.tsx:79` | `after_all`, per-particle `delay_secs` | `composed-interactions-specimen` |
| `ArcadeSliderEffect.tsx:24` | keyed burst generation | `composed-slider` |
| `ArcadeSliderEffect.tsx:38` | `Keyframes`, `Easing::CubicBezier` | `composed-slider` |
| `ArcadeSliderEffect.tsx:52` | keyed particle decorations | `composed-slider` |
| `ArcadeExitSequence.tsx:29` | `Keyframes`, explicit times and easing | `composed-routes-specimen` |
| `ArcadeExitSequence.tsx:42` | `Decoration`, scale and opacity keyframes | `route-beam` |
| `ArcadeExitSequence.tsx:57` | `Decoration`, positioned keyframes | `composed-routes-specimen` |
| `ArcadeExitSequence.tsx:72` | `Decoration`, positioned keyframes | `composed-routes-specimen` |
| `ArcadeExitSequence.tsx:87` | `clip_inset`, filter and opacity keyframes | `composed-routes-specimen` |
| `MainMenu.tsx:101` | `AnimatePresence`, clip and filter keyframes | `composed-routes-specimen` |
| `ScreenFrame.tsx:44` | `AnimatePresence`, clip and filter keyframes | `composed-modal-specimen` |
The complete initial, changed, and restored flows are checked in
`samples/reactant/rules/tests/composition.rs`.

## Reserved React APIs

These names are deliberately unavailable in V1. They are not future sample
features and code must not infer their React contracts from nearby Reactant
APIs.

| Reserved API | V1 status | Reason |
|---|---|---|
| `StrictMode` | Unsupported | Reactant does not perform development-only duplicate rendering or effects. |
| `use_layout_effect` | Unsupported | Unity cannot expose React's synchronous post-mutation, pre-paint timing boundary. |
| `use_sync_external_store` | Unsupported | Reactant has no server snapshot or hydration contract; `use_external_store` names the supported client-only behavior. |
