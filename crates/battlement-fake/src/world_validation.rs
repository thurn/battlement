//! Fake-specific catalog and object-reference validation.

use battlement::{AnimatorState, GameObject, GameObjectKind, MaterialAssignment, PreparedAsset};

use crate::assets;

pub(crate) fn validate_object_assets(
    object: &GameObject,
    catalog: &assets::FakeAssetCatalog,
    prepared_assets: &[PreparedAsset],
) {
    match &object.kind {
        GameObjectKind::Image { image } => {
            assert_prepared(
                prepared_assets,
                PreparedAsset::Texture(image.texture.clone()),
            );
            assert!(
                catalog.has_texture(&image.texture),
                "unknown texture asset: {}",
                image.texture
            );
        }
        GameObjectKind::Text { text } => {
            assert_prepared(prepared_assets, PreparedAsset::Font(text.font.clone()));
            assert!(
                catalog.has_font(&text.font),
                "unknown font asset: {}",
                text.font
            );
        }
        GameObjectKind::Prefab {
            address,
            materials,
            animator,
        } => {
            assert_prepared(prepared_assets, PreparedAsset::Prefab(address.clone()));
            let prefab = catalog
                .prefab(address)
                .unwrap_or_else(|| panic!("unknown prefab asset: {address}"));
            validate_material_assignments(
                materials,
                prefab.material_slots(),
                catalog,
                prepared_assets,
            );
            if let Some(state) = animator {
                let descriptor = prefab
                    .animator()
                    .unwrap_or_else(|| panic!("prefab has no Animator: {address}"));
                validate_animator_state(state, descriptor);
            }
        }
        GameObjectKind::Cube { materials }
        | GameObjectKind::Sphere { materials }
        | GameObjectKind::Capsule { materials }
        | GameObjectKind::Cylinder { materials }
        | GameObjectKind::Plane { materials }
        | GameObjectKind::Quad { materials } => {
            validate_material_assignments(materials, Some(1), catalog, prepared_assets);
        }
        GameObjectKind::Empty
        | GameObjectKind::UiDocument(_)
        | GameObjectKind::Camera { .. }
        | GameObjectKind::Light { .. } => {}
    }
}

fn validate_material_assignments(
    materials: &[MaterialAssignment],
    slots: Option<usize>,
    catalog: &assets::FakeAssetCatalog,
    prepared_assets: &[PreparedAsset],
) {
    if !materials.is_empty() {
        let slots = slots.unwrap_or_else(|| panic!("material assignment requires a renderer"));
        for assignment in materials {
            assert!(
                usize::try_from(assignment.slot).is_ok_and(|slot| slot < slots),
                "material slot out of range: {}",
                assignment.slot
            );
            assert_prepared(
                prepared_assets,
                PreparedAsset::Material(assignment.address.clone()),
            );
            assert!(
                catalog.has_material(&assignment.address),
                "unknown material asset: {}",
                assignment.address
            );
        }
    }
}

fn validate_animator_state(state: &AnimatorState, descriptor: &assets::FakeAnimator) {
    assert!(
        descriptor.has_state(state.layer, &state.state),
        "unknown animator state: {}",
        state.state
    );
    for name in state.bool_parameters.keys() {
        assert!(
            descriptor.has_parameter(name, assets::ParameterKind::Bool),
            "unknown bool parameter: {name}"
        );
    }
    for name in state.int_parameters.keys() {
        assert!(
            descriptor.has_parameter(name, assets::ParameterKind::Int),
            "unknown int parameter: {name}"
        );
    }
    for name in state.float_parameters.keys() {
        assert!(
            descriptor.has_parameter(name, assets::ParameterKind::Float),
            "unknown float parameter: {name}"
        );
    }
}

pub(crate) fn renderer_slots(
    kind: &GameObjectKind,
    catalog: &assets::FakeAssetCatalog,
) -> Option<usize> {
    match kind {
        GameObjectKind::Cube { .. }
        | GameObjectKind::Sphere { .. }
        | GameObjectKind::Capsule { .. }
        | GameObjectKind::Cylinder { .. }
        | GameObjectKind::Plane { .. }
        | GameObjectKind::Quad { .. } => Some(1),
        GameObjectKind::Prefab { address, .. } => catalog
            .prefab(address)
            .and_then(assets::FakePrefab::material_slots),
        _ => None,
    }
}

pub(crate) fn require_catalog_asset(catalog: &assets::FakeAssetCatalog, asset: &PreparedAsset) {
    let valid = match asset {
        PreparedAsset::Scene(address) => catalog.has_scene(address),
        PreparedAsset::Prefab(address) => catalog.prefab(address).is_some(),
        PreparedAsset::ParticleEffect(address) => catalog.has_particle_effect(address),
        PreparedAsset::Material(address) => catalog.has_material(address),
        PreparedAsset::Texture(address) => catalog.has_texture(address),
        PreparedAsset::Sprite(address) => catalog.has_sprite(address),
        PreparedAsset::VectorImage(address) => catalog.has_vector_image(address),
        PreparedAsset::RenderTexture(address) => catalog.has_render_texture(address),
        PreparedAsset::AudioClip(address) => catalog.has_audio_clip(address),
        PreparedAsset::Font(address) => catalog.has_font(address),
        PreparedAsset::UiFont(address) => catalog.has_ui_font(address),
        PreparedAsset::UnityFont(address) => catalog.has_unity_font(address),
    };
    assert!(valid, "unknown prepared asset: {asset:?}");
}

pub(crate) fn assert_prepared(prepared_assets: &[PreparedAsset], expected: PreparedAsset) {
    assert!(
        prepared_assets.iter().any(|asset| asset == &expected),
        "asset is not prepared: {expected:?}"
    );
}
