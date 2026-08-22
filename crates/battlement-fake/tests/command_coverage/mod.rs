use super::*;

fn full_snapshot(session_id: battlement::SessionId) -> Snapshot {
    let mut value = snapshot(
        session_id,
        vec![
            camera(),
            GameObject::new(object_id(2), GameObjectKind::Empty),
            GameObject::new(
                object_id(3),
                GameObjectKind::Cube {
                    materials: vec![battlement::MaterialAssignment::new(0, "test/material")],
                },
            ),
            GameObject::new(
                object_id(4),
                GameObjectKind::Image {
                    image: battlement::ImageState::new("test/texture", 1.0, 2.0),
                },
            ),
            GameObject::new(
                object_id(5),
                GameObjectKind::Text {
                    text: battlement::TextState::new("before", "test/font"),
                },
            ),
            GameObject::new(
                object_id(6),
                GameObjectKind::Light {
                    light: battlement::LightState::default(),
                },
            ),
            GameObject::new(
                object_id(7),
                GameObjectKind::Prefab {
                    address: "test/prefab".into(),
                    materials: vec![battlement::MaterialAssignment::new(0, "test/material")],
                    animator: Some(battlement::AnimatorState::new("Idle")),
                },
            ),
        ],
    );
    value.prepared_assets = vec![
        PreparedAsset::Scene("test/scene".into()),
        PreparedAsset::Scene("test/scene2".into()),
        PreparedAsset::Material("test/material".into()),
        PreparedAsset::Texture("test/texture".into()),
        PreparedAsset::Font("test/font".into()),
        PreparedAsset::Prefab("test/prefab".into()),
        PreparedAsset::ParticleEffect("test/particles".into()),
        PreparedAsset::AudioClip("test/audio".into()),
    ];
    value
}

fn push_body(
    commands: &mut Vec<Command>,
    next: &mut u128,
    body: CommandBody,
) -> battlement::CommandId {
    let id = command_id(*next);
    *next += 1;
    commands.push(Command::new(id, body));
    id
}

fn push_nonblocking(
    commands: &mut Vec<Command>,
    next: &mut u128,
    body: CommandBody,
) -> battlement::CommandId {
    let id = command_id(*next);
    *next += 1;
    commands.push(Command::new(id, body).nonblocking());
    id
}

#[test]
fn every_current_command_family_has_a_public_path_and_observable_result() {
    let session_id = session(8);
    let mut commands = Vec::new();
    let mut next = 100;
    push_body(
        &mut commands,
        &mut next,
        CommandBody::AssetsReplaceSet(battlement::ReplaceAssetSetPayload {
            assets: full_snapshot(session_id).prepared_assets,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::SceneLoad(battlement::SceneLoadPayload {
            scene_id: scene_id(11),
            address: "test/scene2".into(),
            make_primary: true,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ObjectCreate(Box::new(battlement::ObjectCreatePayload {
            object: GameObject::new(object_id(8), GameObjectKind::Empty),
        })),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::SceneSetPrimary(battlement::SceneIdPayload {
            scene_id: scene_id(10),
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ObjectCreate(Box::new(battlement::ObjectCreatePayload {
            object: GameObject::new(object_id(9), GameObjectKind::Empty),
        })),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ObjectSetActive(battlement::ObjectSetActivePayload {
            object_id: object_id(2),
            active: false,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ObjectSetActive(battlement::ObjectSetActivePayload {
            object_id: object_id(2),
            active: true,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ObjectReparent(battlement::ObjectReparentPayload {
            object_id: object_id(3),
            parent_id: Some(object_id(2)),
            world_position_stays: true,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TransformSetLocalPosition(battlement::PropertyCommand::canceling(
            battlement::PositionPayload {
                object_id: object_id(3),
                position: Vector3::new(1.0, 2.0, 3.0),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TransformSetWorldPosition(battlement::PropertyCommand::canceling(
            battlement::PositionPayload {
                object_id: object_id(3),
                position: Vector3::new(4.0, 5.0, 6.0),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TransformTweenLocalPosition(battlement::PropertyCommand::canceling(
            battlement::TweenPositionPayload {
                object_id: object_id(3),
                position: Vector3::new(7.0, 8.0, 9.0),
                tween: battlement::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TransformTweenWorldPosition(battlement::PropertyCommand::canceling(
            battlement::TweenPositionPayload {
                object_id: object_id(3),
                position: Vector3::new(10.0, 11.0, 12.0),
                tween: battlement::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TransformSetLocalRotation(battlement::PropertyCommand::canceling(
            battlement::RotationPayload {
                object_id: object_id(3),
                rotation: battlement::Quaternion::IDENTITY,
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TransformSetWorldRotation(battlement::PropertyCommand::canceling(
            battlement::RotationPayload {
                object_id: object_id(3),
                rotation: battlement::Quaternion::IDENTITY,
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TransformTweenLocalRotation(battlement::PropertyCommand::canceling(
            battlement::TweenRotationPayload {
                object_id: object_id(3),
                rotation: battlement::Quaternion::IDENTITY,
                tween: battlement::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TransformTweenWorldRotation(battlement::PropertyCommand::canceling(
            battlement::TweenRotationPayload {
                object_id: object_id(3),
                rotation: battlement::Quaternion::IDENTITY,
                tween: battlement::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TransformSetLocalScale(battlement::PropertyCommand::canceling(
            battlement::ScalePayload {
                object_id: object_id(3),
                scale: Vector3::new(2.0, 2.0, 2.0),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TransformTweenLocalScale(battlement::PropertyCommand::canceling(
            battlement::TweenScalePayload {
                object_id: object_id(3),
                scale: Vector3::ONE,
                tween: battlement::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::RendererSetMaterial(battlement::PropertyCommand::canceling(
            battlement::SetMaterialPayload {
                object_id: object_id(3),
                address: "test/material".into(),
                slot: None,
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::CameraSetEnabled(battlement::ObjectEnabledPayload {
            object_id: object_id(1),
            enabled: false,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::CameraSetEnabled(battlement::ObjectEnabledPayload {
            object_id: object_id(1),
            enabled: true,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::CameraSetPerspective(battlement::PropertyCommand::canceling(
            battlement::PerspectivePayload {
                object_id: object_id(1),
                field_of_view: 70.0,
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::CameraTweenFieldOfView(battlement::PropertyCommand::canceling(
            battlement::TweenFieldOfViewPayload {
                object_id: object_id(1),
                field_of_view: 72.0,
                tween: battlement::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::CameraSetOrthographic(battlement::PropertyCommand::canceling(
            battlement::OrthographicPayload {
                object_id: object_id(1),
                size: 4.0,
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::CameraTweenOrthographicSize(battlement::PropertyCommand::canceling(
            battlement::TweenOrthographicSizePayload {
                object_id: object_id(1),
                size: 3.0,
                tween: battlement::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::CameraSetClipping(battlement::CameraClippingPayload {
            object_id: object_id(1),
            near: 0.2,
            far: 500.0,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::CameraSetClear(battlement::CameraClearPayload {
            object_id: object_id(1),
            clear_mode: battlement::CameraClearMode::SolidColor,
            clear_color: Some(battlement::Color::BLACK),
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::LightSetEnabled(battlement::ObjectEnabledPayload {
            object_id: object_id(6),
            enabled: false,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::LightSetType(battlement::LightTypePayload {
            object_id: object_id(6),
            light_type: battlement::LightType::Spot,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::LightSetColor(battlement::PropertyCommand::canceling(
            battlement::ColorPayload {
                object_id: object_id(6),
                color: battlement::Color::BLACK,
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::LightTweenColor(battlement::PropertyCommand::canceling(
            battlement::TweenColorPayload {
                object_id: object_id(6),
                color: battlement::Color::WHITE,
                tween: battlement::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::LightSetIntensity(battlement::PropertyCommand::canceling(
            battlement::IntensityPayload {
                object_id: object_id(6),
                intensity: 2.0,
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::LightTweenIntensity(battlement::PropertyCommand::canceling(
            battlement::TweenIntensityPayload {
                object_id: object_id(6),
                intensity: 3.0,
                tween: battlement::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::LightSetRange(battlement::LightRangePayload {
            object_id: object_id(6),
            range: 20.0,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::LightSetSpotAngle(battlement::SpotAnglePayload {
            object_id: object_id(6),
            inner_spot_angle: 5.0,
            outer_spot_angle: 40.0,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::LightSetShadows(battlement::LightShadowsPayload {
            object_id: object_id(6),
            shadows: battlement::ShadowMode::Soft,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ImageSetTexture(battlement::SetTexturePayload {
            object_id: object_id(4),
            address: "test/texture".into(),
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ImageSetSize(battlement::ImageSizePayload {
            object_id: object_id(4),
            width: 2.0,
            height: 3.0,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ImageSetFit(battlement::ImageFitPayload {
            object_id: object_id(4),
            fit: battlement::ImageFit::Cover,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ImageSetTint(battlement::PropertyCommand::canceling(
            battlement::TintPayload {
                object_id: object_id(4),
                tint: battlement::RgbColor::BLACK,
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ImageTweenTint(battlement::PropertyCommand::canceling(
            battlement::TweenTintPayload {
                object_id: object_id(4),
                tint: battlement::RgbColor::WHITE,
                tween: battlement::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ImageSetOpacity(battlement::PropertyCommand::canceling(
            battlement::OpacityPayload {
                object_id: object_id(4),
                opacity: 0.5,
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ImageTweenOpacity(battlement::PropertyCommand::canceling(
            battlement::TweenOpacityPayload {
                object_id: object_id(4),
                opacity: 0.75,
                tween: battlement::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ImageSetFaceCamera(battlement::ObjectEnabledPayload {
            object_id: object_id(4),
            enabled: true,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TextSetContent(battlement::TextContentPayload {
            object_id: object_id(5),
            text: "after".to_owned(),
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TextSetFont(battlement::SetFontPayload {
            object_id: object_id(5),
            address: "test/font".into(),
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TextSetSize(battlement::PropertyCommand::canceling(
            battlement::TextSizePayload {
                object_id: object_id(5),
                size: 2.0,
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TextTweenSize(battlement::PropertyCommand::canceling(
            battlement::TweenTextSizePayload {
                object_id: object_id(5),
                size: 3.0,
                tween: battlement::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TextSetColor(battlement::PropertyCommand::canceling(
            battlement::ColorPayload {
                object_id: object_id(5),
                color: battlement::Color::BLACK,
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TextTweenColor(battlement::PropertyCommand::canceling(
            battlement::TweenColorPayload {
                object_id: object_id(5),
                color: battlement::Color::WHITE,
                tween: battlement::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TextSetAlignment(battlement::TextAlignmentPayload {
            object_id: object_id(5),
            horizontal: battlement::HorizontalAlignment::Left,
            vertical: battlement::VerticalAlignment::Top,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TextSetWrapping(battlement::TextWrappingPayload {
            object_id: object_id(5),
            wrap_width: Some(4.0),
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TextSetRichText(battlement::ObjectEnabledPayload {
            object_id: object_id(5),
            enabled: true,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TextSetFaceCamera(battlement::ObjectEnabledPayload {
            object_id: object_id(5),
            enabled: true,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::AnimatorPlay(battlement::AnimatorPlayPayload {
            object_id: object_id(7),
            state: "Walk".to_owned(),
            layer: 0,
            normalized_start_time: 0.25,
            wait_ms: 0,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::AnimatorCrossFade(battlement::AnimatorCrossFadePayload {
            object_id: object_id(7),
            state: "Idle".to_owned(),
            layer: 0,
            normalized_start_time: 0.0,
            wait_ms: 0,
            cross_fade_ms: 1,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::AnimatorSetBool(battlement::AnimatorBoolPayload {
            object_id: object_id(7),
            parameter: "running".to_owned(),
            value: true,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::AnimatorSetInt(battlement::AnimatorIntPayload {
            object_id: object_id(7),
            parameter: "count".to_owned(),
            value: 3,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::AnimatorSetFloat(battlement::AnimatorFloatPayload {
            object_id: object_id(7),
            parameter: "blend".to_owned(),
            value: 0.5,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::AnimatorSetTrigger(battlement::AnimatorParameterPayload {
            object_id: object_id(7),
            parameter: "fire".to_owned(),
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::AnimatorSetSpeed(battlement::AnimatorSpeedPayload {
            object_id: object_id(7),
            speed: 2.0,
        }),
    );
    push_nonblocking(
        &mut commands,
        &mut next,
        CommandBody::ParticlePlay(battlement::ParticlePlayPayload {
            object_id: object_id(7),
            restart: true,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ParticleStop(battlement::ParticleStopPayload {
            object_id: object_id(7),
            clear: true,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ParticleSpawn(battlement::ParticleSpawnPayload {
            address: "test/particles".into(),
            location: battlement::ParticleSpawnLocation::GameObject(object_id(3)),
            lifetime_ms: 1,
        }),
    );
    let audio_command_id = push_body(
        &mut commands,
        &mut next,
        CommandBody::AudioPlay(battlement::AudioPlayPayload {
            address: "test/audio".into(),
            volume: 0.5,
            pitch: 1.0,
            r#loop: false,
            fade_in_ms: 0,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::AudioSetVolume(battlement::PropertyCommand::canceling(
            battlement::AudioVolumePayload {
                audio_command_id,
                volume: 0.75,
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::AudioTweenVolume(battlement::PropertyCommand::canceling(
            battlement::TweenAudioVolumePayload {
                audio_command_id,
                volume: 1.0,
                tween: battlement::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TimeWait(battlement::WaitPayload { duration_ms: 1 }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::OperationCancel(battlement::CancelOperationPayload {
            command_id: audio_command_id,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::AudioStop(battlement::AudioStopPayload {
            audio_command_id,
            fade_out_ms: 0,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::InputSetEnabled(battlement::SetInputEnabledPayload { enabled: false }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::InputSetEnabled(battlement::SetInputEnabledPayload { enabled: true }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::InputSetCamera(battlement::ObjectIdPayload {
            object_id: object_id(1),
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::InputSetPointerEvents(battlement::PointerEventsPayload {
            object_id: object_id(3),
            events: vec![PointerEvent::Click, PointerEvent::Click],
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::InputSetGlobalKeys(battlement::GlobalKeysPayload {
            keys: vec![battlement::KeyCode::Space, battlement::KeyCode::Space],
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ObjectDestroy(battlement::ObjectIdPayload {
            object_id: object_id(8),
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ObjectDestroy(battlement::ObjectIdPayload {
            object_id: object_id(9),
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::SceneUnload(battlement::SceneIdPayload {
            scene_id: scene_id(11),
        }),
    );

    let expected_count = commands.len();
    let response = Response::new(
        session_id,
        vec![ResponseMessage::Batch(Batch::new(
            batch_id(9_000),
            session_id,
            vec![ParallelCommandGroup::new(commands)],
        ))],
    );
    let engine = ScriptedEngine::new(
        [Response::new(
            session_id,
            vec![ResponseMessage::Snapshot(full_snapshot(session_id))],
        )],
        [],
        [Some(response)],
    );
    let mut client = FakeClient::connect(engine, catalog());
    client.poll();

    assert_eq!(client.commands().len(), expected_count);
    client.assert_object_absent(object_id(8));
    client.assert_object_absent(object_id(9));
    assert!(client.world().scene(scene_id(11)).is_none());
    assert_eq!(
        client
            .assert_object(object_id(3))
            .material(0)
            .unwrap()
            .as_str(),
        "test/material"
    );
    assert_eq!(client.assert_object(object_id(4)).particles_playing(), None);
    assert_eq!(
        client.assert_object(object_id(7)).particles_playing(),
        Some(false)
    );
    assert!(client.world().audio(audio_command_id).is_none());
    assert_eq!(client.world().global_keys(), &[battlement::KeyCode::Space]);
    assert_eq!(
        client.assert_object(object_id(5)).kind(),
        &GameObjectKind::Text {
            text: battlement::TextState {
                text: "after".to_owned(),
                font: "test/font".into(),
                size: 3.0,
                color: battlement::Color::WHITE,
                horizontal: battlement::HorizontalAlignment::Left,
                vertical: battlement::VerticalAlignment::Top,
                wrap_width: Some(4.0),
                rich_text: true,
                face_camera: true,
            },
        }
    );
    client.assert_command("particle spawn", |value| {
        matches!(value.body, CommandBody::ParticleSpawn(_))
    });
    client.assert_command("operation cancel", |value| {
        matches!(value.body, CommandBody::OperationCancel(_))
    });
}
