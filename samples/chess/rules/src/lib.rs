//! Native rules engine for the standalone chess sample.

use masonry::{
    CameraState, ClientMessage, Command, Connect, CoreErrorCode, GameObject, GameObjectKind,
    GridLayout, ObjectId, ParentScene, PreparedAsset, Quaternion, Response, Scene, SceneId,
    SessionId, Snapshot, Vector3, object_id, scene_id,
};
use masonry_native::{Engine, EngineError};

const SCENE_ID: SceneId = scene_id!("00000000-0000-0000-0000-000000000001");
const PIECE_IDS: [ObjectId; 32] = [
    object_id!("00000000-0000-0000-0000-000000000100"),
    object_id!("00000000-0000-0000-0000-000000000101"),
    object_id!("00000000-0000-0000-0000-000000000102"),
    object_id!("00000000-0000-0000-0000-000000000103"),
    object_id!("00000000-0000-0000-0000-000000000104"),
    object_id!("00000000-0000-0000-0000-000000000105"),
    object_id!("00000000-0000-0000-0000-000000000106"),
    object_id!("00000000-0000-0000-0000-000000000107"),
    object_id!("00000000-0000-0000-0000-000000000108"),
    object_id!("00000000-0000-0000-0000-000000000109"),
    object_id!("00000000-0000-0000-0000-00000000010a"),
    object_id!("00000000-0000-0000-0000-00000000010b"),
    object_id!("00000000-0000-0000-0000-00000000010c"),
    object_id!("00000000-0000-0000-0000-00000000010d"),
    object_id!("00000000-0000-0000-0000-00000000010e"),
    object_id!("00000000-0000-0000-0000-00000000010f"),
    object_id!("00000000-0000-0000-0000-000000000110"),
    object_id!("00000000-0000-0000-0000-000000000111"),
    object_id!("00000000-0000-0000-0000-000000000112"),
    object_id!("00000000-0000-0000-0000-000000000113"),
    object_id!("00000000-0000-0000-0000-000000000114"),
    object_id!("00000000-0000-0000-0000-000000000115"),
    object_id!("00000000-0000-0000-0000-000000000116"),
    object_id!("00000000-0000-0000-0000-000000000117"),
    object_id!("00000000-0000-0000-0000-000000000118"),
    object_id!("00000000-0000-0000-0000-000000000119"),
    object_id!("00000000-0000-0000-0000-00000000011a"),
    object_id!("00000000-0000-0000-0000-00000000011b"),
    object_id!("00000000-0000-0000-0000-00000000011c"),
    object_id!("00000000-0000-0000-0000-00000000011d"),
    object_id!("00000000-0000-0000-0000-00000000011e"),
    object_id!("00000000-0000-0000-0000-00000000011f"),
];
const BACK_RANK: [Piece; 8] = [
    Piece::Rook,
    Piece::Knight,
    Piece::Bishop,
    Piece::Queen,
    Piece::King,
    Piece::Bishop,
    Piece::Knight,
    Piece::Rook,
];

/// Address of the decorated board scene.
pub const CONTENT_SCENE: &str = "chess/content";
/// Stable identity of the sample camera.
pub const CAMERA_ID: ObjectId = object_id!("00000000-0000-0000-0000-00000000000a");

#[derive(Clone, Copy)]
enum Side {
    White,
    Black,
}

#[derive(Clone, Copy)]
enum Piece {
    Pawn,
    Rook,
    Knight,
    Bishop,
    Queen,
    King,
}

/// Native chess-scene rules engine.
pub struct ChessEngine {
    session_id: SessionId,
}

/// Creates the engine used by the native sample.
pub fn create_engine() -> Result<ChessEngine, EngineError> {
    Ok(ChessEngine {
        session_id: SessionId::new_v4(),
    })
}

impl Engine for ChessEngine {
    type ActionPayload = ();
    type ErrorCode = CoreErrorCode;
    type Command = Command;

    fn connect(&mut self, _message: Connect) -> Result<Response<Self::Command>, EngineError> {
        self.session_id = SessionId::new_v4();
        Ok(Response::snapshot(self::snapshot(self.session_id)))
    }

    fn submit(
        &mut self,
        _message: ClientMessage<Self::ActionPayload, Self::ErrorCode>,
    ) -> Result<Response<Self::Command>, EngineError> {
        Ok(Response::empty(self.session_id))
    }

    fn poll(&mut self) -> Result<Option<Response<Self::Command>>, EngineError> {
        Ok(None)
    }
}

fn snapshot(session_id: SessionId) -> Snapshot {
    let mut objects = vec![
        GameObject::new(CAMERA_ID, CameraState::new().field_of_view(60.0))
            .parent_scene(ParentScene::Persistent)
            .position(Vector3::new(0.0, 8.0, -3.75))
            .rotation(Quaternion::new(
                0.58184814,
                -0.001219943,
                0.0008727778,
                0.813296,
            )),
    ];
    objects.extend(self::pieces());

    Snapshot::new(
        session_id,
        self::prepared_assets(),
        vec![Scene::new(SCENE_ID, CONTENT_SCENE)],
        objects,
        CAMERA_ID,
    )
}

fn pieces() -> Vec<GameObject> {
    let grid = GridLayout::centered(
        Vector3::ZERO,
        8,
        8,
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    );
    PIECE_IDS
        .into_iter()
        .enumerate()
        .map(|(index, object_id)| {
            let (side, piece, column, row) = self::piece_at(index);
            let object = GameObject::new(
                object_id,
                GameObjectKind::prefab(self::address(side, piece)),
            )
            .position(grid.position(column, row));
            if matches!(side, Side::Black) {
                object.rotation(Quaternion::new(0.0, 1.0, 0.0, 0.0))
            } else {
                object
            }
        })
        .collect()
}

fn prepared_assets() -> Vec<PreparedAsset> {
    let mut assets = vec![PreparedAsset::scene(CONTENT_SCENE)];
    for side in [Side::White, Side::Black] {
        for piece in [
            Piece::Pawn,
            Piece::Rook,
            Piece::Knight,
            Piece::Bishop,
            Piece::Queen,
            Piece::King,
        ] {
            assets.push(PreparedAsset::prefab(self::address(side, piece)));
        }
    }
    assets
}

fn piece_at(index: usize) -> (Side, Piece, u32, u32) {
    match index {
        0..=7 => (Side::White, BACK_RANK[index], index as u32, 0),
        8..=15 => (Side::White, Piece::Pawn, (index - 8) as u32, 1),
        16..=23 => (Side::Black, Piece::Pawn, (index - 16) as u32, 6),
        24..=31 => (Side::Black, BACK_RANK[index - 24], (index - 24) as u32, 7),
        _ => unreachable!("chess snapshots contain exactly 32 pieces"),
    }
}

fn address(side: Side, piece: Piece) -> &'static str {
    match (side, piece) {
        (Side::White, Piece::Pawn) => "chess/white/pawn",
        (Side::White, Piece::Rook) => "chess/white/rook",
        (Side::White, Piece::Knight) => "chess/white/knight",
        (Side::White, Piece::Bishop) => "chess/white/bishop",
        (Side::White, Piece::Queen) => "chess/white/queen",
        (Side::White, Piece::King) => "chess/white/king",
        (Side::Black, Piece::Pawn) => "chess/black/pawn",
        (Side::Black, Piece::Rook) => "chess/black/rook",
        (Side::Black, Piece::Knight) => "chess/black/knight",
        (Side::Black, Piece::Bishop) => "chess/black/bishop",
        (Side::Black, Piece::Queen) => "chess/black/queen",
        (Side::Black, Piece::King) => "chess/black/king",
    }
}

masonry_native::export_engine!(create_engine);
