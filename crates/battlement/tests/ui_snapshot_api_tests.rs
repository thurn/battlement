use battlement::{
    GameObject, GameObjectKind, ObjectId, Scene, SessionId, Snapshot, UiDocument, UiDocumentState,
    Validate, ValidationError,
};

const SESSION_ID: &str = "94fa422b-301d-442d-b9a7-10ea54318e78";
const DOCUMENT_ID: &str = "3b5fe431-f332-4314-a0f6-a7353fa17622";
const ROOT_ID: &str = "471834d0-8abc-4964-a3da-f8bc61de7c16";

#[test]
fn snapshot_requires_one_matching_document_and_root() {
    let mut snapshot = snapshot();
    assert_eq!(snapshot.validate(), Ok(()));

    snapshot.ui[0] = UiDocument::with_root_id(id(DOCUMENT_ID), ObjectId::new_v4());
    assert_eq!(snapshot.validate(), Err(ValidationError::InvalidReference));
}

#[test]
fn ui_roots_share_the_global_identity_namespace() {
    let mut snapshot = snapshot();
    snapshot
        .objects
        .push(GameObject::new(id(ROOT_ID), GameObjectKind::Empty));
    assert_eq!(snapshot.validate(), Err(ValidationError::DuplicateObject));
}

fn snapshot() -> Snapshot {
    let document_id = id(DOCUMENT_ID);
    let root_id = id(ROOT_ID);
    let scene = Scene::new(
        ObjectId::new_v4().to_string().parse().unwrap(),
        "scene/main",
    );
    let mut snapshot = Snapshot::new_with_main_camera(
        SESSION_ID.parse::<SessionId>().unwrap(),
        vec![battlement::PreparedAsset::scene("scene/main")],
        vec![scene],
        vec![GameObject::new(
            document_id,
            GameObjectKind::UiDocument(UiDocumentState::new(root_id)),
        )],
    );
    snapshot
        .ui
        .push(UiDocument::with_root_id(document_id, root_id));
    snapshot
}

fn id(value: &str) -> ObjectId {
    value.parse().unwrap()
}
