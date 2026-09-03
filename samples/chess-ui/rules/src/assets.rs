//! Build-time image and font recipes consumed by the chess design system.

battlement_reactant::asset_generator::generate! {
  @background ARCADE_SCREEN_FRAME {
    @canvas 1024px 1536px;
    @subject 21px 21px 982px 1404px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    border: 8px solid transparent;
    background: linear-gradient(110deg, #f4ffff 0%, #53dcff 4%, rgb(8, 116, 239) 12%, #09234c 18%, #19ddff 32%, #e9fbff 50%, #806cff 64%, #ff39c9 83%, #ffd4f4 96%, #ff5ec2 100%);
    clip-path: path("M44.19 0L144.354 0L166.94 26.676L815.06 26.676L837.646 0L937.81 0L982 44.928L982 262.548L963.342 280.8L963.342 1384.344L947.63 1404L34.37 1404L18.658 1384.344L18.658 280.8L0 262.548L0 44.928ZM8 52.416L8 267.556L26.354 285.6L26.354 1376.568L41.81 1396L940.19 1396L955.646 1376.568L955.646 285.6L974 267.556L974 52.416L930.53 8L831.998 8L809.78 34.372L172.22 34.372L150.002 8L51.47 8Z");
    filter: drop-shadow(0 0 10px #368dff24) drop-shadow(0 0 9px #ff2ac018);
  }
}

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

battlement_reactant::asset_generator::generate! {
  @nine-slice ACTION_BUTTON_FRAME {
    @canvas 760px 140px;
    @slices 24px 26px 24px 26px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    border: 6px solid transparent;
    background: linear-gradient(#071027 0%, #020613 100%) border-box padding-box, linear-gradient(110deg, #b9fbff 0%, #3bb9ff 22%, #a49cff 56%, #ff4bd1 90%);
    box-shadow: inset 0 0 27px #000000af;
    clip-path: polygon(2.37% 0%, 97.63% 0%, 100% 12.14%, 100% 87.86%, 97.63% 100%, 2.37% 100%, 0% 87.86%, 0% 12.14%);
    filter: drop-shadow(0 0 10px #3a9affa6);
  }
}

battlement_reactant::asset_generator::generate! {
  @nine-slice SMALL_CONTROL_FRAME {
    @canvas 396px 106px;
    @slices 15px 15px 15px 15px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    border: 3px solid transparent;
    background: linear-gradient(#050b1c 0%, #020611 100%) border-box padding-box, linear-gradient(106deg, #5df5ff 0%, #a5cbff 48%, #ff4bc9 100%);
    box-shadow: inset 0 0 24px #000000af;
    clip-path: polygon(2.53% 0%, 97.47% 0%, 100% 9.43%, 100% 90.57%, 97.47% 100%, 2.53% 100%, 0% 90.57%, 0% 9.43%);
    filter: drop-shadow(0 0 6px #2a67ff61);
  }
}

battlement_reactant::asset_generator::generate! {
  @nine-slice SETTINGS_TAB_ACTIVE {
    @canvas 288px 154px;
    @subject 12px 12px 264px 130px;
    @slices 30px 42px 18px 42px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    border: 4px solid transparent;
    background: linear-gradient(#071831 0%, #030b1d 100%) border-box padding-box, linear-gradient(112deg, #72f5ff 0%, #53afff 44%, #9a83ff 68%, #ff4ed3 100%);
    box-shadow: inset 0 0 34px #000000b0, inset 0 -3px #f14dd7;
    clip-path: polygon(0% 13.85%, 6.82% 0%, 93.18% 0%, 100% 13.85%, 100% 100%, 0% 100%);
    filter: drop-shadow(0 0 10px #2385ff44);
  }
}

battlement_reactant::asset_generator::generate! {
  @nine-slice SETTINGS_TAB_INACTIVE {
    @canvas 288px 154px;
    @subject 12px 12px 264px 130px;
    @slices 30px 42px 18px 42px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    border: 4px solid transparent;
    background: linear-gradient(#071328 0%, #020817 100%) border-box padding-box, linear-gradient(112deg, #72f5ff 0%, #53afff 44%, #9a83ff 68%, #ff4ed3 100%);
    box-shadow: inset 0 0 24px #000000b0, inset 0 0 3px #123b78a8;
    clip-path: polygon(0% 13.85%, 6.82% 0%, 93.18% 0%, 100% 13.85%, 100% 100%, 0% 100%);
  }
}

battlement_reactant::asset_generator::generate! {
  @text-image GAME_LOGO {
    @canvas 900px 360px;
    @subject 0px 45px 900px 250px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    @font-file unity("Assets/Original/barlow-condensed-800-italic.ttf");
    content: "CHESS CHESS\nREVOLUTION";
    font-size: 160px;
    line-height: 118px;
    letter-spacing: -4px;
    text-align: center;
    white-space: pre;
    color: transparent;
    background: linear-gradient(174deg, #ffffff 2%, #e5f5ff 20%, #74c9ff 38%, #f8fbff 51%, #8d72ff 70%, #ff68d9 94%);
    background-clip: text;
    -webkit-text-stroke: 1.4px #f9ffff;
    filter: drop-shadow(4px 6px #092463) drop-shadow(-3px -2px #61096a) drop-shadow(0 12px 8px #000000);
    transform: scale(1.02, 0.9) skewX(-5deg);
  }
}

battlement_reactant::asset_generator::generate! {
  @text-image ACTION_LABEL_PLAY {
    @canvas 480px 146px;
    @subject 0px 21px 480px 108px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    @font-file unity("Assets/Original/barlow-condensed-800-italic.ttf");
    content: "PLAY";
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
}

battlement_reactant::asset_generator::generate! {
  @text-image ACTION_LABEL_SETTINGS {
    @canvas 480px 146px;
    @subject 0px 21px 480px 108px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    @font-file unity("Assets/Original/barlow-condensed-800-italic.ttf");
    content: "SETTINGS";
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
}

battlement_reactant::asset_generator::generate! {
  @text-image ACTION_LABEL_ABOUT {
    @canvas 480px 146px;
    @subject 0px 21px 480px 108px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    @font-file unity("Assets/Original/barlow-condensed-800-italic.ttf");
    content: "ABOUT";
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
}

battlement_reactant::asset_generator::generate! {
  @text-image ACTION_LABEL_QUIT {
    @canvas 480px 146px;
    @subject 0px 21px 480px 108px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    @font-file unity("Assets/Original/barlow-condensed-800-italic.ttf");
    content: "QUIT";
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
}

battlement_reactant::asset_generator::generate! {
  @text-image ACTION_LABEL_RETURN {
    @canvas 480px 146px;
    @subject 0px 21px 480px 108px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    @font-file unity("Assets/Original/barlow-condensed-800-italic.ttf");
    content: "RETURN";
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
}

battlement_reactant::asset_generator::generate! {
  @background CHECKBOX_UNCHECKED {
    @canvas 101px 101px;
    @subject 12px 12px 77px 77px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    border: 4px solid #4ba3ff;
    border-radius: 11px;
    background: linear-gradient(#06142b 0%, #02091a 100%);
    box-shadow: inset 0 0 14px #000000af;
    filter: drop-shadow(0 0 10px #166cff80) drop-shadow(0 0 5px #6af6ff70);
  }
}

battlement_reactant::asset_generator::generate! {
  @background CHECKBOX_CHECK {
    @canvas 101px 101px;
    @subject 25px 29px 50px 44px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    background: #61f1ff;
    clip-path: polygon(0% 47%, 14% 32%, 35% 58%, 85% 0%, 100% 14%, 35% 100%);
    filter: drop-shadow(0 0 7px #128dffb0);
  }
}
