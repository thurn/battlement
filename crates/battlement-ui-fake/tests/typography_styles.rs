use battlement_types::{Color, ObjectId, UiFontAddress};
use battlement_ui::{
  EditorTextRenderingMode, FontStyle, Label, Prop, Style, StyleValue, TextAnchor, TextAutoSize,
  TextElement, TextGenerator, TextOverflow, TextOverflowPosition, TextShadow, UiDocument,
  UiElement, UiNode, Visibility, VisualElementUpdate, WhiteSpace,
};
use battlement_ui_fake::{UiWorld, UiWorldError};

macro_rules! assert_typography_fields {
  ($style:expr, $pattern:pat) => {
    assert!(matches!($style.font_size, $pattern));
    assert!(matches!($style.letter_spacing, $pattern));
    assert!(matches!($style.text_overflow, $pattern));
    assert!(matches!($style.text_shadow, $pattern));
    assert!(matches!($style.unity_editor_text_rendering_mode, $pattern));
    assert!(matches!($style.unity_font_definition, $pattern));
    assert!(matches!($style.unity_font_style_and_weight, $pattern));
    assert!(matches!($style.unity_paragraph_spacing, $pattern));
    assert!(matches!($style.unity_text_align, $pattern));
    assert!(matches!($style.unity_text_auto_size, $pattern));
    assert!(matches!($style.unity_text_generator, $pattern));
    assert!(matches!($style.unity_text_outline_color, $pattern));
    assert!(matches!($style.unity_text_outline_width, $pattern));
    assert!(matches!($style.unity_text_overflow_position, $pattern));
    assert!(matches!($style.visibility, $pattern));
    assert!(matches!($style.white_space, $pattern));
    assert!(matches!($style.word_spacing, $pattern));
  };
}

#[test]
fn text_properties_and_complete_typography_style_merge_sparsely() {
  let id = ObjectId::new_v4();
  let initial = TextElement::new("<b>Signal</b> 🚀")
    .rich_text(true)
    .emoji_fallback(true)
    .parse_escape_sequences(true)
    .tooltip_when_elided(true)
    .selectable(true)
    .double_click_selects_word(true)
    .triple_click_selects_line(true)
    .select_all_on_focus(false)
    .select_all_on_mouse_up(false)
    .style(complete_style());
  let mut world = UiWorld::default();
  world
    .replace(vec![
      UiDocument::new(ObjectId::new_v4()).child(UiNode::new(id, initial)),
    ])
    .unwrap();
  let font = UiFontAddress::new("ui/font-definition");
  assert_eq!(world.font_usage_count(&font), 1);

  world
    .update(VisualElementUpdate::Properties {
      object_id: id,
      element: UiElement::from(TextElement::default().rich_text(false)).into(),
    })
    .unwrap();
  let UiElement::TextElement(value) = world.element(id).unwrap().element() else {
    panic!("expected text element");
  };
  assert_eq!(value.enable_rich_text, Some(false));
  assert_eq!(value.emoji_fallback_support, Some(true));
  assert_eq!(value.text.as_deref(), Some("<b>Signal</b> 🚀"));
  assert_eq!(
    value.element.style.unity_text_generator,
    Prop::Set(StyleValue::Value(TextGenerator::Advanced))
  );

  world
    .update(VisualElementUpdate::Properties {
      object_id: id,
      element: UiElement::from(TextElement::default().style(reset_typography())).into(),
    })
    .unwrap();
  let reset = world.element(id).unwrap().style();
  assert_typography_fields!(reset, Prop::Reset);
  assert_eq!(world.font_usage_count(&font), 0);
}

#[test]
fn invalid_typography_numbers_reject_without_mutation() {
  let id = ObjectId::new_v4();
  let mut world = UiWorld::default();
  world
    .replace(vec![UiDocument::new(ObjectId::new_v4()).child(
      UiNode::new(id, Label::new("Stable").style(Style::new().font_size(24))),
    )])
    .unwrap();
  let before = world.element(id).unwrap().style().clone();

  for style in [
    Style::new().font_size(0.0),
    Style::new().unity_text_outline_width(-1.0),
    Style::new().text_shadow(TextShadow::new(0.0, 0.0, -1.0, Color::rgb(0.0, 0.0, 0.0))),
    Style::new().unity_text_auto_size(TextAutoSize::best_fit(32.0, 24.0)),
  ] {
    assert_eq!(
      world.update(VisualElementUpdate::Properties {
        object_id: id,
        element: UiElement::from(Label::default().style(style)).into(),
      }),
      Err(UiWorldError::InvalidProperty)
    );
    assert_eq!(world.element(id).unwrap().style(), &before);
  }
}

fn complete_style() -> Style {
  Style::new()
    .font_size(28)
    .letter_spacing(1.0)
    .text_overflow(TextOverflow::Ellipsis)
    .text_shadow(TextShadow::new(
      2.0,
      3.0,
      1.0,
      Color::rgba(0.0, 0.0, 0.0, 0.7),
    ))
    .unity_editor_text_rendering_mode(EditorTextRenderingMode::Sdf)
    .unity_font_definition(UiFontAddress::new("ui/font-definition"))
    .unity_font_style_and_weight(FontStyle::BoldAndItalic)
    .unity_paragraph_spacing(4)
    .unity_text_align(TextAnchor::MiddleCenter)
    .unity_text_auto_size(TextAutoSize::best_fit(24.0, 40.0))
    .unity_text_generator(TextGenerator::Advanced)
    .unity_text_outline_color(Color::rgb(0.2, 0.4, 0.8))
    .unity_text_outline_width(2)
    .unity_text_overflow_position(TextOverflowPosition::Middle)
    .visibility(Visibility::Hidden)
    .white_space(WhiteSpace::PreWrap)
    .word_spacing(5)
}

fn reset_typography() -> Style {
  Style::new()
    .font_size(Prop::Reset)
    .letter_spacing(Prop::Reset)
    .text_overflow(Prop::Reset)
    .text_shadow(Prop::Reset)
    .unity_editor_text_rendering_mode(Prop::Reset)
    .unity_font_definition(Prop::Reset)
    .unity_font_style_and_weight(Prop::Reset)
    .unity_paragraph_spacing(Prop::Reset)
    .unity_text_align(Prop::Reset)
    .unity_text_auto_size(Prop::Reset)
    .unity_text_generator(Prop::Reset)
    .unity_text_outline_color(Prop::Reset)
    .unity_text_outline_width(Prop::Reset)
    .unity_text_overflow_position(Prop::Reset)
    .visibility(Prop::Reset)
    .white_space(Prop::Reset)
    .word_spacing(Prop::Reset)
}
