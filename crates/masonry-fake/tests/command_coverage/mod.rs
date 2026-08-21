use super::*;

fn full_snapshot(session_id: masonry::SessionId) -> Snapshot {
    let mut value = snapshot(
        session_id,
        vec![
            camera(),
            GameObject::new(object_id(2), GameObjectKind::Empty),
            GameObject::new(
                object_id(3),
                GameObjectKind::Cube {
                    materials: vec![masonry::MaterialAssignment::new(0, "test/material")],
                },
            ),
            GameObject::new(
                object_id(4),
                GameObjectKind::Image {
                    image: masonry::ImageState::new("test/texture", 1.0, 2.0),
                },
            ),
            GameObject::new(
                object_id(5),
                GameObjectKind::Text {
                    text: masonry::TextState::new("before", "test/font"),
                },
            ),
            GameObject::new(
                object_id(6),
                GameObjectKind::Light {
                    light: masonry::LightState::default(),
                },
            ),
            GameObject::new(
                object_id(7),
                GameObjectKind::Prefab {
                    address: "test/prefab".into(),
                    materials: vec![masonry::MaterialAssignment::new(0, "test/material")],
                    animator: Some(masonry::AnimatorState::new("Idle")),
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
) -> masonry::CommandId {
    let id = command_id(*next);
    *next += 1;
    commands.push(Command::new(id, body));
    id
}

fn push_nonblocking(
    commands: &mut Vec<Command>,
    next: &mut u128,
    body: CommandBody,
) -> masonry::CommandId {
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
        CommandBody::AssetsReplaceSet(masonry::ReplaceAssetSetPayload {
            assets: full_snapshot(session_id).prepared_assets,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::SceneLoad(masonry::SceneLoadPayload {
            scene_id: scene_id(11),
            address: "test/scene2".into(),
            make_primary: true,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ObjectCreate(Box::new(masonry::ObjectCreatePayload {
            object: GameObject::new(object_id(8), GameObjectKind::Empty),
        })),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::SceneSetPrimary(masonry::SceneIdPayload {
            scene_id: scene_id(10),
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ObjectCreate(Box::new(masonry::ObjectCreatePayload {
            object: GameObject::new(object_id(9), GameObjectKind::Empty),
        })),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ObjectSetActive(masonry::ObjectSetActivePayload {
            object_id: object_id(2),
            active: false,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ObjectSetActive(masonry::ObjectSetActivePayload {
            object_id: object_id(2),
            active: true,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ObjectReparent(masonry::ObjectReparentPayload {
            object_id: object_id(3),
            parent_id: Some(object_id(2)),
            world_position_stays: true,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TransformSetLocalPosition(masonry::PropertyCommand::canceling(
            masonry::PositionPayload {
                object_id: object_id(3),
                position: Vector3::new(1.0, 2.0, 3.0),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TransformSetWorldPosition(masonry::PropertyCommand::canceling(
            masonry::PositionPayload {
                object_id: object_id(3),
                position: Vector3::new(4.0, 5.0, 6.0),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TransformTweenLocalPosition(masonry::PropertyCommand::canceling(
            masonry::TweenPositionPayload {
                object_id: object_id(3),
                position: Vector3::new(7.0, 8.0, 9.0),
                tween: masonry::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TransformTweenWorldPosition(masonry::PropertyCommand::canceling(
            masonry::TweenPositionPayload {
                object_id: object_id(3),
                position: Vector3::new(10.0, 11.0, 12.0),
                tween: masonry::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TransformSetLocalRotation(masonry::PropertyCommand::canceling(
            masonry::RotationPayload {
                object_id: object_id(3),
                rotation: masonry::Quaternion::IDENTITY,
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TransformSetWorldRotation(masonry::PropertyCommand::canceling(
            masonry::RotationPayload {
                object_id: object_id(3),
                rotation: masonry::Quaternion::IDENTITY,
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TransformTweenLocalRotation(masonry::PropertyCommand::canceling(
            masonry::TweenRotationPayload {
                object_id: object_id(3),
                rotation: masonry::Quaternion::IDENTITY,
                tween: masonry::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TransformTweenWorldRotation(masonry::PropertyCommand::canceling(
            masonry::TweenRotationPayload {
                object_id: object_id(3),
                rotation: masonry::Quaternion::IDENTITY,
                tween: masonry::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TransformSetLocalScale(masonry::PropertyCommand::canceling(
            masonry::ScalePayload {
                object_id: object_id(3),
                scale: Vector3::new(2.0, 2.0, 2.0),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TransformTweenLocalScale(masonry::PropertyCommand::canceling(
            masonry::TweenScalePayload {
                object_id: object_id(3),
                scale: Vector3::ONE,
                tween: masonry::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::RendererSetMaterial(masonry::PropertyCommand::canceling(
            masonry::SetMaterialPayload {
                object_id: object_id(3),
                address: "test/material".into(),
                slot: None,
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::CameraSetEnabled(masonry::ObjectEnabledPayload {
            object_id: object_id(1),
            enabled: false,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::CameraSetEnabled(masonry::ObjectEnabledPayload {
            object_id: object_id(1),
            enabled: true,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::CameraSetPerspective(masonry::PropertyCommand::canceling(
            masonry::PerspectivePayload {
                object_id: object_id(1),
                field_of_view: 70.0,
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::CameraTweenFieldOfView(masonry::PropertyCommand::canceling(
            masonry::TweenFieldOfViewPayload {
                object_id: object_id(1),
                field_of_view: 72.0,
                tween: masonry::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::CameraSetOrthographic(masonry::PropertyCommand::canceling(
            masonry::OrthographicPayload {
                object_id: object_id(1),
                size: 4.0,
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::CameraTweenOrthographicSize(masonry::PropertyCommand::canceling(
            masonry::TweenOrthographicSizePayload {
                object_id: object_id(1),
                size: 3.0,
                tween: masonry::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::CameraSetClipping(masonry::CameraClippingPayload {
            object_id: object_id(1),
            near: 0.2,
            far: 500.0,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::CameraSetClear(masonry::CameraClearPayload {
            object_id: object_id(1),
            clear_mode: masonry::CameraClearMode::SolidColor,
            clear_color: Some(masonry::Color::BLACK),
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::LightSetEnabled(masonry::ObjectEnabledPayload {
            object_id: object_id(6),
            enabled: false,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::LightSetType(masonry::LightTypePayload {
            object_id: object_id(6),
            light_type: masonry::LightType::Spot,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::LightSetColor(masonry::PropertyCommand::canceling(masonry::ColorPayload {
            object_id: object_id(6),
            color: masonry::Color::BLACK,
        })),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::LightTweenColor(masonry::PropertyCommand::canceling(
            masonry::TweenColorPayload {
                object_id: object_id(6),
                color: masonry::Color::WHITE,
                tween: masonry::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::LightSetIntensity(masonry::PropertyCommand::canceling(
            masonry::IntensityPayload {
                object_id: object_id(6),
                intensity: 2.0,
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::LightTweenIntensity(masonry::PropertyCommand::canceling(
            masonry::TweenIntensityPayload {
                object_id: object_id(6),
                intensity: 3.0,
                tween: masonry::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::LightSetRange(masonry::LightRangePayload {
            object_id: object_id(6),
            range: 20.0,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::LightSetSpotAngle(masonry::SpotAnglePayload {
            object_id: object_id(6),
            inner_spot_angle: 5.0,
            outer_spot_angle: 40.0,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::LightSetShadows(masonry::LightShadowsPayload {
            object_id: object_id(6),
            shadows: masonry::ShadowMode::Soft,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ImageSetTexture(masonry::SetTexturePayload {
            object_id: object_id(4),
            address: "test/texture".into(),
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ImageSetSize(masonry::ImageSizePayload {
            object_id: object_id(4),
            width: 2.0,
            height: 3.0,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ImageSetFit(masonry::ImageFitPayload {
            object_id: object_id(4),
            fit: masonry::ImageFit::Cover,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ImageSetTint(masonry::PropertyCommand::canceling(masonry::TintPayload {
            object_id: object_id(4),
            tint: masonry::RgbColor::BLACK,
        })),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ImageTweenTint(masonry::PropertyCommand::canceling(
            masonry::TweenTintPayload {
                object_id: object_id(4),
                tint: masonry::RgbColor::WHITE,
                tween: masonry::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ImageSetOpacity(masonry::PropertyCommand::canceling(
            masonry::OpacityPayload {
                object_id: object_id(4),
                opacity: 0.5,
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ImageTweenOpacity(masonry::PropertyCommand::canceling(
            masonry::TweenOpacityPayload {
                object_id: object_id(4),
                opacity: 0.75,
                tween: masonry::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ImageSetFaceCamera(masonry::ObjectEnabledPayload {
            object_id: object_id(4),
            enabled: true,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TextSetContent(masonry::TextContentPayload {
            object_id: object_id(5),
            text: "after".to_owned(),
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TextSetFont(masonry::SetFontPayload {
            object_id: object_id(5),
            address: "test/font".into(),
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TextSetSize(masonry::PropertyCommand::canceling(
            masonry::TextSizePayload {
                object_id: object_id(5),
                size: 2.0,
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TextTweenSize(masonry::PropertyCommand::canceling(
            masonry::TweenTextSizePayload {
                object_id: object_id(5),
                size: 3.0,
                tween: masonry::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TextSetColor(masonry::PropertyCommand::canceling(masonry::ColorPayload {
            object_id: object_id(5),
            color: masonry::Color::BLACK,
        })),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TextTweenColor(masonry::PropertyCommand::canceling(
            masonry::TweenColorPayload {
                object_id: object_id(5),
                color: masonry::Color::WHITE,
                tween: masonry::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TextSetAlignment(masonry::TextAlignmentPayload {
            object_id: object_id(5),
            horizontal: masonry::HorizontalAlignment::Left,
            vertical: masonry::VerticalAlignment::Top,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TextSetWrapping(masonry::TextWrappingPayload {
            object_id: object_id(5),
            wrap_width: Some(4.0),
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TextSetRichText(masonry::ObjectEnabledPayload {
            object_id: object_id(5),
            enabled: true,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TextSetFaceCamera(masonry::ObjectEnabledPayload {
            object_id: object_id(5),
            enabled: true,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::AnimatorPlay(masonry::AnimatorPlayPayload {
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
        CommandBody::AnimatorCrossFade(masonry::AnimatorCrossFadePayload {
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
        CommandBody::AnimatorSetBool(masonry::AnimatorBoolPayload {
            object_id: object_id(7),
            parameter: "running".to_owned(),
            value: true,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::AnimatorSetInt(masonry::AnimatorIntPayload {
            object_id: object_id(7),
            parameter: "count".to_owned(),
            value: 3,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::AnimatorSetFloat(masonry::AnimatorFloatPayload {
            object_id: object_id(7),
            parameter: "blend".to_owned(),
            value: 0.5,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::AnimatorSetTrigger(masonry::AnimatorParameterPayload {
            object_id: object_id(7),
            parameter: "fire".to_owned(),
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::AnimatorSetSpeed(masonry::AnimatorSpeedPayload {
            object_id: object_id(7),
            speed: 2.0,
        }),
    );
    push_nonblocking(
        &mut commands,
        &mut next,
        CommandBody::ParticlePlay(masonry::ParticlePlayPayload {
            object_id: object_id(7),
            restart: true,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ParticleStop(masonry::ParticleStopPayload {
            object_id: object_id(7),
            clear: true,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ParticleSpawn(masonry::ParticleSpawnPayload {
            address: "test/particles".into(),
            location: masonry::ParticleSpawnLocation::GameObject(object_id(3)),
            lifetime_ms: 1,
        }),
    );
    let audio_command_id = push_body(
        &mut commands,
        &mut next,
        CommandBody::AudioPlay(masonry::AudioPlayPayload {
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
        CommandBody::AudioSetVolume(masonry::PropertyCommand::canceling(
            masonry::AudioVolumePayload {
                audio_command_id,
                volume: 0.75,
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::AudioTweenVolume(masonry::PropertyCommand::canceling(
            masonry::TweenAudioVolumePayload {
                audio_command_id,
                volume: 1.0,
                tween: masonry::Tween::default(),
            },
        )),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::TimeWait(masonry::WaitPayload { duration_ms: 1 }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::OperationCancel(masonry::CancelOperationPayload {
            command_id: audio_command_id,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::AudioStop(masonry::AudioStopPayload {
            audio_command_id,
            fade_out_ms: 0,
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::InputSetEnabled(masonry::SetInputEnabledPayload { enabled: false }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::InputSetEnabled(masonry::SetInputEnabledPayload { enabled: true }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::InputSetCamera(masonry::ObjectIdPayload {
            object_id: object_id(1),
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::InputSetPointerEvents(masonry::PointerEventsPayload {
            object_id: object_id(3),
            events: vec![PointerEvent::Click, PointerEvent::Click],
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::InputSetGlobalKeys(masonry::GlobalKeysPayload {
            keys: vec![masonry::KeyCode::Space, masonry::KeyCode::Space],
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ObjectDestroy(masonry::ObjectIdPayload {
            object_id: object_id(8),
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::ObjectDestroy(masonry::ObjectIdPayload {
            object_id: object_id(9),
        }),
    );
    push_body(
        &mut commands,
        &mut next,
        CommandBody::SceneUnload(masonry::SceneIdPayload {
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
    assert_eq!(client.world().global_keys(), &[masonry::KeyCode::Space]);
    assert_eq!(
        client.assert_object(object_id(5)).kind(),
        &GameObjectKind::Text {
            text: masonry::TextState {
                text: "after".to_owned(),
                font: "test/font".into(),
                size: 3.0,
                color: masonry::Color::WHITE,
                horizontal: masonry::HorizontalAlignment::Left,
                vertical: masonry::VerticalAlignment::Top,
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
