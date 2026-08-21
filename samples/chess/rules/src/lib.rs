//! Native rules engine for the standalone chess sample.

use masonry::{
    CameraState, ClientMessage, Command, Connect, CoreErrorCode, GameObject, GameObjectKind,
    GridLayout, ObjectId, ParentScene, PreparedAsset, Quaternion, Response, Scene, SceneId,
    SessionId, Snapshot, Vector3, object_id, scene_id,
};
use masonry_native::{Engine, EngineError};

const SCENE_ID: SceneId = scene_id!("36630324-bd92-4497-b328-3599930dffa9");
const PIECE_IDS: [ObjectId; 32] = [
    object_id!("3adb6e99-244d-47c1-8599-0697f311e1fc"),
    object_id!("9e94f5b6-e0f9-49f3-b770-3759d99fb16b"),
    object_id!("ab0352e6-869f-4dd7-ac86-c7436fc792ab"),
    object_id!("1465dcba-109a-4ad7-9fa0-a3553df8e2de"),
    object_id!("c64e16be-c3e3-425f-b55f-41062f124803"),
    object_id!("8c12a5c5-36d7-4c56-a0eb-ac9b1892f586"),
    object_id!("e8b4c350-7a4f-4301-8bce-7a8c2f2c6746"),
    object_id!("116a2d86-cf53-466d-b72a-29b057fc55e3"),
    object_id!("a11dbb79-1a3c-4761-9b6c-113bf58cb890"),
    object_id!("ea4e3481-34db-4c71-b12d-4a495c226757"),
    object_id!("7e119714-8c56-4c09-b47b-0953870003dd"),
    object_id!("222da336-8a5c-4e5e-b93f-5c1cd2c82144"),
    object_id!("1b502848-cf43-44fa-961d-0fa360186fba"),
    object_id!("e5021330-6761-4f88-92d5-f295a81a2644"),
    object_id!("5e81e0c4-01fe-49cb-9b4f-52917e09f531"),
    object_id!("9afb4bf4-2893-4c70-8f10-4ee88d027b7c"),
    object_id!("cf192a04-d27f-4132-82e5-941cc45ffa2b"),
    object_id!("41382898-5ca9-48b3-9111-2ab3ea98c142"),
    object_id!("570391ac-ceb5-4ba3-83ce-cebb8c33162e"),
    object_id!("6750c66a-3a07-4f4f-8256-6ad46c55193b"),
    object_id!("2f2f4744-6ff3-45ae-b03d-63637ed07313"),
    object_id!("7065f5ef-90cd-4f64-afaa-1e134064b7cb"),
    object_id!("b89582ef-8d68-400c-8a9f-46b026268150"),
    object_id!("01efbb5b-d8e4-4883-827a-5058054d1904"),
    object_id!("fd810f3e-b116-4a67-9eec-ecc4b2e5fe92"),
    object_id!("84a35802-262e-45ed-96bb-8055169a6395"),
    object_id!("3e3cde8f-f278-47dc-aad9-e6c5312e7363"),
    object_id!("d1f48e79-ce8e-4f41-ad29-e53f20a6aef3"),
    object_id!("5f703cb7-61ae-477b-8808-dd715ca85436"),
    object_id!("0e28cae9-9119-4df7-a576-9f184b47d91d"),
    object_id!("3bb6fccc-55ab-46b6-b9a1-65657e55add9"),
    object_id!("c9f6606b-d204-4f82-a3c5-7113469984e8"),
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
/// Address of the white pawn prefab.
pub const WHITE_PAWN_PREFAB: &str = "chess/white/pawn";
/// Address of the white rook prefab.
pub const WHITE_ROOK_PREFAB: &str = "chess/white/rook";
/// Address of the white knight prefab.
pub const WHITE_KNIGHT_PREFAB: &str = "chess/white/knight";
/// Address of the white bishop prefab.
pub const WHITE_BISHOP_PREFAB: &str = "chess/white/bishop";
/// Address of the white queen prefab.
pub const WHITE_QUEEN_PREFAB: &str = "chess/white/queen";
/// Address of the white king prefab.
pub const WHITE_KING_PREFAB: &str = "chess/white/king";
/// Address of the black pawn prefab.
pub const BLACK_PAWN_PREFAB: &str = "chess/black/pawn";
/// Address of the black rook prefab.
pub const BLACK_ROOK_PREFAB: &str = "chess/black/rook";
/// Address of the black knight prefab.
pub const BLACK_KNIGHT_PREFAB: &str = "chess/black/knight";
/// Address of the black bishop prefab.
pub const BLACK_BISHOP_PREFAB: &str = "chess/black/bishop";
/// Address of the black queen prefab.
pub const BLACK_QUEEN_PREFAB: &str = "chess/black/queen";
/// Address of the black king prefab.
pub const BLACK_KING_PREFAB: &str = "chess/black/king";
/// Addresses of all chess-piece prefabs.
pub const PIECE_PREFABS: [&str; 12] = [
    WHITE_PAWN_PREFAB,
    WHITE_ROOK_PREFAB,
    WHITE_KNIGHT_PREFAB,
    WHITE_BISHOP_PREFAB,
    WHITE_QUEEN_PREFAB,
    WHITE_KING_PREFAB,
    BLACK_PAWN_PREFAB,
    BLACK_ROOK_PREFAB,
    BLACK_KNIGHT_PREFAB,
    BLACK_BISHOP_PREFAB,
    BLACK_QUEEN_PREFAB,
    BLACK_KING_PREFAB,
];
/// Stable identity of the sample camera.
pub const CAMERA_ID: ObjectId = object_id!("65e0c540-8597-44a6-b4f8-2a974101bbdc");

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
        (Side::White, Piece::Pawn) => WHITE_PAWN_PREFAB,
        (Side::White, Piece::Rook) => WHITE_ROOK_PREFAB,
        (Side::White, Piece::Knight) => WHITE_KNIGHT_PREFAB,
        (Side::White, Piece::Bishop) => WHITE_BISHOP_PREFAB,
        (Side::White, Piece::Queen) => WHITE_QUEEN_PREFAB,
        (Side::White, Piece::King) => WHITE_KING_PREFAB,
        (Side::Black, Piece::Pawn) => BLACK_PAWN_PREFAB,
        (Side::Black, Piece::Rook) => BLACK_ROOK_PREFAB,
        (Side::Black, Piece::Knight) => BLACK_KNIGHT_PREFAB,
        (Side::Black, Piece::Bishop) => BLACK_BISHOP_PREFAB,
        (Side::Black, Piece::Queen) => BLACK_QUEEN_PREFAB,
        (Side::Black, Piece::King) => BLACK_KING_PREFAB,
    }
}

masonry_native::export_engine!(create_engine);
