//! The ordered examples shown by the chess UI gallery.

use crate::{
  action_harness::ActionHarness,
  frame_harness::FrameHarness,
  gallery::{Demonstration, Gallery},
  portrait_harness::PortraitHarness,
  review_page::ReviewPage,
  select_harness::SelectHarness,
  setting_row_harness::SettingRowHarness,
  toggle_harness::ToggleHarness,
  volume_harness::VolumeHarness,
};

/// Builds the gallery from component values and their explanations.
///
/// Examples can carry required props and callbacks. Gallery selection keys own
/// their mounted state; registering a value neither renders it nor runs hooks.
pub fn gallery() -> Gallery {
  Gallery::new()
    .page(
      ReviewPage::new("Gallery shell")
        .description("Scrollable navigation selects one isolated demonstration; migrated mockup content is intentionally not asserted.")
        .child(Demonstration),
    )
    .page(
      ReviewPage::new("PortraitViewport")
        .description("Fixed stage scales to fit available space; responsive content reflow is not asserted.")
        .child(PortraitHarness),
    )
    .page(
      ReviewPage::new("ScreenFrame and ConceptFrame")
        .description("Arcade frame and clipped interior render; pulses, exits, generated skin, and controls are not asserted.")
        .child(FrameHarness),
    )
    .page(
      ReviewPage::new("SettingRow")
        .description("SettingRow aligns label and child horizontally; responsive reflow and interactive controls are not asserted.")
        .child(SettingRowHarness),
    )
    .page(
      ReviewPage::new("ToggleControl layout and state")
        .description("ToggleControl renders label, checkbox, and controlled toggling; focus, animation, and help remain unasserted.")
        .child(ToggleHarness),
    )
    .page(
      ReviewPage::new("SelectControl closed state")
        .description("SelectControl renders changing controlled values and its caret; opening, options, focus, and animation remain unasserted.")
        .child(SelectHarness),
    )
    .page(
      ReviewPage::new("VolumeControl layout")
        .description("VolumeControl renders track, fill, thumb, value, and controlled changes; rich input and effects remain unasserted.")
        .child(VolumeHarness),
    )
    .page(
      ReviewPage::new("ActionButton")
        .description("ActionButton renders typed children and invokes clicks; interaction states, particles, and navigation remain unasserted.")
        .child(ActionHarness),
    )
    .page(
      ReviewPage::new("SettingsTabs layout")
        .description("SettingsTabs selects controlled tabs horizontally; directional focus, panel transitions, and responsive labels remain unasserted."),
    )
    .page(
      ReviewPage::new("ScreenHeader")
        .description("ScreenHeader renders game and settings variants; generated wordmark, scaling, and animation remain unasserted."),
    )
    .page(
      ReviewPage::new("useInteraction")
        .description("useInteraction drives hover, press, release, and cancellation visuals; focus modality and particles remain unasserted."),
    )
    .page(
      ReviewPage::new("Focus-visible behavior")
        .description("Keyboard and controller focus-visible states render correctly while pointer focus hides the keyboard-only ring; complete controls remain unasserted."),
    )
    .page(
      ReviewPage::new("ToggleControl accessibility")
        .description("ToggleControl exposes labeled checkbox semantics and help description; effects, help modal, and composition remain unasserted."),
    )
    .page(
      ReviewPage::new("SelectControl pointer popover")
        .description("SelectControl opens one anchored listbox, selects options, and dismisses outside; keyboard behavior remains unasserted."),
    )
    .page(
      ReviewPage::new("SelectControl keyboard and controller behavior")
        .description("SelectControl supports arrows, Home, End, typeahead, Escape, restoration, and listbox semantics through handlers and queued ref focus; animation remains unasserted."),
    )
    .page(
      ReviewPage::new("VolumeControl input")
        .description("VolumeControl supports drag, keyboard steps, endpoints, pages, and controller input; release effects remain unasserted."),
    )
    .page(
      ReviewPage::new("SettingsTabs navigation")
        .description("SettingsTabs preserves four Tab stops and adds arrow and controller selection through handlers and ref focus; panel animation remains unasserted."),
    )
    .page(
      ReviewPage::new("ArcadeModal behavior")
        .description("ArcadeModal traps focus, dismisses safely, restores its opener, and exposes dialog semantics on its modal wrapper; animation remains unasserted."),
    )
    .page(
      ReviewPage::new("InfoBadge and Privacy Policy")
        .description("InfoBadge opens accessible crash-report help and activates Privacy Policy; data erasure remains absent."),
    )
    .page(
      ReviewPage::new("Input settings table")
        .description("InputSettings scrolls bindings beneath a sticky header; rebinding, conflicts, and visual icons remain unasserted."),
    )
    .page(
      ReviewPage::new("Keyboard rebinding")
        .description("InputSettings captures keyboard bindings, rejects conflicts, resets defaults, and announces status; icons and controller rebinding are not asserted."),
    )
    .page(
      ReviewPage::new("FontScale")
        .description("FontScale reflows rows and scales text and controls; persistence and complete screens remain unasserted."),
    )
    .page(
      ReviewPage::new("Generated control skin")
        .description("Generated assets skin controls and labels; interaction behavior, dynamic effects, and screen composition are not asserted."),
    )
    .page(
      ReviewPage::new("Input icons and settings panel skin")
        .description("InputBindingIcons and the settings panel render precisely; rebinding behavior and full composition remain unasserted."),
    )
    .page(
      ReviewPage::new("Control shine and release bursts")
        .description("Buttons, checkboxes, and sliders play shine and keyed release bursts; ambient and route effects remain unasserted."),
    )
    .page(
      ReviewPage::new("Dropdown animation")
        .description("Dropdown and options animate presence, stagger, selection flash, and interruption; settings composition remains unasserted."),
    )
    .page(
      ReviewPage::new("ArcadeTabTransition")
        .description("ArcadeTabTransition enters, exits, and sweeps by direction; complete tab contents and routing remain unasserted."),
    )
    .page(
      ReviewPage::new("ArcadeModal animation")
        .description("ArcadeModal animates backdrop, panel, and shine with reduced-motion alternatives; screen composition remains unasserted."),
    )
    .page(
      ReviewPage::new("ArcadeAttractMode")
        .description("ArcadeAttractMode animates seeded grid and particles deterministically; menu controls and audio remain unasserted."),
    )
    .page(
      ReviewPage::new("ArcadeFramePulse")
        .description("ArcadeFramePulse animates border comets around the restored Return cutout; exits and route effects remain unasserted."),
    )
    .page(
      ReviewPage::new("BackgroundMusicProvider")
        .description("BackgroundMusic loops audio, applies effective volume and background mute, and exposes playback context; heartbeat remains unasserted."),
    )
    .page(
      ReviewPage::new("Music indicator and heartbeat")
        .description("MusicPlaybackIndicator mutes or enables sound while controls pulse from audio time; complete menu composition is not asserted."),
    )
    .page(
      ReviewPage::new("ArcadeMenuTransition")
        .description("ArcadeMenuTransition swaps keyed screens with beam and reveal effects; complete routed screens remain unasserted."),
    )
    .page(
      ReviewPage::new("ArcadeExitSequence")
        .description("ArcadeExitSequence and frame collapse synchronize dismissal; gameplay, quitting, and routed composition remain unasserted."),
    )
    .page(
      ReviewPage::new("Gameplay and Graphics settings")
        .description("Gameplay and Graphics settings compose matching controls and props; other tabs and final transitions remain unasserted."),
    )
    .page(
      ReviewPage::new("SoundSettings")
        .description("SoundSettings composes three sliders and background mute against shared audio state; Input settings remain unasserted."),
    )
    .page(
      ReviewPage::new("InputSettings composition")
        .description("InputSettings composes bindings, icons, scrolling, rebinding, and its modal; cross-tab integration is not asserted."),
    )
    .page(
      ReviewPage::new("SettingsScreen")
        .description("SettingsScreen composes tabs, panels, Return, and both dialogs; main menu and route transition remain unasserted."),
    )
    .page(
      ReviewPage::new("MainMenu")
        .description("MainMenu composes background, header, buttons, music, and exit behavior; the complete router remains unasserted."),
    )
    .page(
      ReviewPage::new("ArcadeScreenRouter")
        .description("ArcadeScreenRouter composes every accessible mockup behavior; no player-visible behavior remains outside this page's scope."),
    )
}
