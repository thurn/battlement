use battlement::{PickingMode, UiFontAddress};
use reactant::{hooks, overlay::Overlay, portal::PortalTarget, prelude::*};
use reactant_rules::ReducerGame;
use trox::ls;

use crate::{
  assets,
  controller::HeartsController,
  domain::{Event, PassDirection, Phase, Seat},
  reducer::HeartsReducer,
  settings::Preferences,
};

pub(crate) struct Results {
  pub target: PortalTarget,
  pub game: HeartsController,
  pub new_game: EventCallback<()>,
  pub menu: EventCallback<()>,
}

pub(crate) struct MatchStatus(pub HeartsController);
pub(crate) struct PresentationPause(pub bool);
pub(crate) struct Announcements;

pub(crate) fn text_style(font: f32) -> Style {
  Style::new()
    .font_size(font.px())
    .white_space(WhiteSpace::Normal)
    .margin_bottom(8.px())
}

pub(crate) fn button_style(font: f32) -> Style {
  Style::new()
    .min_height(48.px())
    .margin_top(6.px())
    .font_size(font.px())
    .color(Color::rgb(0.08, 0.14, 0.06))
    .background_color(Color::rgb(0.89, 0.85, 0.67))
    .border_radius(6.px())
}

pub(crate) fn panel() -> View {
  let layout = crate::layout::use_layout();
  let width = (layout.safe.width as f32 - 48.0).min(580.0);
  View::new().style(
    Style::new()
      .position(Position::Absolute)
      .left((layout.safe.x as f32 + (layout.safe.width as f32 - width) / 2.0).px())
      .top((layout.safe.y as f32 + 32.0).px())
      .width(width.px())
      .max_height((layout.safe.height as f32 - 64.0).px())
      .padding(20.px())
      .background_color(Color::rgb(0.96, 0.92, 0.77))
      .color(Color::rgb(0.06, 0.12, 0.03))
      .border_radius(12.px())
      .unity_font_definition(UiFontAddress::from(assets::hearts::fonts::CONTROL)),
  )
}

pub(crate) fn backdrop() -> Style {
  Style::new().background_color(Color::rgba(0.02, 0.07, 0.02, 0.65))
}

pub(crate) fn seat_name(seat: Seat) -> &'static str {
  match seat {
    Seat::South => "You",
    Seat::West => "West",
    Seat::North => "North",
    Seat::East => "East",
  }
}

pub(crate) fn passing(direction: PassDirection) -> &'static str {
  match direction {
    PassDirection::Left => "Pass three cards to the left (West).",
    PassDirection::Right => "Pass three cards to the right (East).",
    PassDirection::Across => "Pass three cards across (North).",
    PassDirection::Hold => "Hold hand: keep your cards.",
  }
}

impl Component for MatchStatus {
  fn render(&self) -> impl Render {
    let table = &self.0.view.table;
    let layout = crate::layout::use_layout();
    let p = hooks::use_context::<Preferences>();
    let prompt = match table.phase {
      Phase::Passing => self::passing(table.pass_direction).to_owned(),
      Phase::Playing { turn } => format!(
        "{}{} · {}",
        if table.pass_direction == PassDirection::Hold {
          "Hold hand · "
        } else {
          ""
        },
        if turn == Seat::South {
          "Your turn".to_owned()
        } else {
          format!("{turn:?}'s turn")
        },
        if table.hearts_broken {
          "Hearts are broken"
        } else {
          "Hearts unbroken"
        }
      ),
      Phase::HandOver => "Hand complete · review the scores".into(),
      Phase::MatchOver { .. } => "Match complete · lowest score wins".into(),
    };
    View::new()
      .picking_mode(PickingMode::Ignore)
      .style(
        Style::new()
          .position(Position::Absolute)
          .left((layout.safe.x as f32 + 18.0).px())
          .top((layout.safe.y as f32 + if layout.portrait { 120.0 } else { 58.0 }).px())
          .width(if layout.portrait {
            (layout.safe.width as f32 - 36.0).px()
          } else {
            190.px()
          })
          .color(Color::rgb(0.06, 0.12, 0.03))
          .unity_font_definition(UiFontAddress::from(assets::hearts::fonts::CONTROL)),
      )
      .child(Text::new(ls(prompt)).style(self::text_style(p.font())))
  }
}

impl Component for Results {
  fn render(&self) -> impl Render {
    let table = &self.game.view.table;
    let p = hooks::use_context::<Preferences>();
    let result = table.result.expect("results require a scored hand");
    let title = match table.phase {
      Phase::HandOver => "Hand results".to_owned(),
      Phase::MatchOver { winners } => {
        let names: Vec<_> = Seat::ALL
          .into_iter()
          .filter(|seat| winners[seat.index()])
          .map(self::seat_name)
          .collect();
        if names.len() > 1 {
          format!("Shared win · {}", names.join(" and "))
        } else {
          format!("{} won the match", names[0])
        }
      }
      _ => panic!("results require a completed hand"),
    };
    let initial = reactant::element_ref::use_element_ref();
    let next = self.game.clone();
    Overlay::modal(self.target.clone(), ls("Results"))
      .initial_focus(initial.clone())
      .style(self::backdrop())
      .child(
        self::panel().child((
          Heading::new(ls(title), 1).style(
            Style::new()
              .font_size((p.font() + 8.0).px())
              .white_space(WhiteSpace::Normal),
          ),
          Text::new(ls(format!(
            "Hand {} · lowest total wins",
            table.hand_index + 1
          )))
          .style(self::text_style(p.font())),
          result.moon.map(|seat| {
            Text::new(ls(format!(
              "{} shot the moon! Each opponent receives 26 points.",
              self::seat_name(seat)
            )))
            .style(self::text_style(p.font()))
          }),
          Seat::ALL
            .into_iter()
            .map(|seat| {
              Text::new(ls(format!(
                "{}: +{} this hand · {} total",
                self::seat_name(seat),
                result.points[seat.index()],
                table.totals[seat.index()]
              )))
              .style(self::text_style(p.font()))
            })
            .collect::<Vec<_>>(),
          if table.phase == Phase::HandOver {
            Node::new(
              Button::new(ls("Next hand"))
                .host_name("next-hand")
                .element_ref(initial)
                .style(self::button_style(p.font()))
                .disabled(!self.game.game.presentation().is_settled())
                .on_press(move || {
                  next.next_hand();
                }),
            )
          } else {
            Node::new(
              Button::new(ls("New game"))
                .element_ref(initial)
                .style(self::button_style(p.font()))
                .on_press(self.new_game.clone()),
            )
          },
          Button::new(ls("Menu"))
            .style(self::button_style(p.font()))
            .on_press(self.menu.clone()),
        )),
      )
  }
}

impl Component for PresentationPause {
  fn render(&self) -> impl Render {
    let presentation = reactant::use_game_presentation();
    let inactive = !reactant::application::use_application_state().is_active();
    let paused = self.0 || inactive;
    hooks::use_effect(
      move || {
        if paused {
          presentation.pause();
        } else {
          presentation.resume();
        }
      },
      paused,
    );
  }
}

impl Component for Announcements {
  fn render(&self) -> impl Render {
    let event = reactant::use_game_publication::<ReducerGame<HeartsReducer>>();
    let announce = reactant::announcement::use_announce();
    let occurrence = event.clone();
    hooks::use_effect(
      move || {
        if matches!(
          event.as_deref(),
          Some(Event::CardPlayed {
            broke_hearts: true,
            ..
          })
        ) {
          announce.send(ls("Hearts are broken. Hearts may now be led."));
        }
      },
      occurrence,
    );
  }
}
