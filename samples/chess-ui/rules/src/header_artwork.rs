//! Prepared decorative lettering with transparent margins for offset shadows.

battlement_reactant::asset_generator::generate_family! {
  @text-image {
    @filter-mode trilinear;
    @font-file unity("Assets/Original/barlow-condensed-800-italic.ttf");
    text-align: center;
    white-space: pre;
    color: transparent;
    background: linear-gradient(174deg, #ffffff 2%, #e5f5ff 20%, #74c9ff 38%, #f8fbff 51%, #8d72ff 70%, #ff68d9 94%);
    background-clip: text;
    -webkit-text-stroke: 1.4px #f9ffff;
    filter: drop-shadow(4px 6px #092463) drop-shadow(-3px -2px #61096a) drop-shadow(0 12px 8px #000000);
  }
  GAME_LOGO {
    @canvas 854px 330px;
    @subject 62.0390625px 20.609375px 729.921875px 288.78125px;
    content: "CHESS CHESS\nREVOLUTION";
    padding: 18px 24px 34px 4px;
    font-size: 160px;
    line-height: 118.4px;
    letter-spacing: -4px;
    transform: translate(10px, -2px) scale(1.02, 0.9) skewX(-5deg);
  }
  SETTINGS_TITLE {
    @canvas 854px 240px;
    @subject 190.875px 38px 472.25px 191.296875px;
    content: "Settings";
    padding: 20px 20px 36px 4px;
    font-size: 165px;
    line-height: 135.3px;
    letter-spacing: -5px;
    transform: translate(14px, -7px) scale(1.01, 0.83) skewX(-5deg);
  }
}

battlement_reactant::asset_generator::generate_family! {
  @background {
    @canvas 314px 58px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
  }
  STRIPE_LEFT {
    background: repeating-linear-gradient(132deg, #075fff 0 17px, #05164b 17px 32px);
    clip-path: polygon(0 0, 100% 0, 93% 100%, 0 100%);
    box-shadow: 0 0 18px #075fff;
  }
  STRIPE_RIGHT {
    background: repeating-linear-gradient(132deg, #f21160 0 17px, #4b0827 17px 32px);
    clip-path: polygon(7% 0, 100% 0, 100% 100%, 0 100%);
    box-shadow: 0 0 18px #f21160;
  }
}
