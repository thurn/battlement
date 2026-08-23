use battlement::{
    ActionId, Command, CommandBody, ControllerButton, ControllerInputSettings, GameObject,
    ImageState, ObjectSetActivePayload, PhysicalKey, PointerEvent, Response, Scene, Snapshot,
    Vector3,
};
use battlement_native::EngineError;
use cozy_chess::Square;

use crate::{
    CAMERA_ROTATION, ChessEngine, INVALID_DROP_SOUND, MUSIC_VOLUME_STEP, PLAY_BUTTON_ID,
    REFRESH_BUTTON_ID, SCENE_ID, VOLUME_DOWN_SOUND, VOLUME_UP_SOUND, assets, audio, cursor,
};

impl ChessEngine {
    pub(crate) fn snapshot(&self) -> Snapshot {
        let mut objects = self
            .highlight_ids
            .iter()
            .zip(Square::ALL)
            .map(|(&object_id, square)| crate::highlight_object(object_id, square))
            .collect::<Vec<_>>();
        if self.started {
            objects.push(crate::refresh_button(self.screen_aspect).active(false));
            objects.push(cursor::object(self.cursor, self.cursor_visible));
            for square in Square::ALL {
                if let Some(object_id) = self.objects[square as usize] {
                    objects.push(crate::piece_object(
                        object_id,
                        square,
                        self.board
                            .color_on(square)
                            .expect("mapped pieces have a color"),
                        self.board
                            .piece_on(square)
                            .expect("mapped pieces have a type"),
                    ));
                }
            }
        } else {
            objects.push(
                GameObject::new(
                    PLAY_BUTTON_ID,
                    ImageState::new(assets::PLAY_BUTTON, 0.8, 0.24),
                )
                .position(Vector3::new(0.0, 6.38, -3.86))
                .rotation(CAMERA_ROTATION)
                .pointer_events([PointerEvent::Click]),
            );
        }
        Snapshot::new_with_main_camera(
            self.session_id,
            crate::prepared_assets(),
            vec![Scene::new(SCENE_ID, assets::CONTENT)],
            objects,
        )
        .global_keys([
            PhysicalKey::ArrowLeft,
            PhysicalKey::ArrowRight,
            PhysicalKey::ArrowUp,
            PhysicalKey::ArrowDown,
            PhysicalKey::Enter,
            PhysicalKey::NumpadEnter,
            PhysicalKey::Space,
            PhysicalKey::Escape,
            PhysicalKey::Minus,
            PhysicalKey::Equal,
        ])
        .controller_input(
            ControllerInputSettings::new()
                .buttons([
                    ControllerButton::South,
                    ControllerButton::East,
                    ControllerButton::LeftShoulder,
                    ControllerButton::RightShoulder,
                    ControllerButton::Start,
                ])
                .stick_dead_zone(0.35)
                .repeat_timing_ms(275, 125),
        )
    }

    pub(crate) fn toggle_pause(
        &mut self,
        action_id: ActionId,
    ) -> Result<Response<Command>, EngineError> {
        self.pause_open = !self.pause_open;
        self.confirm_new_game = false;
        Ok(audio::response_for_action(
            self.session_id,
            action_id,
            [CommandBody::ObjectSetActive(ObjectSetActivePayload {
                object_id: REFRESH_BUTTON_ID,
                active: self.pause_open,
            })],
        ))
    }

    pub(crate) fn handle_pause_button(
        &mut self,
        action_id: ActionId,
        button: ControllerButton,
    ) -> Result<Response<Command>, EngineError> {
        match button {
            ControllerButton::South => self.confirm_or_start_new_game(action_id, true),
            ControllerButton::East if self.confirm_new_game => {
                self.confirm_new_game = false;
                Ok(Response::empty(self.session_id))
            }
            ControllerButton::East => self.toggle_pause(action_id),
            ControllerButton::LeftShoulder => self.adjust_music(action_id, -MUSIC_VOLUME_STEP),
            ControllerButton::RightShoulder => self.adjust_music(action_id, MUSIC_VOLUME_STEP),
            _ => Ok(Response::empty(self.session_id)),
        }
    }

    pub(crate) fn confirm_or_start_new_game(
        &mut self,
        action_id: ActionId,
        cursor_visible: bool,
    ) -> Result<Response<Command>, EngineError> {
        if self.confirm_new_game {
            return self.new_game(action_id, cursor_visible);
        }
        self.confirm_new_game = true;
        Ok(audio::response_for_action(
            self.session_id,
            action_id,
            [audio::play_sound(INVALID_DROP_SOUND)],
        ))
    }

    fn adjust_music(
        &mut self,
        action_id: ActionId,
        delta: f64,
    ) -> Result<Response<Command>, EngineError> {
        let volume = self.music.set_volume(self.music.volume() + delta);
        Ok(audio::response_for_action(
            self.session_id,
            action_id,
            volume.into_iter().chain([audio::play_sound(if delta > 0.0 {
                VOLUME_UP_SOUND
            } else {
                VOLUME_DOWN_SOUND
            })]),
        ))
    }
}
