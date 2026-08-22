use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::{Duration, Instant},
};

use masonry::{
    CommandBody, Connect, DragMode, GameObjectKind, KeyCode, ObjectId, PointerButton,
    ScreenPosition, ScreenSize, Vector3,
};
use masonry_fake::{
    assets::{FakeAssetCatalog, FakePrefab},
    client::{FakeClient, PointerInput},
    time::ManualClock,
};
use masonry_rules::{
    CRITICAL_BEAT_INTERVAL_MS, CRITICAL_FIRST_BEAT_OFFSET_MS, ChessEngine, MUSIC_TRACKS,
    PIECE_PREFABS, PIECE_SPAWN_BEAT_COUNT, PIECE_SPAWN_SEQUENCE_DURATION_MS, PLAY_BUTTON_ID,
    REFRESH_BUTTON_ID,
    assets::{self, black, effects, white},
    audio::SOUND_EFFECTS,
    create_engine, create_engine_with_clock, create_engine_with_position,
    create_engine_with_think_time, create_seeded_engine,
};

static NEXT_TEMP_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

#[test]
fn initial_world_displays_play_without_creating_pieces() {
    let client = FakeClient::connect(
        create_engine().expect("engine should initialize"),
        self::assets(),
    );

    assert!(
        !client
            .world()
            .objects()
            .any(|object| { matches!(object.kind(), GameObjectKind::Prefab { .. }) })
    );
    let button = client.world().object(PLAY_BUTTON_ID).expect("Play button");
    assert_eq!(button.pointer_events(), &[masonry::PointerEvent::Click]);
    let rotation = button.local_transform().rotation;
    assert!((rotation.x - 0.58184814).abs() < 1e-6);
    assert!((rotation.y + 0.001219943).abs() < 1e-6);
    assert!((rotation.z - 0.0008727778).abs() < 1e-6);
    assert!((rotation.w - 0.813296).abs() < 1e-6);
    assert!(matches!(
        button.kind(),
        GameObjectKind::Image { image }
            if image.texture == assets::PLAY_BUTTON
                && image.width == 0.8
                && image.height == 0.24
                && !image.face_camera
    ));
    let highlights = client
        .world()
        .objects()
        .filter(|object| matches!(object.kind(), GameObjectKind::Plane { .. }))
        .collect::<Vec<_>>();
    assert_eq!(highlights.len(), 64);
    assert!(highlights.iter().all(|highlight| {
        !highlight.active_self()
            && highlight.pointer_events() == [masonry::PointerEvent::Click]
            && highlight.drag_mode().is_none()
            && highlight.material(0) == Some(&assets::LEGAL_SQUARE)
    }));
    assert!(client.world().object(REFRESH_BUTTON_ID).is_none());
}

#[test]
fn clicking_a_piece_then_a_legal_square_moves_it_with_a_tween() {
    let mut client = self::started_client(create_engine_with_think_time(Duration::from_secs(1)));
    let from = self::square('e', 2);
    let to = self::square('e', 4);
    let pawn = self::piece_at(&client, from);

    self::select(&mut client, pawn, from);

    assert_eq!(
        self::active_highlight_squares(&client),
        vec![self::square('e', 3), to]
    );
    client.click(self::highlight_at(&client, to));

    client.assert_world_position(pawn, to, 1e-9);
    assert!(self::active_highlight_squares(&client).is_empty());
    assert!(client.commands().iter().any(|entry| {
        matches!(
            &entry.command.body,
            CommandBody::TransformTweenWorldPosition(command)
                if command.payload.object_id == pawn
                    && command.payload.position == to
                    && command.payload.tween.duration_ms > 0
        )
    }));
}

#[test]
fn click_to_move_animates_a_knight_along_two_sides_of_its_l_shape() {
    let mut client = self::positioned_client("4k3/8/8/8/8/8/8/1N2K3 w - - 0 1");
    let from = self::square('b', 1);
    let corner = self::square('b', 3);
    let to = self::square('c', 3);
    let knight = self::piece_at(&client, from);
    let command_count = client.commands().len();

    self::select(&mut client, knight, from);
    client.click(self::highlight_at(&client, to));

    let path = client.commands()[command_count..]
        .iter()
        .filter_map(|entry| match &entry.command.body {
            CommandBody::TransformTweenWorldPosition(command)
                if command.payload.object_id == knight =>
            {
                Some((entry.group_index, command.payload.position))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(path, vec![(0, corner), (1, to)]);
    client.assert_world_position(knight, to, 1e-9);
}

#[test]
fn click_capture_removes_the_captured_piece_after_the_move_reaches_its_square() {
    let mut client = self::positioned_client("4k3/8/8/4p3/3B4/8/8/4K3 w - - 0 1");
    let from = self::square('d', 4);
    let to = self::square('e', 5);
    let bishop = self::piece_at(&client, from);
    let captured = self::piece_at(&client, to);
    let command_count = client.commands().len();

    self::select(&mut client, bishop, from);
    client.click(captured);

    let commands = &client.commands()[command_count..];
    let move_group = commands
        .iter()
        .find_map(|entry| match &entry.command.body {
            CommandBody::TransformTweenWorldPosition(command)
                if command.payload.object_id == bishop =>
            {
                Some(entry.group_index)
            }
            _ => None,
        })
        .expect("capturing piece should animate");
    let capture_group = commands
        .iter()
        .find_map(|entry| match entry.command.body {
            CommandBody::ObjectDestroy(command) if command.object_id == captured => {
                Some(entry.group_index)
            }
            _ => None,
        })
        .expect("captured piece should be removed");
    assert!(capture_group > move_group);
    assert!(client.world().object(captured).is_none());
}

#[test]
fn knight_capture_finishes_both_legs_before_removing_the_captured_piece() {
    let mut client = self::positioned_client("4k3/8/8/3p4/8/2N5/8/4K3 w - - 0 1");
    let from = self::square('c', 3);
    let to = self::square('d', 5);
    let knight = self::piece_at(&client, from);
    let captured = self::piece_at(&client, to);
    let command_count = client.commands().len();

    self::select(&mut client, knight, from);
    client.click(captured);

    let commands = &client.commands()[command_count..];
    let last_move_group = commands
        .iter()
        .filter_map(|entry| match &entry.command.body {
            CommandBody::TransformTweenWorldPosition(command)
                if command.payload.object_id == knight =>
            {
                Some(entry.group_index)
            }
            _ => None,
        })
        .max()
        .expect("knight should animate both legs");
    let capture_group = commands
        .iter()
        .find_map(|entry| match entry.command.body {
            CommandBody::ObjectDestroy(command) if command.object_id == captured => {
                Some(entry.group_index)
            }
            _ => None,
        })
        .expect("captured piece should be removed");
    assert!(capture_group > last_move_group);
}

#[test]
fn dragging_a_piece_highlights_its_legal_destinations_until_drop() {
    let mut client = self::started_client(create_engine_with_think_time(Duration::from_secs(1)));
    let pawn = self::piece_at(&client, self::square('e', 2));
    let pointer = self::pointer_input(self::square('e', 2));

    client.drag_start(pawn, pointer);

    assert!(self::played_sfx(&client).last().is_some_and(|sound| {
        ["sfx/click", "sfx/click-2", "sfx/click-3", "sfx/click-4"].contains(sound)
    }));
    assert_eq!(
        self::active_highlight_squares(&client),
        vec![self::square('e', 3), self::square('e', 4)]
    );

    client.drag_end(pawn, pointer, self::square('e', 4));

    assert!(self::active_highlight_squares(&client).is_empty());
    assert!(self::played_sfx(&client).last().is_some_and(|sound| {
        [
            "sfx/bounce-0",
            "sfx/bounce-1",
            "sfx/bounce-2",
            "sfx/bounce-3",
        ]
        .contains(sound)
    }));
}

#[test]
fn picking_up_any_player_piece_never_sends_an_empty_command_group() {
    let mut client = self::started_client(create_engine_with_think_time(Duration::from_secs(1)));

    for rank in [1, 2] {
        for file in 'a'..='h' {
            let square = self::square(file, rank);
            let piece = self::piece_at(&client, square);
            let pointer = self::pointer_input(square);

            client.drag_start(piece, pointer);
            client.drag_end(piece, pointer, square);
        }
    }
}

#[test]
fn castling_highlights_the_visible_king_destinations() {
    let mut client = self::positioned_client("4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1");
    let king = self::piece_at(&client, self::square('e', 1));

    client.drag_start(king, self::pointer_input(self::square('e', 1)));

    let highlights = self::active_highlight_squares(&client);
    assert!(highlights.contains(&self::square('c', 1)));
    assert!(highlights.contains(&self::square('g', 1)));
    assert!(!highlights.contains(&self::square('a', 1)));
    assert!(!highlights.contains(&self::square('h', 1)));
}

#[test]
fn refresh_control_appears_after_play() {
    let mut client = FakeClient::connect(
        create_engine().expect("engine should initialize"),
        self::assets(),
    );
    assert!(client.world().object(REFRESH_BUTTON_ID).is_none());

    client.click(PLAY_BUTTON_ID);

    let refresh = client
        .world()
        .object(REFRESH_BUTTON_ID)
        .expect("Refresh button");
    assert_eq!(refresh.pointer_events(), &[masonry::PointerEvent::Click]);
    assert!(matches!(
        refresh.kind(),
        GameObjectKind::Image { image }
            if image.texture == assets::REFRESH_BUTTON
                && image.width == 0.16
                && image.height == 0.16
    ));
    assert!(refresh.local_transform().position.x > 0.0);
}

#[test]
fn saved_position_opens_on_the_next_launch() {
    let directory = TempDirectory::new();
    let connect = self::persistent_connect(directory.path());
    let mut client = FakeClient::connect_with(
        create_engine_with_position("7k/8/5KQ1/8/8/8/8/8 w - - 0 1", Duration::from_secs(1))
            .expect("position should be valid"),
        self::assets(),
        connect.clone(),
    );
    client.click(PLAY_BUTTON_ID);
    let queen = self::piece_at(&client, self::square('g', 6));

    self::drag(
        &mut client,
        queen,
        self::square('g', 6),
        self::square('g', 7),
    );
    drop(client);

    let restored = FakeClient::connect_with(
        create_engine().expect("engine should initialize"),
        self::assets(),
        connect,
    );
    assert!(restored.world().object(PLAY_BUTTON_ID).is_none());
    assert!(restored.world().object(REFRESH_BUTTON_ID).is_some());
    self::piece_at(&restored, self::square('g', 7));
    assert!(directory.path().join("chess-game.json").is_file());
}

#[test]
fn refresh_button_starts_the_position_over() {
    let mut client = self::positioned_client("7k/8/5KQ1/8/8/8/8/8 w - - 0 1");
    let queen = self::piece_at(&client, self::square('g', 6));
    let previous_piece_ids = client
        .world()
        .objects()
        .filter(|object| matches!(object.kind(), GameObjectKind::Prefab { .. }))
        .map(|object| object.id())
        .collect::<Vec<_>>();

    self::drag(
        &mut client,
        queen,
        self::square('g', 6),
        self::square('g', 7),
    );
    assert!(client.world().input_enabled());
    client.click(REFRESH_BUTTON_ID);

    assert_eq!(
        self::played_sfx(&client).last(),
        Some(&"sfx/scene-transition")
    );
    self::piece_at(&client, self::square('g', 6));
    assert!(client.world().objects().all(|object| {
        !matches!(object.kind(), GameObjectKind::Prefab { .. })
            || !previous_piece_ids.contains(&object.id())
    }));
    assert!(
        client
            .world()
            .objects()
            .all(|object| object.local_transform().position != self::square('g', 7))
    );
}

#[test]
fn play_click_creates_a_standard_player_facing_position() {
    let client = self::started_client(create_engine().expect("engine should initialize"));
    let pieces = client
        .world()
        .objects()
        .filter(|object| matches!(object.kind(), GameObjectKind::Prefab { .. }))
        .collect::<Vec<_>>();

    assert_eq!(pieces.len(), 32);
    assert!(client.world().uses_main_camera());
    assert!(
        client
            .world()
            .objects()
            .all(|object| !matches!(object.kind(), GameObjectKind::Camera { .. }))
    );
    assert_eq!(
        pieces
            .iter()
            .filter(|piece| piece.drag_mode() == Some(DragMode::SnapToPointer))
            .count(),
        16
    );
    assert_eq!(
        pieces
            .iter()
            .filter(|piece| piece.drag_mode().is_none())
            .count(),
        16
    );
    let white_queen = client
        .world()
        .object(self::piece_at(&client, self::square('d', 1)))
        .unwrap();
    let black_king = client
        .world()
        .object(self::piece_at(&client, self::square('e', 8)))
        .unwrap();
    assert!(matches!(
        white_queen.kind(),
        GameObjectKind::Prefab { address, .. } if address == &white::QUEEN
    ));
    assert!(matches!(
        black_king.kind(),
        GameObjectKind::Prefab { address, .. } if address == &black::KING
    ));
}

#[test]
fn play_click_randomizes_both_sides_and_spawns_four_pieces_on_each_of_eight_beats() {
    let first = self::started_client(create_seeded_engine(1));
    let second = self::started_client(create_seeded_engine(2));
    let spawn_order = |client: &FakeClient<ChessEngine>| {
        client
            .commands()
            .iter()
            .filter_map(|entry| match &entry.command.body {
                CommandBody::ObjectCreate(value) => Some(value.object.object_id),
                _ => None,
            })
            .collect::<Vec<_>>()
    };

    assert_ne!(spawn_order(&first), spawn_order(&second));
    let effects = first
        .commands()
        .iter()
        .filter(|entry| matches!(entry.command.body, CommandBody::ParticleSpawn(_)))
        .collect::<Vec<_>>();
    let waits = first
        .commands()
        .iter()
        .filter_map(|entry| match entry.command.body {
            CommandBody::TimeWait(wait) => Some(wait.duration_ms),
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut pieces_per_beat = Vec::new();
    let mut current_beat = None;
    for entry in first.commands() {
        match &entry.command.body {
            CommandBody::TimeWait(_) => {
                if let Some(piece_count) = current_beat.replace(0) {
                    pieces_per_beat.push(piece_count);
                }
            }
            CommandBody::ObjectCreate(value)
                if current_beat.is_some()
                    && matches!(value.object.kind, GameObjectKind::Prefab { .. }) =>
            {
                current_beat = current_beat.map(|piece_count| piece_count + 1);
            }
            _ => {}
        }
    }
    pieces_per_beat.extend(current_beat);

    assert_eq!(effects.len(), 32);
    assert!(effects.iter().all(|entry| {
        matches!(
            &entry.command.body,
            CommandBody::ParticleSpawn(effect)
                if effect.address == effects::PIECE_SPAWN
                    && effect.lifetime_ms == 1_000
                    && !entry.command.blocking
        )
    }));
    assert_eq!(waits, [80, 570, 570, 570, 570, 570, 570, 570]);
    assert_eq!(pieces_per_beat, [4, 4, 4, 4, 4, 4, 4, 4]);
    assert_eq!(CRITICAL_FIRST_BEAT_OFFSET_MS, 80);
    assert_eq!(CRITICAL_BEAT_INTERVAL_MS, 570);
    assert_eq!(PIECE_SPAWN_BEAT_COUNT, 8);
    assert_eq!(PIECE_SPAWN_SEQUENCE_DURATION_MS, 4_070);
    assert_eq!(self::played_music(&first), [MUSIC_TRACKS[0].as_str()]);
    assert!(first.world().input_enabled());
    assert!(self::played_sfx(&first).contains(&"sfx/accept"));
}

#[test]
fn illegal_drag_returns_the_piece_to_its_square() {
    let mut client = FakeClient::connect(
        create_engine().expect("engine should initialize"),
        self::assets(),
    );
    client.click(PLAY_BUTTON_ID);
    let pawn = self::piece_at(&client, Vector3::new(0.5, 0.0, -2.5));
    let pointer = self::pointer_input(Vector3::new(0.5, 0.0, -2.5));

    client.drag_start(pawn, pointer);
    client.drag_end(pawn, pointer, Vector3::new(0.5, 0.0, 0.5));

    client.assert_world_position(pawn, Vector3::new(0.5, 0.0, -2.5), 1e-9);
    assert!(client.world().input_enabled());
    assert_eq!(self::played_sfx(&client).last(), Some(&"sfx/error"));
}

#[test]
fn legal_drag_starts_a_nonblocking_ai_turn_and_applies_its_reply() {
    let mut client = FakeClient::connect(
        create_engine_with_think_time(Duration::from_millis(75)),
        self::assets(),
    );
    client.click(PLAY_BUTTON_ID);
    let pawn = self::piece_at(&client, Vector3::new(0.5, 0.0, -2.5));
    let pointer = self::pointer_input(Vector3::new(0.5, 0.0, -2.5));
    let leftmost_pawn = self::piece_at(&client, self::square('a', 7));
    let submitted_at = Instant::now();

    client.drag_start(pawn, pointer);
    client.drag_end(pawn, pointer, Vector3::new(0.5, 0.0, -0.5));

    assert!(submitted_at.elapsed() < Duration::from_millis(50));
    client.assert_world_position(pawn, Vector3::new(0.5, 0.0, -0.5), 1e-9);
    assert!(!client.world().input_enabled());

    let deadline = Instant::now() + Duration::from_secs(2);
    while !client.world().input_enabled() && Instant::now() < deadline {
        client.poll();
        thread::sleep(Duration::from_millis(5));
    }

    assert!(
        client.world().input_enabled(),
        "AI did not finish before timeout"
    );
    assert!(submitted_at.elapsed() >= Duration::from_millis(70));
    client.assert_world_position(leftmost_pawn, self::square('a', 7), 1e-9);
    assert!(client.world().objects().any(|object| {
        matches!(object.kind(), GameObjectKind::Prefab { address, .. }
            if address.as_str().starts_with("black/"))
            && object.local_transform().position.z < 2.5
    }));
}

#[test]
fn castling_moves_the_king_and_rook_to_their_visible_squares() {
    let mut client = self::positioned_client("4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1");
    let king = self::piece_at(&client, self::square('e', 1));
    let rook = self::piece_at(&client, self::square('h', 1));

    self::drag(
        &mut client,
        king,
        self::square('e', 1),
        self::square('g', 1),
    );

    client.assert_world_position(king, self::square('g', 1), 1e-9);
    client.assert_world_position(rook, self::square('f', 1), 1e-9);
    assert!(self::played_sfx(&client).contains(&"sfx/powerup-a"));
}

#[test]
fn en_passant_removes_the_captured_pawn_from_the_world() {
    let mut client = self::positioned_client("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1");
    let pawn = self::piece_at(&client, self::square('e', 5));
    let captured = self::piece_at(&client, self::square('d', 5));

    self::drag(
        &mut client,
        pawn,
        self::square('e', 5),
        self::square('d', 6),
    );

    client.assert_world_position(pawn, self::square('d', 6), 1e-9);
    assert!(client.world().object(captured).is_none());
    assert!(client.commands().iter().any(|entry| {
        matches!(
            &entry.command.body,
            CommandBody::ParticleSpawn(effect)
                if effect.address == effects::CAPTURE
                    && effect.location
                        == masonry::ParticleSpawnLocation::WorldPosition(self::square('d', 5))
                    && effect.lifetime_ms == 2_000
                    && !entry.command.blocking
        )
    }));
    assert!(self::played_sfx(&client).last().is_some_and(|sound| {
        [
            "sfx/attack-a",
            "sfx/attack-b",
            "sfx/attack-c",
            "sfx/attack-d",
        ]
        .contains(sound)
    }));
}

#[test]
fn promotion_replaces_the_pawn_with_a_draggable_queen() {
    let mut client = self::positioned_client("4k3/P7/8/8/8/8/8/4K3 w - - 0 1");
    let pawn = self::piece_at(&client, self::square('a', 7));

    self::drag(
        &mut client,
        pawn,
        self::square('a', 7),
        self::square('a', 8),
    );

    assert!(client.world().object(pawn).is_none());
    let queen = client
        .world()
        .objects()
        .find(|object| object.local_transform().position == self::square('a', 8))
        .expect("promotion should create a piece on a8");
    assert!(matches!(
        queen.kind(),
        GameObjectKind::Prefab { address, .. } if address == &white::QUEEN
    ));
    assert_eq!(queen.drag_mode(), Some(DragMode::SnapToPointer));
    assert!(self::played_sfx(&client).contains(&"sfx/powerup-b"));
}

#[test]
fn checking_move_plays_the_alarm() {
    let mut client = self::positioned_client("4k3/8/8/8/8/8/R7/4K3 w - - 0 1");
    let rook = self::piece_at(&client, self::square('a', 2));

    self::drag(
        &mut client,
        rook,
        self::square('a', 2),
        self::square('e', 2),
    );

    assert!(self::played_sfx(&client).contains(&"sfx/alarm"));
}

#[test]
fn checkmate_plays_the_player_victory_sound() {
    let mut client = self::positioned_client("7k/5K2/6Q1/8/8/8/8/8 w - - 0 1");
    let queen = self::piece_at(&client, self::square('g', 6));

    self::drag(
        &mut client,
        queen,
        self::square('g', 6),
        self::square('g', 7),
    );

    assert!(self::played_sfx(&client).contains(&"sfx/lap-complete"));
}

#[test]
fn ai_plays_an_available_checkmate_through_polling() {
    let mut client = FakeClient::connect(
        create_engine_with_position("8/8/8/8/8/5kq1/8/7K b - - 0 1", Duration::from_millis(100))
            .expect("position should be valid"),
        self::assets(),
    );
    client.click(PLAY_BUTTON_ID);
    let queen = self::piece_at(&client, self::square('g', 3));
    let deadline = Instant::now() + Duration::from_secs(2);

    while client.world().world_transform(queen).position == self::square('g', 3)
        && Instant::now() < deadline
    {
        client.poll();
        thread::sleep(Duration::from_millis(5));
    }

    client.assert_world_position(queen, self::square('g', 2), 1e-9);
    assert!(client.world().input_enabled());
    assert!(self::played_sfx(&client).contains(&"sfx/fall-and-die"));
}

#[test]
fn music_loops_for_two_minutes_then_crossfades_in_playlist_order() {
    let (mut client, clock) = self::clocked_client();

    client.poll();
    assert_eq!(self::played_music(&client), vec![MUSIC_TRACKS[0].as_str()]);

    for expected in MUSIC_TRACKS.iter().cycle().skip(1).take(MUSIC_TRACKS.len()) {
        clock.advance(Duration::from_secs(119));
        client.poll();
        assert_ne!(
            self::played_music(&client).last().copied(),
            Some(expected.as_str())
        );
        clock.advance(Duration::from_secs(1));
        client.poll();
        assert_eq!(
            self::played_music(&client).last().copied(),
            Some(expected.as_str())
        );
    }
}

#[test]
fn arrow_keys_control_background_music_volume_from_rust() {
    let (mut client, _) = self::clocked_client();
    client.poll();
    let play_id = client
        .commands()
        .iter()
        .find_map(|entry| {
            matches!(
                &entry.command.body,
                CommandBody::AudioPlay(play)
                    if play.address.as_str().starts_with("music/")
            )
            .then_some(entry.command.command_id)
        })
        .expect("music should start on the first poll");

    client.key_down(KeyCode::ArrowUp);
    assert!((client.world().audio(play_id).unwrap().volume() - 0.45).abs() < 1e-9);
    assert_eq!(self::played_sfx(&client).last(), Some(&"sfx/chirp-a"));
    client.key_up(KeyCode::ArrowUp);
    for _ in 0..10 {
        client.key_down(KeyCode::ArrowDown);
        client.key_up(KeyCode::ArrowDown);
    }
    assert_eq!(client.world().audio(play_id).unwrap().volume(), 0.0);
    assert_eq!(self::played_sfx(&client).last(), Some(&"sfx/chirp-crunch"));
}

fn piece_at(client: &FakeClient<ChessEngine>, position: Vector3) -> ObjectId {
    client
        .world()
        .objects()
        .find(|object| object.local_transform().position == position)
        .expect("expected a piece on the requested square")
        .id()
}

fn highlight_at(client: &FakeClient<ChessEngine>, position: Vector3) -> ObjectId {
    client
        .world()
        .objects()
        .find(|object| {
            matches!(object.kind(), GameObjectKind::Plane { .. })
                && object.local_transform().position.x == position.x
                && object.local_transform().position.z == position.z
        })
        .expect("expected a highlight on the requested square")
        .id()
}

fn pointer_input(world_hit: Vector3) -> PointerInput {
    PointerInput {
        pointer_id: 0,
        screen_position: ScreenPosition::new(500.0, 300.0),
        world_hit,
        button: PointerButton::Left,
    }
}

fn positioned_client(fen: &str) -> FakeClient<ChessEngine> {
    self::started_client(
        create_engine_with_position(fen, Duration::from_secs(1)).expect("position should be valid"),
    )
}

fn drag(client: &mut FakeClient<ChessEngine>, piece: ObjectId, from: Vector3, to: Vector3) {
    let pointer = self::pointer_input(from);
    client.drag_start(piece, pointer);
    client.drag_end(piece, pointer, to);
}

fn select(client: &mut FakeClient<ChessEngine>, piece: ObjectId, square: Vector3) {
    self::drag(client, piece, square, square);
}

fn square(file: char, rank: u8) -> Vector3 {
    Vector3::new(
        f64::from(file as u8 - b'a') - 3.5,
        0.0,
        f64::from(rank) - 4.5,
    )
}

fn assets() -> FakeAssetCatalog {
    let mut assets = FakeAssetCatalog::new();
    assets.add_scene(assets::CONTENT);
    for address in PIECE_PREFABS {
        assets.add_prefab(
            address,
            FakePrefab::new()
                .with_material_slots(1)
                .with_pointer_collider(),
        );
    }
    for address in MUSIC_TRACKS {
        assets.add_audio_clip(address);
    }
    for address in SOUND_EFFECTS {
        assets.add_audio_clip(address);
    }
    assets.add_texture(assets::PLAY_BUTTON);
    assets.add_material(assets::LEGAL_SQUARE);
    assets.add_texture(assets::REFRESH_BUTTON);
    assets.add_particle_effect(effects::PIECE_SPAWN);
    assets.add_particle_effect(effects::CAPTURE);
    assets
}

fn active_highlight_squares(client: &FakeClient<ChessEngine>) -> Vec<Vector3> {
    let mut squares = client
        .world()
        .objects()
        .filter(|object| {
            object.active_self() && matches!(object.kind(), GameObjectKind::Plane { .. })
        })
        .map(|object| {
            let position = object.local_transform().position;
            Vector3::new(position.x, 0.0, position.z)
        })
        .collect::<Vec<_>>();
    squares.sort_by(|left, right| left.z.total_cmp(&right.z).then(left.x.total_cmp(&right.x)));
    squares
}

fn persistent_connect(path: &Path) -> Connect {
    Connect::new(
        "masonry-fake",
        "masonry-fake",
        ScreenSize::new(1_920, 1_080),
    )
    .persistent_data_path(path.to_string_lossy())
}

fn clocked_client() -> (FakeClient<ChessEngine>, ManualClock) {
    let (mut client, clock) = FakeClient::connect_clocked(
        |clock| create_engine_with_clock(move || clock.now()),
        self::assets(),
    );
    client.click(PLAY_BUTTON_ID);
    (client, clock)
}

fn started_client(engine: ChessEngine) -> FakeClient<ChessEngine> {
    let mut client = FakeClient::connect(engine, self::assets());
    client.click(PLAY_BUTTON_ID);
    client
}

fn played_music(client: &FakeClient<ChessEngine>) -> Vec<&str> {
    client
        .commands()
        .iter()
        .filter_map(|entry| match &entry.command.body {
            CommandBody::AudioPlay(play) if play.address.as_str().starts_with("music/") => {
                Some(play.address.as_str())
            }
            _ => None,
        })
        .collect()
}

fn played_sfx(client: &FakeClient<ChessEngine>) -> Vec<&str> {
    client
        .commands()
        .iter()
        .filter_map(|entry| match &entry.command.body {
            CommandBody::AudioPlay(play) if play.address.as_str().starts_with("sfx/") => {
                Some(play.address.as_str())
            }
            _ => None,
        })
        .collect()
}

struct TempDirectory(PathBuf);

impl TempDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "masonry-chess-{}-{}",
            std::process::id(),
            NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("temporary save directory should be created");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("temporary save directory should be removed");
    }
}
