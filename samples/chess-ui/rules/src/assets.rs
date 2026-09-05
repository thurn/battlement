//! Build-time image and font recipes consumed by the chess design system.

battlement_reactant::asset_generator::generate! {
  @background SETTINGS_PANEL_FRAME {
    @canvas 887px 1021px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    border: 2px solid transparent;
    background: radial-gradient(ellipse at 7% 46%, #0553b826 0%, transparent 36%) border-box padding-box, linear-gradient(90deg, #0053be12 0%, transparent 25%, transparent 75%, #7e00910e 100%) border-box padding-box, linear-gradient(#041126 0%, #020b1b 100%) border-box padding-box, linear-gradient(110deg, #446690 0%, #2c456f 54%, #875984 100%);
    box-shadow: inset 0 0 45px #000000af;
    clip-path: polygon(0% 0%, 100% 0%, 100% 98.5%, 98.4% 100%, 1.5% 100%, 0% 98.5%);
    filter: drop-shadow(0 0 5px #1c59b447);
    isolation: isolate;
  }
}

battlement_reactant::asset_generator::generate_family! {
  @text-image {
    @canvas 480px 146px;
    @subject 0px 21px 480px 108px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    @font-file unity("Assets/Original/barlow-condensed-800-italic.ttf");
    font-size: 91px;
    line-height: 108px;
    letter-spacing: -2px;
    text-align: center;
    white-space: nowrap;
    color: transparent;
    background: linear-gradient(174deg, #ffffff 5%, #dff8ff 31%, #52baff 49%, #f8faff 57%, rgb(128, 110, 255) 77%, #ff6dda 100%);
    background-clip: text;
    -webkit-text-stroke: 1px #f7ffff;
    filter: drop-shadow(3px 5px #122964) drop-shadow(0 7px 5px #000000);
    transform: skewX(-5deg);
  }
  ACTION_LABEL_PLAY { content: "PLAY"; }
  ACTION_LABEL_SETTINGS { content: "SETTINGS"; }
  ACTION_LABEL_ABOUT { content: "ABOUT"; }
  ACTION_LABEL_QUIT { content: "QUIT"; }
  ACTION_LABEL_RETURN { content: "RETURN"; }
}
