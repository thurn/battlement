//! Test-owned descriptions of Unity Addressables assets.

use std::collections::{BTreeMap, BTreeSet};

use masonry::{
    AudioClipAddress, CameraState, FontAddress, LightState, MaterialAddress, PrefabAddress,
    SceneAddress, TextureAddress,
};

/// An immutable-after-sharing catalog of assets available to a fake client.
#[derive(Clone, Debug, Default)]
pub struct FakeAssetCatalog {
    addresses: BTreeSet<String>,
    scenes: BTreeSet<String>,
    prefabs: BTreeMap<String, FakePrefab>,
    particle_effects: BTreeSet<String>,
    materials: BTreeSet<String>,
    textures: BTreeSet<String>,
    audio_clips: BTreeSet<String>,
    fonts: BTreeSet<String>,
}

impl FakeAssetCatalog {
    /// Creates an empty asset catalog.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a content scene address.
    pub fn add_scene(&mut self, address: impl Into<SceneAddress>) {
        let address = address.into();
        self.insert_address(address.as_str());
        self.scenes.insert(address.into_string());
    }

    /// Registers a prefab address and its root capabilities.
    pub fn add_prefab(&mut self, address: impl Into<PrefabAddress>, value: FakePrefab) {
        let address = address.into();
        self.insert_address(address.as_str());
        let key = address.into_string();
        if self.prefabs.insert(key, value).is_some() {
            panic!("duplicate prefab declaration");
        }
    }

    /// Registers a temporary particle-effect prefab address.
    pub fn add_particle_effect(&mut self, address: impl Into<PrefabAddress>) {
        let address = address.into();
        self.insert_address(address.as_str());
        self.particle_effects.insert(address.into_string());
    }

    /// Registers a material address.
    pub fn add_material(&mut self, address: impl Into<MaterialAddress>) {
        let address = address.into();
        self.insert_address(address.as_str());
        self.materials.insert(address.into_string());
    }

    /// Registers a texture address.
    pub fn add_texture(&mut self, address: impl Into<TextureAddress>) {
        let address = address.into();
        self.insert_address(address.as_str());
        self.textures.insert(address.into_string());
    }

    /// Registers multiple texture addresses.
    pub fn add_textures<T>(&mut self, addresses: impl IntoIterator<Item = T>)
    where
        T: Into<TextureAddress>,
    {
        for address in addresses {
            self.add_texture(address);
        }
    }

    /// Registers an audio clip address.
    pub fn add_audio_clip(&mut self, address: impl Into<AudioClipAddress>) {
        let address = address.into();
        self.insert_address(address.as_str());
        self.audio_clips.insert(address.into_string());
    }

    /// Registers a TextMesh Pro font address.
    pub fn add_font(&mut self, address: impl Into<FontAddress>) {
        let address = address.into();
        self.insert_address(address.as_str());
        self.fonts.insert(address.into_string());
    }

    pub(crate) fn has_scene(&self, address: &SceneAddress) -> bool {
        self.scenes.contains(address.as_str())
    }

    pub(crate) fn prefab(&self, address: &PrefabAddress) -> Option<&FakePrefab> {
        self.prefabs.get(address.as_str())
    }

    pub(crate) fn has_particle_effect(&self, address: &PrefabAddress) -> bool {
        self.particle_effects.contains(address.as_str())
    }

    pub(crate) fn has_material(&self, address: &MaterialAddress) -> bool {
        self.materials.contains(address.as_str())
    }

    pub(crate) fn has_texture(&self, address: &TextureAddress) -> bool {
        self.textures.contains(address.as_str())
    }

    pub(crate) fn has_audio_clip(&self, address: &AudioClipAddress) -> bool {
        self.audio_clips.contains(address.as_str())
    }

    pub(crate) fn has_font(&self, address: &FontAddress) -> bool {
        self.fonts.contains(address.as_str())
    }

    fn insert_address(&mut self, address: &str) {
        if !self.addresses.insert(address.to_owned()) {
            panic!("duplicate asset declaration: {address}");
        }
    }
}

/// Root capabilities discovered from a prepared prefab in Unity.
#[derive(Clone, Debug, Default)]
pub struct FakePrefab {
    material_slots: Option<usize>,
    camera: Option<CameraState>,
    light: Option<LightState>,
    animator: Option<FakeAnimator>,
    particle_systems: Option<()>,
    pointer_collider: Option<()>,
}

impl FakePrefab {
    /// Creates a prefab with no supported root components.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a renderer with the requested positive material-slot count.
    #[must_use]
    pub fn with_material_slots(mut self, count: usize) -> Self {
        assert!(count > 0, "prefab material-slot count must be positive");
        assert!(
            self.material_slots.is_none(),
            "prefab renderer already declared"
        );
        self.material_slots = Some(count);
        self
    }

    /// Adds a root camera with its initial logical state.
    #[must_use]
    pub fn with_camera(mut self, initial: CameraState) -> Self {
        assert!(self.camera.is_none(), "prefab camera already declared");
        self.camera = Some(initial);
        self
    }

    /// Adds a root light with its initial logical state.
    #[must_use]
    pub fn with_light(mut self, initial: LightState) -> Self {
        assert!(self.light.is_none(), "prefab light already declared");
        self.light = Some(initial);
        self
    }

    /// Adds a root animator descriptor.
    #[must_use]
    pub fn with_animator(mut self, animator: FakeAnimator) -> Self {
        assert!(self.animator.is_none(), "prefab animator already declared");
        self.animator = Some(animator);
        self
    }

    /// Declares that the prefab hierarchy contains particle systems.
    #[must_use]
    pub fn with_particle_systems(mut self) -> Self {
        assert!(
            self.particle_systems.replace(()).is_none(),
            "prefab particle systems already declared"
        );
        self
    }

    /// Declares that the prefab hierarchy has a suitable pointer collider.
    #[must_use]
    pub fn with_pointer_collider(mut self) -> Self {
        assert!(
            self.pointer_collider.replace(()).is_none(),
            "prefab pointer collider already declared"
        );
        self
    }

    pub(crate) fn material_slots(&self) -> Option<usize> {
        self.material_slots
    }

    pub(crate) fn camera(&self) -> Option<CameraState> {
        self.camera
    }

    pub(crate) fn light(&self) -> Option<LightState> {
        self.light
    }

    pub(crate) fn animator(&self) -> Option<&FakeAnimator> {
        self.animator.as_ref()
    }

    pub(crate) fn particle_systems(&self) -> bool {
        self.particle_systems.is_some()
    }

    pub(crate) fn pointer_collider(&self) -> bool {
        self.pointer_collider.is_some()
    }
}

/// Names, layers, and parameter types exposed by a prefab's root Animator.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FakeAnimator {
    states: BTreeMap<u32, BTreeSet<String>>,
    bool_parameters: BTreeSet<String>,
    int_parameters: BTreeSet<String>,
    float_parameters: BTreeSet<String>,
    trigger_parameters: BTreeSet<String>,
}

impl FakeAnimator {
    /// Creates an animator with no states or parameters.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a state to one Animator layer.
    #[must_use]
    pub fn with_state(mut self, layer: u32, state: impl Into<String>) -> Self {
        let state = state.into();
        assert!(
            self.states.entry(layer).or_default().insert(state),
            "duplicate animator state"
        );
        self
    }

    /// Adds a boolean Animator parameter.
    #[must_use]
    pub fn with_bool_parameter(mut self, name: impl Into<String>) -> Self {
        self.add_parameter(name.into(), ParameterKind::Bool);
        self
    }

    /// Adds an integer Animator parameter.
    #[must_use]
    pub fn with_int_parameter(mut self, name: impl Into<String>) -> Self {
        self.add_parameter(name.into(), ParameterKind::Int);
        self
    }

    /// Adds a floating-point Animator parameter.
    #[must_use]
    pub fn with_float_parameter(mut self, name: impl Into<String>) -> Self {
        self.add_parameter(name.into(), ParameterKind::Float);
        self
    }

    /// Adds an Animator trigger parameter.
    #[must_use]
    pub fn with_trigger_parameter(mut self, name: impl Into<String>) -> Self {
        self.add_parameter(name.into(), ParameterKind::Trigger);
        self
    }

    pub(crate) fn has_state(&self, layer: u32, state: &str) -> bool {
        self.states
            .get(&layer)
            .is_some_and(|states| states.contains(state))
    }

    pub(crate) fn first_state(&self) -> Option<(u32, String)> {
        self.states
            .iter()
            .find_map(|(layer, states)| states.first().map(|state| (*layer, state.clone())))
    }

    pub(crate) fn has_parameter(&self, name: &str, kind: ParameterKind) -> bool {
        match kind {
            ParameterKind::Bool => self.bool_parameters.contains(name),
            ParameterKind::Int => self.int_parameters.contains(name),
            ParameterKind::Float => self.float_parameters.contains(name),
            ParameterKind::Trigger => self.trigger_parameters.contains(name),
        }
    }

    fn add_parameter(&mut self, name: String, kind: ParameterKind) {
        assert!(
            !self.has_any_parameter(&name),
            "duplicate animator parameter: {name}"
        );
        let inserted = match kind {
            ParameterKind::Bool => self.bool_parameters.insert(name),
            ParameterKind::Int => self.int_parameters.insert(name),
            ParameterKind::Float => self.float_parameters.insert(name),
            ParameterKind::Trigger => self.trigger_parameters.insert(name),
        };
        assert!(inserted, "duplicate animator parameter");
    }

    fn has_any_parameter(&self, name: &str) -> bool {
        self.bool_parameters.contains(name)
            || self.int_parameters.contains(name)
            || self.float_parameters.contains(name)
            || self.trigger_parameters.contains(name)
    }
}

#[derive(Clone, Copy)]
pub(crate) enum ParameterKind {
    Bool,
    Int,
    Float,
    Trigger,
}
