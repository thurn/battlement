use std::{cell::RefCell, collections::BTreeMap};

use battlement::{
  Command, CommandBody, ControllerButton, ControllerInputSettings, GlobalKeysPayload, PhysicalKey,
};
use reactant_core::{app_context::AppHandle, app_runtime::ApplicationContext, hooks};

use crate::game_app::ServicesContext;

/// Physical input needed by a mounted component, in addition to application defaults.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InputSubscription {
  /// Physical keyboard keys to deliver after native focus arbitration.
  pub keys: Vec<PhysicalKey>,
  /// Controller buttons to deliver after native focus arbitration.
  pub buttons: Vec<ControllerButton>,
  /// Enables the native D-pad and left-stick navigation path.
  pub navigation: bool,
}

/// Keeps physical input enabled while mounted, replacing it when bindings change.
/// Other mounted subscriptions and application defaults remain enabled.
pub fn use_input_subscription(subscription: InputSubscription) {
  let services = hooks::use_required_context::<ApplicationContext>()
    .value::<ServicesContext>()
    .expect("input subscriptions require a Reactant Application root");
  let coordinator = services
    .coordinator
    .upgrade()
    .expect("application runtime ended while rendering");
  let registry = coordinator.input_subscriptions.clone();
  let input = coordinator.input.clone();
  let app = hooks::use_required_context::<AppHandle>();
  let identity = hooks::use_id();
  let dependency = subscription.clone();
  hooks::use_effect(
    move || {
      input.reset_global_input();
      registry.register(identity.clone(), subscription, &app);
      move || {
        input.reset_global_input();
        registry.unregister(&identity, &app);
      }
    },
    dependency,
  );
}

#[derive(Default)]
pub(crate) struct InputSubscriptions {
  keys: RefCell<Vec<PhysicalKey>>,
  controller: RefCell<Option<ControllerInputSettings>>,
  mounted: RefCell<BTreeMap<String, InputSubscription>>,
}

impl InputSubscriptions {
  pub(crate) fn set_keys(&self, keys: Vec<PhysicalKey>) {
    let mut unique = Vec::new();
    for key in keys {
      if !unique.contains(&key) {
        unique.push(key);
      }
    }
    *self.keys.borrow_mut() = unique;
  }

  pub(crate) fn set_controller(&self, settings: ControllerInputSettings) {
    *self.controller.borrow_mut() = Some(settings);
  }

  fn register(&self, identity: String, subscription: InputSubscription, app: &AppHandle) {
    self.mounted.borrow_mut().insert(identity, subscription);
    self.publish(app);
  }

  fn unregister(&self, identity: &str, app: &AppHandle) {
    self.mounted.borrow_mut().remove(identity);
    self.publish(app);
  }

  fn publish(&self, app: &AppHandle) {
    let mut keys = self.keys.borrow().clone();
    let mut controller = self.controller.borrow().clone().unwrap_or_else(|| {
      let mut settings = ControllerInputSettings::new();
      settings.navigation_enabled = false;
      settings
    });
    for subscription in self.mounted.borrow().values() {
      for key in &subscription.keys {
        if !keys.contains(key) {
          keys.push(*key);
        }
      }
      for button in &subscription.buttons {
        if !controller.buttons.contains(button) {
          controller.buttons.push(*button);
        }
      }
      controller.navigation_enabled |= subscription.navigation;
    }
    app.send(Command::new_v4(CommandBody::InputSetGlobalKeys(
      GlobalKeysPayload { keys },
    )));
    app.send(Command::new_v4(CommandBody::InputSetController(controller)));
  }
}
