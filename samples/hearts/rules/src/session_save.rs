use battlement::{PickingMode, UiFontAddress, object_id};
use reactant::{GameStatus, hooks, overlay::Overlay, portal::PortalTarget, prelude::*};
use trox::ls;

use crate::{
  assets,
  controller::HeartsController,
  match_ui,
  saved_game::{SavedMatch, SavedSettings},
  startup::Saves,
};

pub(crate) struct SaveStatus {
  pub target: PortalTarget,
  pub exiting: bool,
  pub set_exiting: hooks::StateSetter<bool>,
}

pub(crate) fn use_autosave(game: &HeartsController, settings: SavedSettings, exiting: bool) {
  let saves = hooks::use_optional_context::<Saves>();
  let accepted = game.game.accepted();
  let version = accepted.version;
  let record = SavedMatch::capture(version, &accepted.state);
  let capture = record.clone();
  let storage = saves.clone();
  hooks::use_effect(
    move || {
      if let Some(saves) = storage {
        saves.game.update(capture);
      }
    },
    version,
  );
  let storage = saves.clone();
  hooks::use_effect(
    move || {
      if let Some(saves) = storage
        && saves.settings.desired() != Some(&settings)
      {
        saves.settings.update(settings);
      }
    },
    settings,
  );
  let ready = saves.as_ref().is_some_and(|saves| {
    let game_ready = saves.game.pending().is_none() && saves.game.value() == Some(&record);
    let settings_ready =
      saves.settings.pending().is_none() && saves.settings.value() == Some(&settings);
    game_ready && settings_ready && game.game.status() != GameStatus::Busy
  });
  hooks::use_effect(
    move || {
      if exiting
        && ready
        && let Some(saves) = saves
      {
        saves.home.set(None);
      }
    },
    (exiting, ready),
  );
}

impl Component for SaveStatus {
  fn render(&self) -> impl Render {
    let saves = hooks::use_optional_context::<Saves>();
    saves.map(|saves| {
      if self.exiting {
        return Node::new(ExitDialog {
          target: self.target.clone(),
          saves,
          set_exiting: self.set_exiting.clone(),
        });
      }
      let status = self::status(&saves);
      let ready = self::ready(&saves);
      let marker = View::new()
        .id(object_id!("6644ed66-12dc-4590-9af8-19d174a47081").into())
        .enabled(ready)
        .style(Style::new().width(1.px()).height(1.px()));
      let label = Text::new(ls(status)).style(
        match_ui::text_style(18.0)
          .padding(4.px())
          .border_radius(4.px())
          .background_color(Color::rgb(0.96, 0.92, 0.77)),
      );
      Node::new(
        View::new()
          .picking_mode(PickingMode::Ignore)
          .style(Style::new().width(100.pct()).height(100.pct()))
          .child(
            View::new()
              .style(
                Style::new()
                  .position(Position::Absolute)
                  .left(145.px())
                  .top(26.px())
                  .unity_font_definition(UiFontAddress::from(assets::hearts::fonts::CONTROL))
                  .color(Color::rgb(0.06, 0.12, 0.03)),
              )
              .child((marker, label)),
          ),
      )
    })
  }
}

struct ExitDialog {
  target: PortalTarget,
  saves: Saves,
  set_exiting: hooks::StateSetter<bool>,
}

impl Component for ExitDialog {
  fn render(&self) -> impl Render {
    let initial = reactant::element_ref::use_element_ref();
    let retry = self.saves.clone();
    let home = self.saves.home.clone();
    let resume = self.set_exiting.clone();
    Overlay::modal(self.target.clone(), ls("Saving before leaving"))
      .initial_focus(initial.clone())
      .style(match_ui::backdrop())
      .child(match_ui::panel().child((
        Heading::new(ls("Saving before leaving"), 1),
        Text::new(ls(self::status(&self.saves))).style(match_ui::text_style(20.0)),
        Text::new(ls("Background saving is best effort. Closing the app before a save finishes may lose recent moves."))
          .style(match_ui::text_style(20.0)),
        self::failed(&self.saves).then(|| Button::new(ls("Retry save"))
          .style(match_ui::button_style(20.0)).on_press(move || self::retry(&retry))),
        Button::new(ls("Keep playing")).element_ref(initial)
          .style(match_ui::button_style(20.0)).on_press(move || resume.set(false)),
        Button::new(ls("Leave without waiting")).style(match_ui::button_style(20.0))
          .on_press(move || home.set(None)),
      )))
  }
}

pub(crate) fn failed(saves: &Saves) -> bool {
  saves.game.error().is_some() || saves.settings.error().is_some()
}

fn ready(saves: &Saves) -> bool {
  let idle = saves.game.pending().is_none() && saves.settings.pending().is_none();
  !self::failed(saves) && idle && saves.game.value() == saves.game.desired()
}

pub(crate) fn retry(saves: &Saves) {
  if saves.game.error().is_some() {
    saves.game.retry();
  }
  if saves.settings.error().is_some() {
    saves.settings.retry();
  }
}

fn status(saves: &Saves) -> String {
  if self::failed(saves) {
    return "Save failed · Menu".into();
  }
  let accepted = saves.game.desired().map_or(0, |value| value.revision);
  if self::ready(saves) {
    format!("Saved revision {accepted}")
  } else {
    format!("Saving revision {accepted}…")
  }
}
