use battlement::{
  CameraState, Choice, ClickEvent, DropdownField, F32Range, FocusEvent, GameObject, GameObjectKind,
  GeometryEvent, KeyEvent, LifecycleEvent, LinkEvent, MinMaxSlider, NavigationEvent,
  NavigationMoveEvent, ObjectId, PanelScaleMode, PanelSettings, ParentScene, PointerButtonEvent,
  PointerCancelEvent, PointerCaptureEvent, PointerCrossingEvent, PointerMoveEvent, PreparedAsset,
  Prop, RadioButton, RadioButtonGroup, Scene, SceneId, ScrollEvent, ScrollView, Scroller,
  SelectionEvent, SessionId, Slider, SliderInt, Snapshot, TabCloseEvent, TabReorderEvent,
  TabSelectionEvent, TabView, TextField, TextInputEvent, Toggle, ToggleButtonGroup,
  TransitionEvent, UiDocument, UiDocumentState, UiEvent, UiEventBody, UiEventKind, UiEventPhase,
  UiEventSubscription, UiValue, ValueChangingEvent, ValueCommitEvent, VisualElement,
  VisualElementProperties, WheelEvent,
};
use battlement_reactant::{
  event::{
    ChangeEventRenderExt, EventRenderExt, ReactantEvent, ScrollEventRenderExt, TabEventRenderExt,
    TextEventRenderExt, ValueChangingRenderExt, ValueCommittedRenderExt,
  },
  executor::{BoxFuture, SpawnedTask, Spawner},
  render::Render,
  runtime::Reactant,
};

struct IdleSpawner;

#[derive(Default)]
struct Ledger {
  entries: Vec<String>,
}

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

macro_rules! propagating {
  ($value:expr, $brief:ident, $aware:ident, $capture:ident, $capture_aware:ident, $payload:ty) => {
    $value
      .$brief(|_: &mut Ledger| {})
      .$aware(|_: &mut Ledger, _: ReactantEvent<$payload>| {})
      .$capture(|_: &mut Ledger| {})
      .$capture_aware(|_: &mut Ledger, _: ReactantEvent<$payload>| {})
  };
}

macro_rules! target_only {
  ($value:expr, $brief:ident, $aware:ident, $payload:ty) => {
    $value
      .$brief(|_: &mut Ledger| {})
      .$aware(|_: &mut Ledger, _: ReactantEvent<$payload>| {})
  };
}

#[test]
fn every_common_builder_has_its_typed_form_and_approved_capture_surface() {
  let value = VisualElement::new();
  let value = propagating!(
    value,
    on_pointer_down,
    on_pointer_down_event,
    on_pointer_down_capture,
    on_pointer_down_capture_event,
    PointerButtonEvent
  );
  let value = propagating!(
    value,
    on_pointer_move,
    on_pointer_move_event,
    on_pointer_move_capture,
    on_pointer_move_capture_event,
    PointerMoveEvent
  );
  let value = propagating!(
    value,
    on_pointer_up,
    on_pointer_up_event,
    on_pointer_up_capture,
    on_pointer_up_capture_event,
    PointerButtonEvent
  );
  let value = propagating!(
    value,
    on_pointer_cancel,
    on_pointer_cancel_event,
    on_pointer_cancel_capture,
    on_pointer_cancel_capture_event,
    PointerCancelEvent
  );
  let value = propagating!(
    value,
    on_click,
    on_click_event,
    on_click_capture,
    on_click_capture_event,
    ClickEvent
  );
  let value = propagating!(
    value,
    on_pointer_over,
    on_pointer_over_event,
    on_pointer_over_capture,
    on_pointer_over_capture_event,
    PointerCrossingEvent
  );
  let value = propagating!(
    value,
    on_pointer_out,
    on_pointer_out_event,
    on_pointer_out_capture,
    on_pointer_out_capture_event,
    PointerCrossingEvent
  );
  let value = propagating!(
    value,
    on_wheel,
    on_wheel_event,
    on_wheel_capture,
    on_wheel_capture_event,
    WheelEvent
  );
  let value = propagating!(
    value,
    on_pointer_capture,
    on_pointer_capture_event,
    on_pointer_capture_capture,
    on_pointer_capture_capture_event,
    PointerCaptureEvent
  );
  let value = propagating!(
    value,
    on_pointer_capture_out,
    on_pointer_capture_out_event,
    on_pointer_capture_out_capture,
    on_pointer_capture_out_capture_event,
    PointerCaptureEvent
  );
  let value = propagating!(
    value,
    on_key_down,
    on_key_down_event,
    on_key_down_capture,
    on_key_down_capture_event,
    KeyEvent
  );
  let value = propagating!(
    value,
    on_key_up,
    on_key_up_event,
    on_key_up_capture,
    on_key_up_capture_event,
    KeyEvent
  );
  let value = propagating!(
    value,
    on_navigation_move,
    on_navigation_move_event,
    on_navigation_move_capture,
    on_navigation_move_capture_event,
    NavigationMoveEvent
  );
  let value = propagating!(
    value,
    on_navigation_cancel,
    on_navigation_cancel_event,
    on_navigation_cancel_capture,
    on_navigation_cancel_capture_event,
    NavigationEvent
  );
  let value = propagating!(
    value,
    on_focus_in,
    on_focus_in_event,
    on_focus_in_capture,
    on_focus_in_capture_event,
    FocusEvent
  );
  let value = propagating!(
    value,
    on_focus_out,
    on_focus_out_event,
    on_focus_out_capture,
    on_focus_out_capture_event,
    FocusEvent
  );
  let value = propagating!(
    value,
    on_focus,
    on_focus_event,
    on_focus_capture,
    on_focus_capture_event,
    FocusEvent
  );
  let value = propagating!(
    value,
    on_blur,
    on_blur_event,
    on_blur_capture,
    on_blur_capture_event,
    FocusEvent
  );
  let value = propagating!(
    value,
    on_link_enter,
    on_link_enter_event,
    on_link_enter_capture,
    on_link_enter_capture_event,
    LinkEvent
  );
  let value = propagating!(
    value,
    on_link_leave,
    on_link_leave_event,
    on_link_leave_capture,
    on_link_leave_capture_event,
    LinkEvent
  );
  let value = propagating!(
    value,
    on_link_down,
    on_link_down_event,
    on_link_down_capture,
    on_link_down_capture_event,
    LinkEvent
  );
  let value = propagating!(
    value,
    on_link_up,
    on_link_up_event,
    on_link_up_capture,
    on_link_up_capture_event,
    LinkEvent
  );
  let value = target_only!(
    value,
    on_pointer_enter,
    on_pointer_enter_event,
    PointerCrossingEvent
  );
  let value = target_only!(
    value,
    on_pointer_leave,
    on_pointer_leave_event,
    PointerCrossingEvent
  );
  let value = target_only!(
    value,
    on_geometry_changed,
    on_geometry_changed_event,
    GeometryEvent
  );
  let value = target_only!(
    value,
    on_attach_to_panel,
    on_attach_to_panel_event,
    LifecycleEvent
  );
  let value = target_only!(
    value,
    on_detach_from_panel,
    on_detach_from_panel_event,
    LifecycleEvent
  );
  let value = target_only!(
    value,
    on_transition_start,
    on_transition_start_event,
    TransitionEvent
  );
  let value = target_only!(
    value,
    on_transition_end,
    on_transition_end_event,
    TransitionEvent
  );
  let value = target_only!(
    value,
    on_transition_cancel,
    on_transition_cancel_event,
    TransitionEvent
  );

  let document = self::document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), move |_game: &Ledger| value.clone());
  let snapshot = reactant
    .begin_session(&mut Ledger::default())
    .expect("event catalog renders")
    .into_parts(self::snapshot(&document))
    .0;
  let subscriptions = &snapshot.ui[0].element.event_subscriptions;
  let Prop::Set(subscriptions) = subscriptions else {
    panic!("propagating handlers should install root coverage");
  };
  assert_eq!(subscriptions.len(), 40);
  assert!(subscriptions.contains(&UiEventSubscription::target(UiEventKind::Click)));
  assert!(subscriptions.contains(&UiEventSubscription::new(
    UiEventKind::Click,
    UiEventPhase::Trickle,
  )));
  assert_eq!(
    snapshot.ui[0].children[0]
      .element
      .visual_element()
      .event_subscriptions,
    Prop::Set(vec![
      UiEventSubscription::target(UiEventKind::GeometryChanged),
      UiEventSubscription::target(UiEventKind::AttachToPanel),
      UiEventSubscription::target(UiEventKind::DetachFromPanel),
      UiEventSubscription::target(UiEventKind::TransitionStart),
      UiEventSubscription::target(UiEventKind::TransitionEnd),
      UiEventSubscription::target(UiEventKind::TransitionCancel),
    ])
  );
  let _ = reactant.shutdown(&mut Ledger::default()).into_groups();
}

#[test]
fn control_changes_dispatch_typed_values_and_target_only_subscriptions() {
  let document = self::document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), self::typed_controls);
  let mut ledger = Ledger::default();
  let snapshot = reactant
    .begin_session(&mut ledger)
    .expect("controls render")
    .into_parts(self::snapshot(&document))
    .0;
  let ids = ["text", "slider", "toggle", "dropdown", "tabs"]
    .map(|name| self::find_named(&snapshot.ui[0].children, name));

  self::dispatch(
    &mut reactant,
    &mut ledger,
    UiEvent {
      target_id: ids[0],
      body: UiEventBody::Input(TextInputEvent {
        value: "draft".to_owned(),
      }),
    },
  );
  self::dispatch(
    &mut reactant,
    &mut ledger,
    UiEvent {
      target_id: ids[1],
      body: UiEventBody::ValueChanging(ValueChangingEvent {
        proposed: UiValue::F32(2.5),
      }),
    },
  );
  self::dispatch(
    &mut reactant,
    &mut ledger,
    UiEvent {
      target_id: ids[2],
      body: UiEventBody::ValueCommitted(ValueCommitEvent {
        previous: UiValue::Bool(false),
        proposed: UiValue::Bool(true),
      }),
    },
  );
  self::dispatch(
    &mut reactant,
    &mut ledger,
    UiEvent {
      target_id: ids[3],
      body: UiEventBody::ValueCommitted(ValueCommitEvent {
        previous: UiValue::Choice(Choice::none()),
        proposed: UiValue::Choice(Choice::selected(1, "B")),
      }),
    },
  );
  self::dispatch(
    &mut reactant,
    &mut ledger,
    UiEvent {
      target_id: ids[4],
      body: UiEventBody::TabSelectionRequested(TabSelectionEvent {
        previous_index: 0,
        proposed_index: 1,
        proposed_tab_id: ObjectId::new_v4(),
      }),
    },
  );

  assert_eq!(
    ledger.entries,
    [
      "text:draft",
      "slider:2.5",
      "toggle:true",
      "dropdown:B",
      "tabs:1"
    ]
  );
  for object_id in ids {
    let node = self::find_node(&snapshot.ui[0].children, object_id);
    let Prop::Set(subscriptions) = &node.element.visual_element().event_subscriptions else {
      panic!("change handlers should subscribe on their target controls");
    };
    assert_eq!(subscriptions.len(), 1);
    assert_eq!(subscriptions[0].phase, UiEventPhase::Target);
  }
  let _ = reactant.shutdown(&mut ledger).into_groups();
}

#[test]
fn control_specific_builder_catalog_preserves_exact_host_types() {
  let text = TextField::new();
  let text = target_only!(text, on_input, on_input_event, TextInputEvent);
  let text = target_only!(
    text,
    on_selection_changed,
    on_selection_changed_event,
    SelectionEvent
  );
  let text = target_only!(
    text,
    on_value_committed,
    on_value_committed_event,
    ValueCommitEvent
  );
  let text = text
    .on_change(|_: &mut Ledger| {})
    .on_change_event(|_: &mut Ledger, _: ReactantEvent<String>| {});

  let scroll = ScrollView::new();
  let scroll = target_only!(
    scroll,
    on_scroll_settled,
    on_scroll_settled_event,
    ScrollEvent
  );
  let scroll = target_only!(
    scroll,
    on_scroll_changed,
    on_scroll_changed_event,
    ScrollEvent
  );

  let tabs = TabView::new();
  let tabs = target_only!(
    tabs,
    on_tab_selection_requested,
    on_tab_selection_requested_event,
    TabSelectionEvent
  );
  let tabs = target_only!(
    tabs,
    on_tab_close_requested,
    on_tab_close_requested_event,
    TabCloseEvent
  );
  let tabs = target_only!(
    tabs,
    on_tab_reorder_requested,
    on_tab_reorder_requested_event,
    TabReorderEvent
  );
  let tabs = tabs
    .on_change(|_: &mut Ledger| {})
    .on_change_event(|_: &mut Ledger, _: ReactantEvent<u32>| {});

  let scroller = Scroller::new()
    .on_value_changing(|_: &mut Ledger| {})
    .on_value_changing_event(|_: &mut Ledger, _: ReactantEvent<ValueChangingEvent>| {})
    .on_value_committed(|_: &mut Ledger| {})
    .on_value_committed_event(|_: &mut Ledger, _: ReactantEvent<ValueCommitEvent>| {})
    .on_change_event(|_: &mut Ledger, _: ReactantEvent<f32>| {});
  let slider = Slider::new()
    .on_value_changing(|_: &mut Ledger| {})
    .on_value_changing_event(|_: &mut Ledger, _: ReactantEvent<ValueChangingEvent>| {})
    .on_value_committed(|_: &mut Ledger| {})
    .on_value_committed_event(|_: &mut Ledger, _: ReactantEvent<ValueCommitEvent>| {})
    .on_change_event(|_: &mut Ledger, _: ReactantEvent<f32>| {});
  let slider_int = SliderInt::new()
    .on_value_changing(|_: &mut Ledger| {})
    .on_value_changing_event(|_: &mut Ledger, _: ReactantEvent<ValueChangingEvent>| {})
    .on_value_committed(|_: &mut Ledger| {})
    .on_value_committed_event(|_: &mut Ledger, _: ReactantEvent<ValueCommitEvent>| {})
    .on_change_event(|_: &mut Ledger, _: ReactantEvent<i32>| {});
  let min_max = MinMaxSlider::new()
    .on_value_changing(|_: &mut Ledger| {})
    .on_value_changing_event(|_: &mut Ledger, _: ReactantEvent<ValueChangingEvent>| {})
    .on_value_committed(|_: &mut Ledger| {})
    .on_value_committed_event(|_: &mut Ledger, _: ReactantEvent<ValueCommitEvent>| {})
    .on_change_event(|_: &mut Ledger, _: ReactantEvent<F32Range>| {});
  let toggle = Toggle::new()
    .on_value_committed(|_: &mut Ledger| {})
    .on_value_committed_event(|_: &mut Ledger, _: ReactantEvent<ValueCommitEvent>| {})
    .on_change_event(|_: &mut Ledger, _: ReactantEvent<bool>| {});
  let radio = RadioButton::new()
    .on_value_committed(|_: &mut Ledger| {})
    .on_value_committed_event(|_: &mut Ledger, _: ReactantEvent<ValueCommitEvent>| {})
    .on_change_event(|_: &mut Ledger, _: ReactantEvent<bool>| {});
  let radio_group = RadioButtonGroup::new()
    .on_value_committed(|_: &mut Ledger| {})
    .on_value_committed_event(|_: &mut Ledger, _: ReactantEvent<ValueCommitEvent>| {})
    .on_change_event(|_: &mut Ledger, _: ReactantEvent<Option<u32>>| {});
  let toggle_group = ToggleButtonGroup::new()
    .on_value_committed(|_: &mut Ledger| {})
    .on_value_committed_event(|_: &mut Ledger, _: ReactantEvent<ValueCommitEvent>| {})
    .on_change_event(|_: &mut Ledger, _: ReactantEvent<Vec<u32>>| {});
  let dropdown = DropdownField::new()
    .on_value_committed(|_: &mut Ledger| {})
    .on_value_committed_event(|_: &mut Ledger, _: ReactantEvent<ValueCommitEvent>| {})
    .on_change_event(|_: &mut Ledger, _: ReactantEvent<Choice>| {});

  let _catalog = (
    text,
    scroll,
    tabs,
    scroller,
    slider,
    slider_int,
    min_max,
    toggle,
    radio,
    radio_group,
    toggle_group,
    dropdown,
  );
}

fn typed_controls(ledger: &Ledger) -> impl Render + use<> {
  let _ = ledger;
  (
    TextField::new().name("text").on_change_event(
      |game: &mut Ledger, event: ReactantEvent<String>| {
        game.entries.push(format!("text:{}", event.payload()))
      },
    ),
    Slider::new()
      .name("slider")
      .on_change_event(|game: &mut Ledger, event: ReactantEvent<f32>| {
        game.entries.push(format!("slider:{}", event.payload()))
      }),
    Toggle::new().name("toggle").on_change_event(
      |game: &mut Ledger, event: ReactantEvent<bool>| {
        game.entries.push(format!("toggle:{}", event.payload()))
      },
    ),
    DropdownField::new().name("dropdown").on_change_event(
      |game: &mut Ledger, event: ReactantEvent<Choice>| {
        game.entries.push(format!(
          "dropdown:{}",
          event.payload().value.as_deref().unwrap_or_default()
        ))
      },
    ),
    TabView::new()
      .name("tabs")
      .on_change_event(|game: &mut Ledger, event: ReactantEvent<u32>| {
        game.entries.push(format!("tabs:{}", event.payload()))
      }),
  )
}

fn dispatch(reactant: &mut Reactant<Ledger>, ledger: &mut Ledger, event: UiEvent) {
  let commit = reactant
    .dispatch(ledger, event)
    .expect("typed event dispatch succeeds");
  assert!(commit.is_empty());
  assert!(commit.into_groups().is_empty());
}

fn find_named(nodes: &[battlement::UiNode], name: &str) -> ObjectId {
  nodes
    .iter()
    .find_map(|node| {
      (node.element.visual_element().name == Prop::Set(name.to_owned()))
        .then_some(node.object_id)
        .or_else(|| self::find_named_optional(&node.children, name))
    })
    .unwrap_or_else(|| panic!("missing node named {name}"))
}

fn find_named_optional(nodes: &[battlement::UiNode], name: &str) -> Option<ObjectId> {
  nodes.iter().find_map(|node| {
    (node.element.visual_element().name == Prop::Set(name.to_owned()))
      .then_some(node.object_id)
      .or_else(|| self::find_named_optional(&node.children, name))
  })
}

fn find_node(nodes: &[battlement::UiNode], object_id: ObjectId) -> &battlement::UiNode {
  nodes
    .iter()
    .find_map(|node| {
      (node.object_id == object_id)
        .then_some(node)
        .or_else(|| self::find_node_optional(&node.children, object_id))
    })
    .unwrap_or_else(|| panic!("missing node {object_id}"))
}

fn find_node_optional(
  nodes: &[battlement::UiNode],
  object_id: ObjectId,
) -> Option<&battlement::UiNode> {
  nodes.iter().find_map(|node| {
    (node.object_id == object_id)
      .then_some(node)
      .or_else(|| self::find_node_optional(&node.children, object_id))
  })
}

fn document() -> UiDocument {
  UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4())
}

fn snapshot(document: &UiDocument) -> Snapshot {
  let camera_id = ObjectId::new_v4();
  Snapshot::new(
    SessionId::new_v4(),
    vec![PreparedAsset::Scene("test/scene".into())],
    vec![Scene::new(SceneId::new_v4(), "test/scene")],
    vec![
      GameObject::new(camera_id, CameraState::new()),
      GameObject::new(
        document.document_id,
        GameObjectKind::UiDocument(
          UiDocumentState::new(document.root_id)
            .panel_settings(PanelSettings::new().scale_mode(PanelScaleMode::ConstantPixelSize)),
        ),
      )
      .parent_scene(ParentScene::Persistent),
    ],
    camera_id,
  )
}
