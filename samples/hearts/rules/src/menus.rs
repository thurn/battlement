use battlement::AudioMix;
use reactant::{hooks, overlay::Overlay, portal::PortalTarget, prelude::*};
use trox::ls;

use crate::{
  match_ui,
  settings::{Preferences, Settings},
};

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Menu {
  Pause,
  Settings,
  Rules,
  ConfirmNew,
  PassingHelp,
  PlayHelp,
}

pub(crate) struct Menus {
  pub target: PortalTarget,
  pub page: Menu,
  pub set_page: hooks::StateSetter<Option<Menu>>,
  pub preferences: Preferences,
  pub set_preferences: hooks::StateSetter<Preferences>,
  pub mix: AudioMix,
  pub set_mix: hooks::StateSetter<AudioMix>,
  pub reset: EventCallback<()>,
}

impl Component for Menus {
  fn render(&self) -> impl Render {
    let font = self.preferences.font();
    let title = match self.page {
      Menu::Pause => "Game paused",
      Menu::Settings => "Settings",
      Menu::Rules => "How to play Hearts",
      Menu::ConfirmNew => "Start a new game?",
      Menu::PassingHelp => "Choose three cards to pass",
      Menu::PlayHelp => "Play a card, avoid points",
    };
    let close = self.set_page.clone();
    let dismiss = self.set_page.clone();
    let settings = self.set_page.clone();
    let rules = self.set_page.clone();
    let new_game = self.set_page.clone();
    let reset = self
      .reset
      .clone()
      .then(self.set_page.update_callback(|_| None));
    let initial = reactant::element_ref::use_element_ref();
    let content = match self.page {
      Menu::Pause => Node::new((
        Text::new(ls("Your game is paused. Resume when you are ready." )).style(match_ui::text_style(font)),
        Button::new(ls("Settings")).style(match_ui::button_style(font)).on_press(move || settings.set(Some(Menu::Settings))),
        Button::new(ls("Rules")).style(match_ui::button_style(font)).on_press(move || rules.set(Some(Menu::Rules))),
        Button::new(ls("New game")).style(match_ui::button_style(font)).on_press(move || new_game.set(Some(Menu::ConfirmNew))),
      )),
      Menu::Settings => Node::new(Settings { preferences: self.preferences, set_preferences: self.set_preferences.clone(), mix: self.mix, set_mix: self.set_mix.clone() }),
      Menu::Rules => Node::new(Text::new(ls("Lowest total wins. Each heart is 1 point; the Queen of Spades is 13. Collect all 26 points to shoot the moon: each opponent receives 26 instead.\n\nPass three cards left, right, then across; every fourth hand is a hold hand. The Two of Clubs opens. Follow suit if you can; highest card in the led suit wins the trick and leads next.\n\nNo hearts or Queen of Spades on the first trick unless you have only penalties. Hearts cannot be led until broken unless you hold only hearts. When any total reaches 100, the lowest score wins; ties share the win.")).style(match_ui::text_style(font))),
      Menu::ConfirmNew => Node::new((
        Text::new(ls("This replaces your current match. Keep playing by choosing Cancel." )).style(match_ui::text_style(font)),
        Button::new(ls("Start new game")).host_name("confirm-new-game").style(match_ui::button_style(font)).on_press(reset),
      )),
      Menu::PassingHelp => Node::new(Text::new(ls("Select three cards from your hand, then choose Pass three cards. Selected cards lift so you can check them before confirming.\n\nYou can tap or click cards, or use arrows / D-pad and Enter / the primary button. Use Inspect for a card's name and play restrictions.")).style(match_ui::text_style(font))),
      Menu::PlayHelp => Node::new(Text::new(ls("The Two of Clubs starts. Follow the led suit whenever possible. Each heart is 1 point; the Queen of Spades is 13. Lowest total wins.\n\nSelect a legal card, then select it again or choose Play selected card. You can also drag it into the center. Illegal cards remain inspectable. Escape / back cancels or closes a dialog.")).style(match_ui::text_style(font))),
    };
    Overlay::modal(self.target.clone(), ls(title))
      .initial_focus(initial.clone())
      .on_dismiss(move || dismiss.set(None))
      .style(match_ui::backdrop())
      .child(
        match_ui::panel().child((
          Heading::new(ls(title), 1).style(
            Style::new()
              .font_size((font + 8.0).px())
              .white_space(WhiteSpace::Normal)
              .margin_bottom(12.px()),
          ),
          content,
          Button::new(ls(match self.page {
            Menu::Pause => "Resume game",
            Menu::ConfirmNew => "Cancel",
            Menu::PassingHelp | Menu::PlayHelp => "Got it",
            _ => "Back to game",
          }))
          .element_ref(initial)
          .style(match_ui::button_style(font))
          .on_press(move || close.set(None)),
        )),
      )
  }
}
