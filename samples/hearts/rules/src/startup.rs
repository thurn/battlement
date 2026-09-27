use std::sync::Arc;

use battlement::object_id;
use reactant::{PersistenceBackend, PersistentState, hooks, prelude::*};
use trox::ls;

use crate::{
  app::{self, Opponents},
  domain::HeartsState,
  match_ui,
  saved_game::{SavedMatch, SavedSettings},
};

#[derive(Clone, PartialEq)]
pub(crate) struct Saves {
  pub game: PersistentState<SavedMatch>,
  pub settings: PersistentState<SavedSettings>,
  pub home: hooks::StateSetter<Option<(HeartsState, bool)>>,
}

pub(crate) struct Startup {
  pub backend: Arc<dyn PersistenceBackend>,
  pub opponents: Opponents,
  pub new_match: fn() -> HeartsState,
}

struct Home {
  saves: Saves,
  new_match: fn() -> HeartsState,
}

impl Component for Startup {
  fn render(&self) -> impl Render {
    let game =
      reactant::use_persistent_state_with::<SavedMatch>("hearts-match.json", self.backend.clone());
    let settings = reactant::use_persistent_state_with::<SavedSettings>(
      "hearts-settings.json",
      self.backend.clone(),
    );
    let (playing, home) = hooks::use_state(None::<(HeartsState, bool)>);
    let saves = Saves {
      game,
      settings,
      home,
    };
    let content = if let Some((initial, fresh)) = playing {
      Node::new(app::persistent_root(
        initial,
        fresh,
        self.opponents,
        self.new_match,
      ))
    } else {
      Node::new(Home {
        saves: saves.clone(),
        new_match: self.new_match,
      })
    };
    ContextProvider::new().context(saves).child(content)
  }
}

impl Component for Home {
  fn render(&self) -> impl Render {
    let game = &self.saves.game;
    let settings = &self.saves.settings;
    let (confirm, set_confirm) = hooks::use_state(false);
    let panel = match_ui::panel();
    let busy = game.pending().is_some() || settings.pending().is_some();
    let loading = game.error().is_none() && !game.hydrated();
    let settings_loading = settings.error().is_none() && !settings.hydrated();
    let ready = !busy && !loading && !settings_loading;
    let settings_ok = settings.hydrated() && settings.error().is_none();
    let resume = self.saves.home.clone();
    let start = self.saves.home.clone();
    let new_match = self.new_match;
    let confirmation = set_confirm.clone();
    let retry = game.clone();
    let retry_settings = settings.clone();
    let defaults = settings.clone();
    let message = if loading || settings_loading {
      "Loading saved progress…"
    } else if busy {
      "Finishing saved progress…"
    } else {
      "Your next hand is waiting."
    };
    let game_error = game.error().map(|_| (
      Text::new(ls("Saved progress could not be read or saved. Retry, or explicitly replace it with a new game."))
        .style(match_ui::text_style(20.0)),
      Button::new(ls("Retry saved game")).disabled(busy)
        .style(match_ui::button_style(20.0)).on_press(move || retry.retry()),
    ));
    let settings_error = settings.error().map(|_| {
      (
        Text::new(ls(
          "Settings could not be read. Retry or replace them with defaults.",
        ))
        .style(match_ui::text_style(20.0)),
        Button::new(ls("Retry settings"))
          .disabled(busy)
          .style(match_ui::button_style(20.0))
          .on_press(move || retry_settings.retry()),
        Button::new(ls("Use default settings"))
          .disabled(busy)
          .style(match_ui::button_style(20.0))
          .on_press(move || defaults.update(SavedSettings::default())),
      )
    });
    let resume = game.value().cloned().map(|saved| {
      Button::new(ls("Continue"))
        .disabled(!ready || !settings_ok)
        .style(match_ui::button_style(20.0))
        .on_press(move || resume.set(Some((saved.state.clone(), false))))
    });
    let replacement = confirm.then(|| {
      (
        Text::new(ls(
          "Start a new game? This replaces any previous saved match.",
        ))
        .style(match_ui::text_style(20.0)),
        Button::new(ls("Start new game"))
          .disabled(!ready || !settings_ok)
          .style(match_ui::button_style(20.0))
          .on_press(move || start.set(Some((new_match(), true)))),
        Button::new(ls("Cancel"))
          .style(match_ui::button_style(20.0))
          .on_press(move || set_confirm.set(false)),
      )
    });
    View::new()
      .style(Style::new().width(100.pct()).height(100.pct()))
      .child((
        panel.child((
          Heading::new(ls("Hearts"), 1).style(Style::new().font_size(32.px())),
          View::new()
            .id(object_id!("6644ed66-12dc-4590-9af8-19d174a47082").into())
            .enabled(ready)
            .child(Text::new(ls(message)).style(match_ui::text_style(20.0))),
          game_error,
          settings_error,
          resume,
          Button::new(ls("New game"))
            .disabled(!ready || !settings_ok)
            .style(match_ui::button_style(20.0))
            .on_press(move || confirmation.set(true)),
          replacement,
        )),
        crate::persistence_fixture::FixtureControls,
      ))
  }
}
