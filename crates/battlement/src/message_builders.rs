//! Fluent configuration methods for protocol messages with constructor defaults.

use crate::{
    ActionId, Batch, BatchStart, Connect, ControllerInputSettings, GameObject, GameObjectKind,
    PanelInputConfiguration, PanelScaleMode, PanelSettings, ParallelCommandGroup, ParentScene,
    PhysicalKey, PreparedAsset, Response, ResponseMessage, Scene, SceneId, Snapshot, UiDocument,
    UiDocumentState,
};

impl Connect {
    /// Replaces the custom command types and returns the updated message.
    #[must_use]
    pub fn custom_command_types(mut self, values: impl IntoIterator<Item = String>) -> Self {
        self.custom_command_types = values.into_iter().collect();
        self
    }

    /// Sets the persistent-data path and returns the updated message.
    #[must_use]
    pub fn persistent_data_path(mut self, value: impl Into<String>) -> Self {
        self.persistent_data_path = Some(value.into());
        self
    }

    /// Sets the StreamingAssets path and returns the updated message.
    #[must_use]
    pub fn streaming_assets_path(mut self, value: impl Into<String>) -> Self {
        self.streaming_assets_path = Some(value.into());
        self
    }
}

impl<C> Response<C> {
    /// Replaces the response messages and returns the updated response.
    #[must_use]
    pub fn messages(mut self, values: impl IntoIterator<Item = ResponseMessage<C>>) -> Self {
        self.messages = values.into_iter().collect();
        self
    }
}

impl Snapshot {
    /// Uses Unity's enabled, active camera tagged `MainCamera` for input and billboards.
    #[must_use]
    pub fn main_camera(mut self) -> Self {
        self.input_camera_id = None;
        self
    }

    /// Sets the primary scene and returns the updated snapshot.
    #[must_use]
    pub fn primary_scene_id(mut self, value: SceneId) -> Self {
        self.primary_scene_id = Some(value);
        self
    }

    /// Sets whether input starts disabled and returns the updated snapshot.
    #[must_use]
    pub fn input_disabled(mut self, value: bool) -> Self {
        self.input_disabled = value;
        self
    }

    /// Replaces process-wide world-space panel input settings.
    #[must_use]
    pub fn panel_input_configuration(mut self, value: PanelInputConfiguration) -> Self {
        self.panel_input_configuration = value;
        self
    }

    /// Replaces the prepared assets and returns the updated snapshot.
    #[must_use]
    pub fn prepared_assets(mut self, values: impl IntoIterator<Item = PreparedAsset>) -> Self {
        self.prepared_assets = values.into_iter().collect();
        self
    }

    /// Replaces the loaded scenes and returns the updated snapshot.
    #[must_use]
    pub fn scenes(mut self, values: impl IntoIterator<Item = Scene>) -> Self {
        self.scenes = values.into_iter().collect();
        self
    }

    /// Replaces the game objects and returns the updated snapshot.
    #[must_use]
    pub fn objects(mut self, values: impl IntoIterator<Item = GameObject>) -> Self {
        self.objects = values.into_iter().collect();
        self
    }

    /// Inserts a persistent screen-space UI document with constant-pixel scaling.
    #[must_use]
    pub fn ui_document(self, document: UiDocument) -> Self {
        self.ui_document_in(document, ParentScene::Persistent)
    }

    /// Inserts a screen-space UI document with constant-pixel scaling.
    #[must_use]
    pub fn ui_document_in(self, document: UiDocument, parent_scene: ParentScene) -> Self {
        self.ui_document_with(document, parent_scene, |state| state)
    }

    /// Inserts a UI document and configures its matching host GameObject.
    ///
    /// The host and visual-root identities come from `document`. The
    /// configuration function can customize rendering and placement without
    /// receiving or repeating either identity.
    #[must_use]
    pub fn ui_document_with<F>(
        mut self,
        document: UiDocument,
        parent_scene: ParentScene,
        configure: F,
    ) -> Self
    where
        F: FnOnce(UiDocumentState) -> UiDocumentState,
    {
        let state = configure(
            UiDocumentState::new(document.root_id)
                .panel_settings(PanelSettings::new().scale_mode(PanelScaleMode::ConstantPixelSize)),
        );
        self.objects.push(
            GameObject::new(document.document_id, GameObjectKind::UiDocument(state))
                .parent_scene(parent_scene),
        );
        self.ui.push(document);
        self
    }

    /// Replaces the enabled global keys and returns the updated snapshot.
    #[must_use]
    pub fn global_keys(mut self, values: impl IntoIterator<Item = PhysicalKey>) -> Self {
        self.global_keys = values.into_iter().collect();
        self
    }

    /// Enables controller input with the supplied settings and returns the updated snapshot.
    #[must_use]
    pub fn controller_input(mut self, value: ControllerInputSettings) -> Self {
        self.controller_input = Some(value);
        self
    }
}

impl<C> Batch<C> {
    /// Sets the action that caused the batch and returns the updated batch.
    #[must_use]
    pub fn caused_by_action_id(mut self, value: ActionId) -> Self {
        self.caused_by_action_id = Some(value);
        self
    }

    /// Sets the batch start policy and returns the updated batch.
    #[must_use]
    pub fn start(mut self, value: BatchStart) -> Self {
        self.start = value;
        self
    }

    /// Replaces the command groups and returns the updated batch.
    #[must_use]
    pub fn groups(mut self, values: impl IntoIterator<Item = ParallelCommandGroup<C>>) -> Self {
        self.groups = values.into_iter().collect();
        self
    }
}

impl<C> ParallelCommandGroup<C> {
    /// Replaces the commands and returns the updated group.
    #[must_use]
    pub fn commands(mut self, values: impl IntoIterator<Item = C>) -> Self {
        self.commands = values.into_iter().collect();
        self
    }
}
