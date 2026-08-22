//! In-memory Battlement-controlled objects, scenes, and component state.

use std::collections::{BTreeMap, HashMap};

use battlement::{
    AnimatorState, CameraState, GameObject, GameObjectKind, ImageState, MaterialAddress,
    MaterialAssignment, ParentScene, PreparedAsset, Scene, SceneId, Snapshot, TextState, Vector3,
};

use crate::{assets, transform, world_validation};

/// A computed world-space transform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldTransform {
    /// World-space position.
    pub position: Vector3,
    /// World-space rotation.
    pub rotation: battlement::Quaternion,
    /// Componentwise world scale.
    pub scale: Vector3,
}

/// Logical state for one playing audio command.
#[derive(Clone, Debug, PartialEq)]
pub struct FakeAudio {
    pub(crate) address: battlement::AudioClipAddress,
    pub(crate) volume: f64,
    pub(crate) pitch: f64,
    pub(crate) looping: bool,
}

impl FakeAudio {
    pub(crate) fn new(
        address: battlement::AudioClipAddress,
        volume: f64,
        pitch: f64,
        looping: bool,
    ) -> Self {
        Self {
            address,
            volume,
            pitch,
            looping,
        }
    }

    /// Returns the prepared clip address.
    #[must_use]
    pub fn address(&self) -> &battlement::AudioClipAddress {
        &self.address
    }

    /// Returns the current logical volume.
    #[must_use]
    pub fn volume(&self) -> f64 {
        self.volume
    }

    /// Returns the requested playback pitch.
    #[must_use]
    pub fn pitch(&self) -> f64 {
        self.pitch
    }

    /// Returns whether this playback loops.
    #[must_use]
    pub fn is_looping(&self) -> bool {
        self.looping
    }
}

/// One Battlement-controlled GameObject and its current logical component state.
#[derive(Clone, Debug, PartialEq)]
pub struct FakeObject {
    id: battlement::ObjectId,
    parent_id: Option<battlement::ObjectId>,
    scene_id: Option<SceneId>,
    active_self: bool,
    active_in_hierarchy: bool,
    local_transform: battlement::LocalTransform,
    pointer_events: Vec<battlement::PointerEvent>,
    drag_mode: Option<battlement::DragMode>,
    pub(crate) kind: GameObjectKind,
    renderer_slots: Option<usize>,
    camera: Option<CameraState>,
    light: Option<battlement::LightState>,
    animator_descriptor: Option<assets::FakeAnimator>,
    particles_playing: Option<bool>,
    collider: bool,
    automatic_collider: bool,
    children: Vec<battlement::ObjectId>,
}

impl FakeObject {
    /// Returns this object's stable identity.
    #[must_use]
    pub fn id(&self) -> battlement::ObjectId {
        self.id
    }

    /// Returns the current parent, if any.
    #[must_use]
    pub fn parent_id(&self) -> Option<battlement::ObjectId> {
        self.parent_id
    }

    /// Returns the loaded scene containing this object, or `None` for persistent objects.
    #[must_use]
    pub fn scene_id(&self) -> Option<SceneId> {
        self.scene_id
    }

    /// Returns the object's `activeSelf` value.
    #[must_use]
    pub fn active_self(&self) -> bool {
        self.active_self
    }

    /// Returns whether the object is active through its complete hierarchy.
    #[must_use]
    pub fn active_in_hierarchy(&self) -> bool {
        self.active_in_hierarchy
    }

    /// Returns the current local transform.
    #[must_use]
    pub fn local_transform(&self) -> battlement::LocalTransform {
        self.local_transform
    }

    /// Returns the complete current protocol kind state.
    #[must_use]
    pub fn kind(&self) -> &GameObjectKind {
        &self.kind
    }

    /// Returns the current image component, when this is an image object.
    #[must_use]
    pub fn image(&self) -> Option<&ImageState> {
        match &self.kind {
            GameObjectKind::Image { image } => Some(image),
            _ => None,
        }
    }

    /// Returns the current text component, when this is a text object.
    #[must_use]
    pub fn text(&self) -> Option<&TextState> {
        match &self.kind {
            GameObjectKind::Text { text } => Some(text),
            _ => None,
        }
    }

    /// Returns the enabled pointer-event set in protocol order.
    #[must_use]
    pub fn pointer_events(&self) -> &[battlement::PointerEvent] {
        &self.pointer_events
    }

    /// Returns the object's configured drag behavior, when draggable.
    #[must_use]
    pub fn drag_mode(&self) -> Option<battlement::DragMode> {
        self.drag_mode
    }

    /// Returns the renderer slot count, when this object has a supported renderer.
    #[must_use]
    pub fn renderer_slot_count(&self) -> Option<usize> {
        self.renderer_slots
    }

    /// Returns the material assigned to one renderer slot.
    #[must_use]
    pub fn material(&self, slot: u32) -> Option<&MaterialAddress> {
        materials(&self.kind).and_then(|values| {
            values
                .iter()
                .find(|assignment| assignment.slot == slot)
                .map(|assignment| &assignment.address)
        })
    }

    /// Returns the current logical camera component, if present.
    #[must_use]
    pub fn camera(&self) -> Option<&CameraState> {
        match &self.kind {
            GameObjectKind::Camera { camera } => Some(camera),
            _ => self.camera.as_ref(),
        }
    }

    /// Returns the current logical light component, if present.
    #[must_use]
    pub fn light(&self) -> Option<&battlement::LightState> {
        match &self.kind {
            GameObjectKind::Light { light } => Some(light),
            _ => self.light.as_ref(),
        }
    }

    /// Returns the current stable Animator state, if present.
    #[must_use]
    pub fn animator(&self) -> Option<&AnimatorState> {
        match &self.kind {
            GameObjectKind::Prefab { animator, .. } => animator.as_ref(),
            _ => None,
        }
    }

    /// Returns the logical particle-playing state, if particle systems were declared.
    #[must_use]
    pub fn particles_playing(&self) -> Option<bool> {
        self.particles_playing
    }
}

/// The current in-memory Battlement world.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FakeWorld {
    scenes: BTreeMap<SceneId, Scene>,
    primary_scene_id: Option<SceneId>,
    prepared_assets: Vec<PreparedAsset>,
    objects: HashMap<battlement::ObjectId, FakeObject>,
    object_order: Vec<battlement::ObjectId>,
    input_camera_id: Option<battlement::ObjectId>,
    uses_main_camera: bool,
    input_enabled: bool,
    global_keys: Vec<battlement::KeyCode>,
    audio: HashMap<battlement::CommandId, FakeAudio>,
}

impl FakeWorld {
    /// Iterates over objects in snapshot and creation order.
    pub fn objects(&self) -> impl Iterator<Item = &FakeObject> {
        self.object_order.iter().map(|id| {
            self.objects
                .get(id)
                .expect("fake world object order contained an unknown object")
        })
    }

    /// Returns the number of objects in the current world.
    #[must_use]
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    /// Iterates over image objects and their image state.
    pub fn images(&self) -> impl Iterator<Item = (&FakeObject, &ImageState)> {
        self.objects()
            .filter_map(|object| object.image().map(|image| (object, image)))
    }

    /// Iterates over text objects and their text state.
    pub fn texts(&self) -> impl Iterator<Item = (&FakeObject, &TextState)> {
        self.objects()
            .filter_map(|object| object.text().map(|text| (object, text)))
    }

    /// Looks up an object by ID.
    #[must_use]
    pub fn object(&self, id: battlement::ObjectId) -> Option<&FakeObject> {
        self.objects.get(&id)
    }

    /// Iterates over an object's direct children, or returns `None` for an unknown object.
    pub fn children(&self, id: battlement::ObjectId) -> Option<impl Iterator<Item = &FakeObject>> {
        let object = self.objects.get(&id)?;
        Some(object.children.iter().map(|child_id| {
            self.objects
                .get(child_id)
                .expect("fake hierarchy child disappeared")
        }))
    }

    /// Computes an object's world transform, panicking when it is unknown.
    #[must_use]
    pub fn world_transform(&self, id: battlement::ObjectId) -> WorldTransform {
        let object = self
            .objects
            .get(&id)
            .unwrap_or_else(|| panic!("unknown object in world transform: {id}"));
        let local = object.local_transform;
        object.parent_id.map_or(
            WorldTransform {
                position: local.position,
                rotation: transform::normalize(local.rotation),
                scale: local.scale,
            },
            |parent_id| transform::compose(self.world_transform(parent_id), local),
        )
    }

    /// Looks up a loaded scene by ID.
    #[must_use]
    pub fn scene(&self, id: SceneId) -> Option<&Scene> {
        self.scenes.get(&id)
    }

    /// Returns the currently selected primary scene.
    #[must_use]
    pub fn primary_scene_id(&self) -> SceneId {
        self.primary_scene_id
            .unwrap_or_else(|| panic!("fake world has no primary scene"))
    }

    /// Looks up a logical audio playback by its play-command ID.
    #[must_use]
    pub fn audio(&self, play_command_id: battlement::CommandId) -> Option<&FakeAudio> {
        self.audio.get(&play_command_id)
    }

    /// Iterates over loaded scenes in stable scene-ID order.
    pub fn scenes(&self) -> impl Iterator<Item = &Scene> {
        self.scenes.values()
    }

    /// Returns the complete prepared-address list.
    #[must_use]
    pub fn prepared_assets(&self) -> &[PreparedAsset] {
        &self.prepared_assets
    }

    /// Returns whether an asset belongs to the complete prepared set.
    #[must_use]
    pub fn is_prepared(&self, asset: &PreparedAsset) -> bool {
        self.prepared_assets.iter().any(|value| value == asset)
    }

    /// Returns whether pointer and keyboard input is enabled.
    #[must_use]
    pub fn input_enabled(&self) -> bool {
        self.input_enabled
    }

    /// Returns the selected Battlement input-camera object ID, if one is selected.
    #[must_use]
    pub fn input_camera_id(&self) -> Option<battlement::ObjectId> {
        self.input_camera_id
    }

    /// Returns whether the snapshot selected Unity's main camera.
    #[must_use]
    pub fn uses_main_camera(&self) -> bool {
        self.uses_main_camera
    }

    /// Returns the enabled global physical-key set.
    #[must_use]
    pub fn global_keys(&self) -> &[battlement::KeyCode] {
        &self.global_keys
    }

    pub(crate) fn replace_snapshot(
        &mut self,
        snapshot: Snapshot,
        catalog: &assets::FakeAssetCatalog,
    ) {
        let primary_scene_id = snapshot
            .primary_scene_id
            .or_else(|| snapshot.scenes.first().map(|scene| scene.scene_id))
            .expect("validated snapshot has no primary scene");
        for asset in &snapshot.prepared_assets {
            world_validation::require_catalog_asset(catalog, asset);
        }
        for scene in &snapshot.scenes {
            world_validation::assert_prepared(
                &snapshot.prepared_assets,
                PreparedAsset::Scene(scene.address.clone()),
            );
            assert!(
                catalog.has_scene(&scene.address),
                "unknown scene asset: {}",
                scene.address
            );
        }

        let mut next = Self {
            scenes: snapshot
                .scenes
                .into_iter()
                .map(|scene| (scene.scene_id, scene))
                .collect(),
            primary_scene_id: Some(primary_scene_id),
            prepared_assets: snapshot.prepared_assets,
            objects: HashMap::with_capacity(snapshot.objects.len()),
            object_order: Vec::with_capacity(snapshot.objects.len()),
            input_camera_id: snapshot.input_camera_id,
            uses_main_camera: snapshot.input_camera_id.is_none(),
            input_enabled: !snapshot.input_disabled,
            global_keys: dedupe(snapshot.global_keys),
            audio: HashMap::new(),
        };

        for object in snapshot.objects {
            let fake = FakeObject::from_game_object(
                object,
                primary_scene_id,
                catalog,
                &next.prepared_assets,
            );
            next.object_order.push(fake.id);
            assert!(
                next.objects.insert(fake.id, fake).is_none(),
                "duplicate object"
            );
        }
        next.link_children();
        next.recompute_active_states();
        if let Some(input_camera_id) = next.input_camera_id {
            assert!(
                next.object(input_camera_id).is_some(),
                "missing input camera"
            );
            assert!(
                next.object(input_camera_id)
                    .is_some_and(FakeObject::active_in_hierarchy),
                "input camera must be active"
            );
            assert!(
                next.object(input_camera_id)
                    .and_then(FakeObject::camera)
                    .is_some_and(|camera| camera.enabled),
                "input camera must be enabled"
            );
        }
        *self = next;
    }

    pub(crate) fn replace_prepared_assets(
        &mut self,
        values: Vec<PreparedAsset>,
        catalog: &assets::FakeAssetCatalog,
    ) {
        for asset in &values {
            world_validation::require_catalog_asset(catalog, asset);
        }
        self.prepared_assets = values;
    }

    pub(crate) fn load_scene(
        &mut self,
        scene_id: SceneId,
        address: battlement::SceneAddress,
        make_primary: bool,
        catalog: &assets::FakeAssetCatalog,
    ) {
        world_validation::assert_prepared(
            &self.prepared_assets,
            PreparedAsset::Scene(address.clone()),
        );
        assert!(
            catalog.has_scene(&address),
            "unknown scene asset: {address}"
        );
        assert!(
            !self.scenes.contains_key(&scene_id),
            "duplicate scene: {scene_id}"
        );
        assert!(
            !self.scenes.values().any(|scene| scene.address == address),
            "scene address already loaded: {address}"
        );
        self.scenes.insert(scene_id, Scene::new(scene_id, address));
        if make_primary {
            self.primary_scene_id = Some(scene_id);
        }
    }

    pub(crate) fn unload_scene(&mut self, scene_id: SceneId) {
        assert!(
            self.scenes.contains_key(&scene_id),
            "unknown scene: {scene_id}"
        );
        assert!(
            self.primary_scene_id != Some(scene_id),
            "cannot unload primary scene: {scene_id}"
        );
        let removed: Vec<_> = self
            .objects
            .values()
            .filter(|object| object.scene_id == Some(scene_id))
            .map(FakeObject::id)
            .collect();
        for id in removed {
            self.remove_object(id);
        }
        self.scenes.remove(&scene_id);
        if self
            .input_camera_id
            .is_some_and(|id| self.object(id).is_none())
        {
            self.input_camera_id = None;
        }
        self.recompute_active_states();
    }

    pub(crate) fn set_primary_scene(&mut self, scene_id: SceneId) {
        assert!(
            self.scenes.contains_key(&scene_id),
            "unknown scene: {scene_id}"
        );
        self.primary_scene_id = Some(scene_id);
    }

    pub(crate) fn create_object(&mut self, object: GameObject, catalog: &assets::FakeAssetCatalog) {
        assert!(
            !self.objects.contains_key(&object.object_id),
            "duplicate object: {}",
            object.object_id
        );
        let primary_scene_id = self.primary_scene_id();
        let requested_scene = match object.parent_scene {
            ParentScene::PrimaryScene => Some(primary_scene_id),
            ParentScene::Scene(scene_id) => {
                assert!(
                    self.scenes.contains_key(&scene_id),
                    "unknown scene: {scene_id}"
                );
                Some(scene_id)
            }
            ParentScene::Persistent => None,
        };
        let fake =
            FakeObject::from_game_object(object, primary_scene_id, catalog, &self.prepared_assets);
        assert!(
            fake.scene_id == requested_scene,
            "object placement could not be resolved"
        );
        if let Some(parent_id) = fake.parent_id {
            self.require_same_placement(fake.scene_id, parent_id);
        }
        let parent_id = fake.parent_id;
        let id = fake.id;
        self.objects.insert(id, fake);
        self.object_order.push(id);
        if let Some(parent_id) = parent_id {
            self.objects
                .get_mut(&parent_id)
                .expect("validated parent disappeared")
                .children
                .push(id);
        }
        self.recompute_active_states();
    }

    pub(crate) fn destroy_object(&mut self, id: battlement::ObjectId) {
        self.require_object(id);
        let descendants = self.descendants(id);
        let removes_input_camera = self
            .input_camera_id
            .is_some_and(|camera| camera == id || descendants.contains(&camera));
        for child_id in descendants.into_iter().rev() {
            self.remove_object(child_id);
        }
        self.remove_object(id);
        if removes_input_camera {
            self.input_camera_id = None;
        }
        self.recompute_active_states();
    }

    pub(crate) fn set_active(&mut self, id: battlement::ObjectId, active: bool) {
        self.require_object_mut(id).active_self = active;
        self.recompute_active_states();
    }

    pub(crate) fn reparent(
        &mut self,
        id: battlement::ObjectId,
        parent_id: Option<battlement::ObjectId>,
        world_position_stays: bool,
    ) {
        self.validate_reparent(id, parent_id);
        let world = world_position_stays.then(|| self.world_transform(id));
        let old_parent = self.require_object(id).parent_id;
        if let Some(old_parent) = old_parent {
            self.objects
                .get_mut(&old_parent)
                .expect("old parent disappeared")
                .children
                .retain(|child| *child != id);
        }
        self.require_object_mut(id).parent_id = parent_id;
        if let Some(parent_id) = parent_id {
            self.objects
                .get_mut(&parent_id)
                .expect("new parent disappeared")
                .children
                .push(id);
        }
        if let Some(world) = world {
            let parent_world = parent_id.map(|value| self.world_transform(value));
            self.require_object_mut(id).local_transform = transform::relative(parent_world, world);
        }
        self.recompute_active_states();
    }

    pub(crate) fn validate_reparent(
        &self,
        id: battlement::ObjectId,
        parent_id: Option<battlement::ObjectId>,
    ) {
        let object = self.require_object(id);
        let Some(parent_id) = parent_id else {
            return;
        };
        let parent = self.require_object(parent_id);
        assert!(id != parent_id, "object cannot parent itself: {id}");
        assert!(
            !self.is_descendant(parent_id, id),
            "object cannot be parented beneath its descendant: {id}"
        );
        assert!(
            object.scene_id == parent.scene_id,
            "object and parent must share placement: {id}"
        );
    }

    pub(crate) fn set_local_position(&mut self, id: battlement::ObjectId, value: Vector3) {
        self.require_object_mut(id).local_transform.position = value;
    }

    pub(crate) fn set_world_position(&mut self, id: battlement::ObjectId, value: Vector3) {
        let world = self.world_transform(id);
        let parent = self
            .require_object(id)
            .parent_id
            .map(|parent| self.world_transform(parent));
        self.require_object_mut(id).local_transform = transform::relative(
            parent,
            WorldTransform {
                position: value,
                ..world
            },
        );
    }

    pub(crate) fn set_local_rotation(
        &mut self,
        id: battlement::ObjectId,
        value: battlement::Quaternion,
    ) {
        self.require_object_mut(id).local_transform.rotation = transform::normalize(value);
    }

    pub(crate) fn set_world_rotation(
        &mut self,
        id: battlement::ObjectId,
        value: battlement::Quaternion,
    ) {
        let world = self.world_transform(id);
        let parent = self
            .require_object(id)
            .parent_id
            .map(|parent| self.world_transform(parent));
        self.require_object_mut(id).local_transform = transform::relative(
            parent,
            WorldTransform {
                rotation: transform::normalize(value),
                ..world
            },
        );
    }

    pub(crate) fn set_local_scale(&mut self, id: battlement::ObjectId, value: Vector3) {
        self.require_object_mut(id).local_transform.scale = value;
    }

    pub(crate) fn object_mut(&mut self, id: battlement::ObjectId) -> &mut FakeObject {
        self.require_object_mut(id)
    }

    pub(crate) fn camera_mut(&mut self, id: battlement::ObjectId) -> &mut CameraState {
        let object = self.require_object_mut(id);
        match &mut object.kind {
            GameObjectKind::Camera { camera } => camera,
            _ => object
                .camera
                .as_mut()
                .unwrap_or_else(|| panic!("object has no camera component: {id}")),
        }
    }

    pub(crate) fn set_camera_enabled(&mut self, id: battlement::ObjectId, enabled: bool) {
        self.camera_mut(id).enabled = enabled;
        if !enabled && self.input_camera_id == Some(id) {
            self.input_camera_id = None;
            self.uses_main_camera = false;
        }
    }

    pub(crate) fn light_mut(&mut self, id: battlement::ObjectId) -> &mut battlement::LightState {
        let object = self.require_object_mut(id);
        match &mut object.kind {
            GameObjectKind::Light { light } => light,
            _ => object
                .light
                .as_mut()
                .unwrap_or_else(|| panic!("object has no light component: {id}")),
        }
    }

    pub(crate) fn ensure_animator_mut(&mut self, id: battlement::ObjectId) -> &mut AnimatorState {
        let initial = self
            .animator_descriptor(id)
            .first_state()
            .unwrap_or((0, String::new()));
        let object = self.object_mut(id);
        match &mut object.kind {
            GameObjectKind::Prefab { animator, .. } => {
                animator.get_or_insert_with(|| AnimatorState {
                    state: initial.1,
                    layer: initial.0,
                    normalized_start_time: 0.0,
                    bool_parameters: Default::default(),
                    int_parameters: Default::default(),
                    float_parameters: Default::default(),
                    speed: 1.0,
                })
            }
            _ => panic!("object has no Animator component: {id}"),
        }
    }

    pub(crate) fn animator_descriptor(&self, id: battlement::ObjectId) -> &assets::FakeAnimator {
        self.require_object(id)
            .animator_descriptor
            .as_ref()
            .unwrap_or_else(|| panic!("object has no Animator component: {id}"))
    }

    pub(crate) fn particles_mut(&mut self, id: battlement::ObjectId) -> &mut bool {
        self.require_object_mut(id)
            .particles_playing
            .as_mut()
            .unwrap_or_else(|| panic!("object has no particle systems: {id}"))
    }

    pub(crate) fn descendant_ids(&self, id: battlement::ObjectId) -> Vec<battlement::ObjectId> {
        self.require_object(id);
        self.descendants(id)
    }

    pub(crate) fn set_input_enabled(&mut self, enabled: bool) {
        self.input_enabled = enabled;
    }

    pub(crate) fn set_input_camera(&mut self, id: battlement::ObjectId) {
        let object = self.require_object(id);
        assert!(
            object.active_in_hierarchy,
            "input camera must be active: {id}"
        );
        assert!(
            object.camera().is_some_and(|camera| camera.enabled),
            "input camera must be enabled: {id}"
        );
        self.input_camera_id = Some(id);
        self.uses_main_camera = false;
    }

    pub(crate) fn set_pointer_events(
        &mut self,
        id: battlement::ObjectId,
        events: Vec<battlement::PointerEvent>,
    ) {
        let object = self.require_object_mut(id);
        object.pointer_events = events;
        if object.automatic_collider {
            object.collider = !object.pointer_events.is_empty() || object.drag_mode.is_some();
        }
    }

    pub(crate) fn set_global_keys(&mut self, keys: Vec<battlement::KeyCode>) {
        self.global_keys = keys;
    }

    pub(crate) fn audio_play(&mut self, command_id: battlement::CommandId, audio: FakeAudio) {
        assert!(
            self.audio.insert(command_id, audio).is_none(),
            "duplicate audio command"
        );
    }

    pub(crate) fn audio_mut(&mut self, command_id: battlement::CommandId) -> &mut FakeAudio {
        self.audio
            .get_mut(&command_id)
            .unwrap_or_else(|| panic!("unknown audio command: {command_id}"))
    }

    pub(crate) fn audio_remove(&mut self, command_id: battlement::CommandId) {
        assert!(
            self.audio.remove(&command_id).is_some(),
            "unknown audio command: {command_id}"
        );
    }

    pub(crate) fn prepared(&self, asset: &PreparedAsset) -> bool {
        self.is_prepared(asset)
    }

    pub(crate) fn has_collider(&self, id: battlement::ObjectId) -> bool {
        self.require_object(id).collider
    }

    pub(crate) fn require_object(&self, id: battlement::ObjectId) -> &FakeObject {
        self.object(id)
            .unwrap_or_else(|| panic!("unknown object: {id}"))
    }

    fn require_object_mut(&mut self, id: battlement::ObjectId) -> &mut FakeObject {
        self.objects
            .get_mut(&id)
            .unwrap_or_else(|| panic!("unknown object: {id}"))
    }

    fn remove_object(&mut self, id: battlement::ObjectId) {
        let object = self
            .objects
            .remove(&id)
            .unwrap_or_else(|| panic!("unknown object: {id}"));
        self.object_order.retain(|value| *value != id);
        if let Some(parent_id) = object.parent_id {
            if let Some(parent) = self.objects.get_mut(&parent_id) {
                parent.children.retain(|child| *child != id);
            }
        }
        for child_id in object.children {
            if let Some(child) = self.objects.get_mut(&child_id) {
                child.parent_id = None;
            }
        }
    }

    fn descendants(&self, id: battlement::ObjectId) -> Vec<battlement::ObjectId> {
        let mut result = Vec::new();
        let mut pending = self.require_object(id).children.clone();
        while let Some(current) = pending.pop() {
            result.push(current);
            pending.extend(self.require_object(current).children.iter().copied());
        }
        result
    }

    fn is_descendant(
        &self,
        candidate: battlement::ObjectId,
        ancestor: battlement::ObjectId,
    ) -> bool {
        let mut current = Some(candidate);
        while let Some(id) = current {
            if id == ancestor {
                return true;
            }
            current = self.require_object(id).parent_id;
        }
        false
    }

    fn require_same_placement(&self, scene_id: Option<SceneId>, parent_id: battlement::ObjectId) {
        assert!(
            self.require_object(parent_id).scene_id == scene_id,
            "object and parent must share placement"
        );
    }

    fn link_children(&mut self) {
        let links: Vec<_> = self
            .object_order
            .iter()
            .filter_map(|id| {
                self.objects
                    .get(id)
                    .and_then(|object| object.parent_id.map(|parent| (parent, object.id)))
            })
            .collect();
        for (parent, child) in links {
            self.objects
                .get_mut(&parent)
                .expect("validated parent disappeared")
                .children
                .push(child);
        }
    }

    fn recompute_active_states(&mut self) {
        for id in self.object_order.clone() {
            let active = self.active_for(id);
            self.objects
                .get_mut(&id)
                .expect("active object disappeared")
                .active_in_hierarchy = active;
        }
    }

    fn active_for(&self, id: battlement::ObjectId) -> bool {
        let object = self.require_object(id);
        object.active_self
            && object
                .parent_id
                .is_none_or(|parent| self.active_for(parent))
    }
}

impl FakeObject {
    fn from_game_object(
        object: GameObject,
        primary_scene_id: SceneId,
        catalog: &assets::FakeAssetCatalog,
        prepared_assets: &[PreparedAsset],
    ) -> Self {
        world_validation::validate_object_assets(&object, catalog, prepared_assets);
        let scene_id = match object.parent_scene {
            ParentScene::PrimaryScene => Some(primary_scene_id),
            ParentScene::Scene(scene_id) => Some(scene_id),
            ParentScene::Persistent => None,
        };
        let renderer_slots = world_validation::renderer_slots(&object.kind, catalog);
        let (camera, light, animator_descriptor, particles_playing, collider) = match &object.kind {
            GameObjectKind::Camera { camera } => (Some(*camera), None, None, None, false),
            GameObjectKind::Light { light } => (None, Some(*light), None, None, false),
            GameObjectKind::Prefab { address, .. } => {
                let prefab = catalog.prefab(address).expect("unknown prefab asset");
                (
                    prefab.camera(),
                    prefab.light(),
                    prefab.animator().cloned(),
                    prefab.particle_systems().then_some(false),
                    prefab.pointer_collider(),
                )
            }
            GameObjectKind::Cube { .. }
            | GameObjectKind::Sphere { .. }
            | GameObjectKind::Capsule { .. }
            | GameObjectKind::Cylinder { .. }
            | GameObjectKind::Plane { .. }
            | GameObjectKind::Quad { .. }
            | GameObjectKind::Image { .. } => (None, None, None, None, true),
            GameObjectKind::Empty | GameObjectKind::Text { .. } => (None, None, None, None, false),
        };
        let automatic_collider = matches!(
            object.kind,
            GameObjectKind::Cube { .. }
                | GameObjectKind::Sphere { .. }
                | GameObjectKind::Capsule { .. }
                | GameObjectKind::Cylinder { .. }
                | GameObjectKind::Plane { .. }
                | GameObjectKind::Quad { .. }
                | GameObjectKind::Image { .. }
        );
        let collider = collider
            && (!automatic_collider
                || !object.pointer_events.is_empty()
                || object.drag_mode.is_some());
        let mut local_transform = object.local_transform;
        local_transform.rotation = crate::transform::normalize(local_transform.rotation);
        Self {
            id: object.object_id,
            parent_id: object.parent_id,
            scene_id,
            active_self: object.active,
            active_in_hierarchy: object.active,
            local_transform,
            pointer_events: object.pointer_events,
            drag_mode: object.drag_mode,
            kind: object.kind,
            renderer_slots,
            camera,
            light,
            animator_descriptor,
            particles_playing,
            collider,
            automatic_collider,
            children: Vec::new(),
        }
    }
}

fn dedupe<T>(values: Vec<T>) -> Vec<T>
where
    T: Copy + Eq + std::hash::Hash,
{
    let mut seen = std::collections::HashSet::with_capacity(values.len());
    values
        .into_iter()
        .filter(|value| seen.insert(*value))
        .collect()
}

fn materials(kind: &GameObjectKind) -> Option<&[MaterialAssignment]> {
    match kind {
        GameObjectKind::Cube { materials }
        | GameObjectKind::Sphere { materials }
        | GameObjectKind::Capsule { materials }
        | GameObjectKind::Cylinder { materials }
        | GameObjectKind::Plane { materials }
        | GameObjectKind::Quad { materials }
        | GameObjectKind::Prefab { materials, .. } => Some(materials),
        _ => None,
    }
}

pub(crate) fn materials_mut(kind: &mut GameObjectKind) -> Option<&mut Vec<MaterialAssignment>> {
    match kind {
        GameObjectKind::Cube { materials }
        | GameObjectKind::Sphere { materials }
        | GameObjectKind::Capsule { materials }
        | GameObjectKind::Cylinder { materials }
        | GameObjectKind::Plane { materials }
        | GameObjectKind::Quad { materials }
        | GameObjectKind::Prefab { materials, .. } => Some(materials),
        _ => None,
    }
}
