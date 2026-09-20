use reactant::{Application, host::View};

#[test]
fn ui_only_app_needs_no_game_state_or_rules_session() {
  let _app = Application::new("ui-only/content").child(View::new());
}

#[test]
fn facade_and_ui_layer_share_the_core_render_contract() {
  fn require_core_render(_: impl reactant_core::render::Render) {}

  let view: reactant_ui::host::View = View::new();
  require_core_render(view);
}
