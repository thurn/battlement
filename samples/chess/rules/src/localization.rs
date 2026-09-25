//! Bundled language selection without remounting the application.

use reactant::{app_context, hooks, prelude::*};
use trox::{Bundle, Localizer};

use crate::settings::{self, Language};

pub struct LocalizationRoot {
  pub children: Children,
}

pub fn localizer(language: Language) -> Localizer {
  let source = Bundle::from_canonical_json(include_str!("../localization/bundles/en-US.trox.json"))
    .expect("valid English chess bundle");
  let target = match language {
    Language::English => source.clone(),
    Language::French => {
      Bundle::from_canonical_json(include_str!("../localization/bundles/fr.trox.json"))
        .expect("valid French chess bundle")
    }
  };
  Localizer::new(target, source).expect("compatible chess bundles")
}

impl Component for LocalizationRoot {
  fn render(&self) -> impl Render {
    let language = settings::use_settings().desired.language;
    let app = app_context::use_app();
    let (initialized, set_initialized) = hooks::use_state(language == Language::English);
    let applied = hooks::use_ref(Language::English);
    hooks::use_effect(
      move || {
        if applied.with(|value| *value) != language {
          app.set_localizer(self::localizer(language));
          applied.with_mut(|value| *value = language);
          set_initialized.set(true);
        }
      },
      language,
    );
    initialized.then(|| self.children.render())
  }
}
