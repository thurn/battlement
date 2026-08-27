use battlement::{
    DocumentPivot, DocumentPosition, GameObject, GameObjectKind, Image, InteractionDistance,
    InteractionLayerMask, ObjectId, PanelInputConfiguration, PanelInputRedirection,
    PanelRenderMode, PanelScaleMode, PanelSettings, ParentScene, PivotReferenceSize, PreparedAsset,
    Scene, ScreenSize, SessionId, Snapshot, SpriteAddress, TextureAddress, UiDocument,
    UiDocumentState, UiNode, Validate, ValidationError, WorldSpaceSizeMode,
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

#[test]
fn snapshot_inserts_a_matched_ui_document_and_host() {
    let document_id = id(DOCUMENT_ID);
    let root_id = id(ROOT_ID);
    let snapshot = empty_snapshot().ui_document(UiDocument::with_root_id(document_id, root_id));

    assert_eq!(snapshot.ui.len(), 1);
    assert_eq!(snapshot.objects.len(), 1);
    assert_eq!(snapshot.objects[0].object_id, document_id);
    assert_eq!(snapshot.objects[0].parent_scene, ParentScene::Persistent);
    let GameObjectKind::UiDocument(state) = &snapshot.objects[0].kind else {
        panic!("the helper must create a UI document host");
    };
    assert_eq!(
        state.panel_settings.scale_mode,
        PanelScaleMode::ConstantPixelSize
    );
    assert_eq!(snapshot.validate(), Ok(()));
}

#[test]
fn snapshot_configures_a_ui_host_without_repeating_its_root() {
    let document_id = id(DOCUMENT_ID);
    let root_id = id(ROOT_ID);
    let snapshot = empty_snapshot().ui_document_with(
        UiDocument::with_root_id(document_id, root_id),
        ParentScene::Persistent,
        |state| state.sorting_order(12),
    );
    let GameObjectKind::UiDocument(state) = &snapshot.objects[0].kind else {
        panic!("the helper must create a UI document host");
    };
    assert_eq!(state.root_id(), root_id);
    assert_eq!(state.sorting_order, 12);
}

#[test]
fn snapshot_accepts_complete_world_document_and_process_input_configuration() {
    let document_id = id(DOCUMENT_ID);
    let root_id = id(ROOT_ID);
    let snapshot = empty_snapshot()
        .panel_input_configuration(
            PanelInputConfiguration::new()
                .interaction_layers(InteractionLayerMask::new(0x8000_0005))
                .maximum_interaction_distance(InteractionDistance::Inclusive(24.0))
                .input_redirection(PanelInputRedirection::Never),
        )
        .ui_document_with(
            UiDocument::with_root_id(document_id, root_id),
            ParentScene::Persistent,
            |state| {
                state
                    .panel_settings(PanelSettings::new().render_mode(PanelRenderMode::WorldSpace))
                    .position(DocumentPosition::Absolute)
                    .world_space_size_mode(WorldSpaceSizeMode::Fixed)
                    .world_space_size(ScreenSize::new(420, 240))
                    .pivot_reference_size(PivotReferenceSize::Layout)
                    .pivot(DocumentPivot::BottomRight)
                    .sorting_order(17)
            },
        );

    assert_eq!(snapshot.validate(), Ok(()));
    assert_eq!(
        snapshot.panel_input_configuration.interaction_layers,
        InteractionLayerMask::new(0x8000_0005)
    );
}

#[test]
fn screen_and_dynamic_world_documents_reject_inapplicable_geometry() {
    let mut screen = snapshot();
    let GameObjectKind::UiDocument(state) = &mut screen.objects[0].kind else {
        panic!("fixture must contain a UI host");
    };
    state.world_space_size = ScreenSize::new(420, 240);
    assert_eq!(screen.validate(), Err(ValidationError::InvalidReference));

    let mut dynamic = snapshot();
    let GameObjectKind::UiDocument(state) = &mut dynamic.objects[0].kind else {
        panic!("fixture must contain a UI host");
    };
    state.panel_settings.render_mode = PanelRenderMode::WorldSpace;
    state.world_space_size_mode = WorldSpaceSizeMode::Dynamic;
    state.world_space_size = ScreenSize::new(420, 240);
    assert_eq!(dynamic.validate(), Err(ValidationError::InvalidReference));
}

#[test]
fn ui_documents_cannot_be_nested_beneath_document_hosts() {
    let parent_id = ObjectId::new_v4();
    let child_id = ObjectId::new_v4();
    let middle_id = ObjectId::new_v4();
    let parent_root = ObjectId::new_v4();
    let child_root = ObjectId::new_v4();
    let mut direct = empty_snapshot()
        .ui_document(UiDocument::with_root_id(parent_id, parent_root))
        .ui_document(UiDocument::with_root_id(child_id, child_root));
    direct.objects[1].parent_id = Some(parent_id);
    assert_eq!(direct.validate(), Err(ValidationError::InvalidHierarchy));

    let mut deeper = empty_snapshot()
        .ui_document(UiDocument::with_root_id(parent_id, parent_root))
        .ui_document(UiDocument::with_root_id(child_id, child_root));
    deeper.objects.push(
        GameObject::new(middle_id, GameObjectKind::Empty)
            .parent_scene(ParentScene::Persistent)
            .parent_id(parent_id),
    );
    deeper.objects[1].parent_id = Some(middle_id);
    assert_eq!(deeper.validate(), Err(ValidationError::InvalidHierarchy));
}

#[test]
fn ui_image_requires_the_exact_prepared_asset_kind() {
    let mut value = snapshot();
    value.ui[0].children.push(UiNode::new(
        ObjectId::new_v4(),
        Image::new().source(TextureAddress::new("ui/gallery/art")),
    ));
    assert_eq!(value.validate(), Err(ValidationError::InvalidReference));

    value
        .prepared_assets
        .push(PreparedAsset::Sprite(SpriteAddress::new("ui/gallery/art")));
    assert_eq!(value.validate(), Err(ValidationError::InvalidReference));

    value.prepared_assets.pop();
    value
        .prepared_assets
        .push(PreparedAsset::Texture(TextureAddress::new(
            "ui/gallery/art",
        )));
    assert_eq!(value.validate(), Ok(()));
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

fn empty_snapshot() -> Snapshot {
    let scene = Scene::new(
        ObjectId::new_v4().to_string().parse().unwrap(),
        "scene/main",
    );
    Snapshot::new_with_main_camera(
        SESSION_ID.parse::<SessionId>().unwrap(),
        vec![battlement::PreparedAsset::scene("scene/main")],
        vec![scene],
        Vec::new(),
    )
}

fn id(value: &str) -> ObjectId {
    value.parse().unwrap()
}
