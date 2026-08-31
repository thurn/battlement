use battlement::{
  ActionBody, AudioPlayPayload, AudioStopPayload, Command, CommandBody, CommandId, Connect,
  ControllerDirection, ControllerInputSettings, ControllerNavigationPayload,
  ControllerNavigationSource, ScreenSize, UiButton, json, object_id,
};
use serde_json::json as value;

#[test]
fn encodes_minified_natural_json() {
  let connect = Connect::new(
    "Linux",
    "6000.5.8f1",
    ScreenSize {
      width: 1920,
      height: 1080,
    },
  );

  assert_eq!(
        json::to_vec(&connect).unwrap(),
        br#"{"platform":"Linux","unity_version":"6000.5.8f1","screen":{"width":1920,"height":1080},"custom_command_types":[]}"#
    );
  assert_eq!(
    json::from_slice::<Connect>(&json::to_vec(&connect).unwrap()).unwrap(),
    connect
  );
}

#[test]
fn controller_navigation_uses_a_platform_neutral_tag_and_omits_initial_repeat() {
  let body = ActionBody::ControllerNavigate(ControllerNavigationPayload {
    controller_id: 7,
    direction: ControllerDirection::Left,
    source: ControllerNavigationSource::LeftStick,
    repeat: false,
  });

  assert_eq!(
    json::to_vec(&body).unwrap(),
    br#"{"ControllerNavigate":{"controller_id":7,"direction":"Left","source":"LeftStick"}}"#
  );
  assert_eq!(
    json::from_slice::<ActionBody>(&json::to_vec(&body).unwrap()).unwrap(),
    body
  );
}

#[test]
fn debug_ui_uses_one_surface_selected_command_body() {
  let body = CommandBody::DebugUi(battlement::DebugUiPayload {
    surface: battlement::DebugUiSurface::FpsViewer,
    visible: true,
  });

  assert_eq!(
    json::to_vec(&body).unwrap(),
    br#"{"DebugUi":{"surface":"FpsViewer","visible":true}}"#
  );
  assert_eq!(
    json::from_slice::<CommandBody>(&json::to_vec(&body).unwrap()).unwrap(),
    body
  );
}

#[test]
fn controller_settings_omit_client_native_navigation_overrides() {
  assert_eq!(
    json::to_vec(&CommandBody::InputSetController(
      ControllerInputSettings::new()
    ))
    .unwrap(),
    br#"{"InputSetController":{"buttons":[]}}"#
  );

  let mut disabled = ControllerInputSettings::new();
  disabled.navigation_enabled = false;
  assert_eq!(
    json::to_vec(&CommandBody::InputSetController(disabled)).unwrap(),
    br#"{"InputSetController":{"buttons":[],"navigation_enabled":false}}"#
  );

  let decoded: CommandBody = json::from_slice(br#"{"InputSetController":{"buttons":[]}}"#).unwrap();
  assert!(matches!(
      decoded,
      CommandBody::InputSetController(settings) if settings.navigation_enabled
  ));
}

#[test]
fn omits_disabled_input_default() {
  assert_eq!(
    json::to_vec(&CommandBody::set_input_enabled(false)).unwrap(),
    br#"{"InputSetEnabled":{}}"#
  );
}

#[test]
fn omits_zero_audio_fade_default() {
  assert_eq!(
    json::to_vec(&CommandBody::AudioPlay(AudioPlayPayload {
      address: "test/sound".into(),
      volume: 1.0,
      pitch: 1.0,
      r#loop: false,
      fade_in_ms: 0,
    }))
    .unwrap(),
    br#"{"AudioPlay":{"address":"test/sound"}}"#
  );

  assert_eq!(
    json::to_vec(&CommandBody::AudioStop(AudioStopPayload {
      audio_command_id: "00112233-4455-6677-8899-aabbccddeeff"
        .parse::<CommandId>()
        .unwrap(),
      fade_out_ms: 0,
    }))
    .unwrap(),
    br#"{"AudioStop":{"audio_command_id":"00112233-4455-6677-8899-aabbccddeeff"}}"#
  );
}

#[test]
fn restores_omitted_protocol_defaults() {
  let command = json::from_slice::<Command>(
        br#"{"command_id":"00112233-4455-6677-8899-aabbccddeeff","body":{"ParticlePlay":{"object_id":"11223344-5566-7788-99aa-bbccddeeff00"}}}"#,
    )
    .unwrap();

  assert!(command.blocking);
  assert!(matches!(
      command.body,
      CommandBody::ParticlePlay(payload) if !payload.restart
  ));

  let reparent = json::from_slice::<Command>(
        br#"{"command_id":"00112233-4455-6677-8899-aabbccddeeff","body":{"ObjectReparent":{"object_id":"11223344-5566-7788-99aa-bbccddeeff00","parent_id":null}}}"#,
    )
    .unwrap();
  assert!(matches!(
      reparent.body,
      CommandBody::ObjectReparent(payload) if !payload.world_position_stays
  ));

  let animator = json::from_slice::<Command>(
        br#"{"command_id":"00112233-4455-6677-8899-aabbccddeeff","body":{"AnimatorSetInt":{"object_id":"11223344-5566-7788-99aa-bbccddeeff00","parameter":"score"}}}"#,
    )
    .unwrap();
  assert!(matches!(
      animator.body,
      CommandBody::AnimatorSetInt(payload) if payload.value == 0
  ));
}

#[test]
fn accepts_whitespace_but_rejects_trailing_values() {
  let connect = Connect::new("Linux", "6000.5.8f1", ScreenSize::new(1, 2));
  let bytes = json::to_vec(&connect).unwrap();

  let mut whitespace = bytes.clone();
  whitespace.extend_from_slice(b" \n\t");
  assert_eq!(json::from_slice::<Connect>(&whitespace).unwrap(), connect);

  let mut trailing = bytes;
  trailing.extend_from_slice(b"null");
  assert!(json::from_slice::<Connect>(&trailing).is_err());
}

#[test]
fn rejects_truncated_invalid_and_excessively_nested_json() {
  assert!(json::from_slice::<Connect>(br#"{"platform":"Linux""#).is_err());
  assert!(json::from_slice::<Connect>(b"not-json").is_err());

  let nested = format!("{}null{}", "[".repeat(129), "]".repeat(129));
  assert!(json::from_slice::<Connect>(nested.as_bytes()).is_err());
}

#[test]
fn ui_commands_keep_minimal_tags_and_sparse_update_values() {
  let parent_id = object_id!("1fd199f0-1a61-4e86-8ad3-c05d6e29d8f8");
  let button_id = object_id!("c1ef647b-2729-4675-a0d5-bafe5916bd36");
  let create = Command::create_visual_element(
    parent_id,
    battlement::UiNode::new(button_id, UiButton::new("Continue")),
  );
  assert_eq!(
    serde_json::to_value(create.body).unwrap(),
    value!({
        "VisualElementCreate": {
            "parent_id": parent_id,
            "node": {
                "object_id": button_id,
                "element": {
                    "Button": { "text": "Continue" }
                }
            }
        }
    })
  );

  let update = Command::update_visual_element(button_id, UiButton::new("").enabled(false));
  assert_eq!(
    serde_json::to_value(update.body).unwrap(),
    value!({
        "VisualElementUpdate": {
            "Properties": {
                "object_id": button_id,
                "element": {
                    "Button": {
                        "enabled": false,
                        "text": ""
                    }
                }
            }
        }
    })
  );
}
