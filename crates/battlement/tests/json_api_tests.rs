use battlement::{Command, CommandBody, Connect, ScreenSize, json};

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
