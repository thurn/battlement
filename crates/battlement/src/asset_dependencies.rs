//! Typed asset discovery for snapshots and command delivery.

use std::collections::BTreeMap;

use crate::{
  BackgroundSource, CommandBody, GameObject, GameObjectKind, IconSource, ImageSource,
  PreparedAsset, Prop, Snapshot, Style, StyleValue, UiElement, UiNode, UiVisualElementProperties,
  VisualElementUpdate,
};

/// A deduplicated set of typed asset references, ordered by address.
#[derive(Default)]
pub struct AssetDependencies {
  assets: BTreeMap<String, PreparedAsset>,
}

impl AssetDependencies {
  /// Adds an asset, rejecting conflicting types for the same address.
  pub fn insert(&mut self, asset: PreparedAsset) {
    let key = self::address(&asset).to_owned();
    if let Some(existing) = self.assets.get(&key) {
      assert_eq!(
        existing, &asset,
        "asset address has conflicting kinds: {key}"
      );
    } else {
      self.assets.insert(key, asset);
    }
  }

  /// Returns the complete preparation set.
  pub fn assets(&self) -> Vec<PreparedAsset> {
    self.assets.values().cloned().collect()
  }

  /// Collects all declared and directly referenced snapshot assets.
  pub fn snapshot(&mut self, snapshot: &Snapshot) {
    for asset in &snapshot.prepared_assets {
      self.insert(asset.clone());
    }
    for scene in &snapshot.scenes {
      self.insert(PreparedAsset::Scene(scene.address.clone()));
    }
    for object in &snapshot.objects {
      self.object(object);
    }
    for document in &snapshot.ui {
      self.style(&document.element.style);
      if let Prop::Set(motion) = &document.element.motion {
        self.motion(motion);
      }
      for node in &document.children {
        self.node(node);
      }
    }
  }

  /// Collects assets that a core command can begin using.
  pub fn command(&mut self, command: &CommandBody) {
    self.motion_command(command);
    match command {
      CommandBody::SceneLoad(value) => self.insert(PreparedAsset::Scene(value.address.clone())),
      CommandBody::ObjectCreate(value) => self.object(&value.object),
      CommandBody::RendererSetMaterial(value) => {
        self.insert(PreparedAsset::Material(value.payload.address.clone()))
      }
      CommandBody::ImageSetTexture(value) => {
        self.insert(PreparedAsset::Texture(value.address.clone()))
      }
      CommandBody::TextSetFont(value) => {
        self.insert(PreparedAsset::TextMeshProFont(value.address.clone()))
      }
      CommandBody::ParticleSpawn(value) => {
        self.insert(PreparedAsset::ParticleEffect(value.address.clone()))
      }
      CommandBody::AudioPlay(value) => self.insert(PreparedAsset::AudioClip(value.address.clone())),
      CommandBody::AudioReplace(value) => {
        self.insert(PreparedAsset::AudioClip(value.address.clone()))
      }
      CommandBody::VisualElementCreate(value) => self.node(&value.node),
      CommandBody::VisualElementUpdate(value) => {
        if let VisualElementUpdate::Properties { element, .. } = value.as_ref() {
          self.element(element);
        }
      }
      CommandBody::AssetsReplaceSet(value) => {
        for asset in &value.assets {
          self.insert(asset.clone());
        }
      }
      _ => {}
    }
  }

  fn object(&mut self, object: &GameObject) {
    match &object.kind {
      GameObjectKind::Prefab {
        address, materials, ..
      } => {
        self.insert(PreparedAsset::Prefab(address.clone()));
        for material in materials {
          self.insert(PreparedAsset::Material(material.address.clone()));
        }
      }
      GameObjectKind::Cube { materials }
      | GameObjectKind::Sphere { materials }
      | GameObjectKind::Capsule { materials }
      | GameObjectKind::Cylinder { materials }
      | GameObjectKind::Plane { materials }
      | GameObjectKind::Quad { materials } => {
        for material in materials {
          self.insert(PreparedAsset::Material(material.address.clone()));
        }
      }
      GameObjectKind::Image { image } => self.insert(PreparedAsset::Texture(image.texture.clone())),
      GameObjectKind::Text { text } => {
        self.insert(PreparedAsset::TextMeshProFont(text.font.clone()))
      }
      GameObjectKind::UiDocument(document) => {
        if let Some(target) = &document.panel_settings.target_texture {
          self.insert(PreparedAsset::RenderTexture(target.clone()));
        }
      }
      _ => {}
    }
  }

  fn node(&mut self, node: &UiNode) {
    self.element(&node.element);
    for child in &node.children {
      self.node(child);
    }
  }

  fn element(&mut self, element: &UiElement) {
    match element {
      UiElement::Image(image) => {
        if let Prop::Set(source) = &image.source {
          self.insert(match source {
            ImageSource::Texture(value) => PreparedAsset::Texture(value.clone()),
            ImageSource::Sprite(value) => PreparedAsset::Sprite(value.clone()),
            ImageSource::VectorImage(value) => PreparedAsset::VectorImage(value.clone()),
            ImageSource::RenderTexture(value) => PreparedAsset::RenderTexture(value.clone()),
          });
        }
      }
      UiElement::Button(button) => {
        if let Prop::Set(source) = &button.icon {
          self.insert(match source {
            IconSource::Texture(value) => PreparedAsset::Texture(value.clone()),
            IconSource::Sprite(value) => PreparedAsset::Sprite(value.clone()),
            IconSource::VectorImage(value) => PreparedAsset::VectorImage(value.clone()),
            IconSource::RenderTexture(value) => PreparedAsset::RenderTexture(value.clone()),
          });
        }
      }
      _ => {}
    }
    self.style(&element.visual_element().style);
    if let Prop::Set(motion) = &element.visual_element().motion {
      self.motion(motion);
    }
    for style in crate::authored_private_part_styles(element) {
      self.style(style);
    }
  }

  pub(crate) fn style(&mut self, style: &Style) {
    if let Prop::Set(StyleValue::Value(value)) = &style.background_image {
      self.insert(match value {
        BackgroundSource::Texture(value) => PreparedAsset::Texture(value.clone()),
        BackgroundSource::Sprite(value) => PreparedAsset::Sprite(value.clone()),
        BackgroundSource::VectorImage(value) => PreparedAsset::VectorImage(value.clone()),
        BackgroundSource::RenderTexture(value) => PreparedAsset::RenderTexture(value.clone()),
      });
    }
    if let Prop::Set(StyleValue::Value(value)) = &style.cursor
      && let Some(address) = value.texture_address()
    {
      self.insert(PreparedAsset::Texture(address.clone()));
    }
    if let Prop::Set(StyleValue::Value(value)) = &style.unity_font_definition {
      self.insert(PreparedAsset::UiFont(value.clone()));
    }
    if let Prop::Set(StyleValue::Value(value)) = &style.unity_material {
      self.insert(PreparedAsset::Material(value.clone()));
    }
  }
}

/// Returns an asset's Addressables address independently of its kind.
pub fn address(asset: &PreparedAsset) -> &str {
  match asset {
    PreparedAsset::Scene(value) => value.as_str(),
    PreparedAsset::Prefab(value) | PreparedAsset::ParticleEffect(value) => value.as_str(),
    PreparedAsset::Material(value) => value.as_str(),
    PreparedAsset::Texture(value) => value.as_str(),
    PreparedAsset::Sprite(value) => value.as_str(),
    PreparedAsset::VectorImage(value) => value.as_str(),
    PreparedAsset::RenderTexture(value) => value.as_str(),
    PreparedAsset::TextMeshProFont(value) => value.as_str(),
    PreparedAsset::UiFont(value) => value.as_str(),
    PreparedAsset::AudioClip(value) => value.as_str(),
  }
}
