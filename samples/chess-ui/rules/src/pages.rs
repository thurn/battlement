//! The ordered examples shown by the chess UI gallery.

use trox::tx;

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
            ReviewPage::new()
                .title(tx("Gallery shell", "User-facing product copy in the Chess UI sample."))
                .description(tx("Scrollable navigation selects one isolated demonstration; migrated mockup content is intentionally not asserted.", "User-facing product copy in the Chess UI sample."))
                .child(Demonstration::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("PortraitViewport", "User-facing product copy in the Chess UI sample."))
                .description(tx("Fixed stage scales to fit available space; responsive content reflow is not asserted.", "User-facing product copy in the Chess UI sample."))
                .child(PortraitHarness::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("ScreenFrame and ConceptFrame", "User-facing product copy in the Chess UI sample."))
                .description(tx("Arcade frame and clipped interior render; pulses, exits, generated skin, and controls are not asserted.", "User-facing product copy in the Chess UI sample."))
                .child(FrameHarness::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("SettingRow", "User-facing product copy in the Chess UI sample."))
                .description(tx("SettingRow aligns label and child horizontally; responsive reflow and interactive controls are not asserted.", "User-facing product copy in the Chess UI sample."))
                .child(SettingRowHarness::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("ToggleControl layout and state", "User-facing product copy in the Chess UI sample."))
                .description(tx("ToggleControl renders label, checkbox, and controlled toggling; focus, animation, and help remain unasserted.", "User-facing product copy in the Chess UI sample."))
                .child(ToggleHarness::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("SelectControl closed state", "User-facing product copy in the Chess UI sample."))
                .description(tx("SelectControl renders changing controlled values and its caret; opening, options, focus, and animation remain unasserted.", "User-facing product copy in the Chess UI sample."))
                .child(SelectHarness::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("VolumeControl layout", "User-facing product copy in the Chess UI sample."))
                .description(tx("VolumeControl renders track, fill, thumb, value, and controlled changes; rich input and effects remain unasserted.", "User-facing product copy in the Chess UI sample."))
                .child(VolumeHarness::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("ActionButton", "User-facing product copy in the Chess UI sample."))
                .description(tx("ActionButton renders typed children and invokes clicks; interaction states, particles, and navigation remain unasserted.", "User-facing product copy in the Chess UI sample."))
                .child(ActionHarness::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("SettingsTabs layout", "User-facing product copy in the Chess UI sample."))
                .description(tx("SettingsTabs selects controlled tabs horizontally; directional focus, panel transitions, and responsive labels remain unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("ScreenHeader", "User-facing product copy in the Chess UI sample."))
                .description(tx("ScreenHeader renders game and settings variants; generated wordmark, scaling, and animation remain unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("useInteraction", "User-facing product copy in the Chess UI sample."))
                .description(tx("useInteraction drives hover, press, release, and cancellation visuals; focus modality and particles remain unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("Focus-visible behavior", "User-facing product copy in the Chess UI sample."))
                .description(tx("Keyboard and controller focus-visible states render correctly while pointer focus hides the keyboard-only ring; complete controls remain unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("ToggleControl control_behavior", "User-facing product copy in the Chess UI sample."))
                .description(tx("ToggleControl exposes labeled checkbox semantics and help description; effects, help modal, and composition remain unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("SelectControl pointer popover", "User-facing product copy in the Chess UI sample."))
                .description(tx("SelectControl opens one anchored listbox, selects options, and dismisses outside; keyboard behavior remains unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("SelectControl keyboard and controller behavior", "User-facing product copy in the Chess UI sample."))
                .description(tx("SelectControl supports arrows, Home, End, typeahead, Escape, restoration, and listbox semantics through handlers and queued ref focus; animation remains unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("VolumeControl input", "User-facing product copy in the Chess UI sample."))
                .description(tx("VolumeControl supports drag, keyboard steps, endpoints, pages, and controller input; release effects remain unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("SettingsTabs navigation", "User-facing product copy in the Chess UI sample."))
                .description(tx("SettingsTabs preserves four Tab stops and adds arrow and controller selection through handlers and ref focus; panel animation remains unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("ArcadeModal behavior", "User-facing product copy in the Chess UI sample."))
                .description(tx("ArcadeModal traps focus, dismisses safely, restores its opener, and exposes dialog semantics on its modal wrapper; animation remains unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("InfoBadge and Privacy Policy", "User-facing product copy in the Chess UI sample."))
                .description(tx("InfoBadge opens accessible crash-report help and activates Privacy Policy; data erasure remains absent.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("Input settings table", "User-facing product copy in the Chess UI sample."))
                .description(tx("InputSettings scrolls bindings beneath a sticky header; rebinding, conflicts, and visual icons remain unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("Keyboard rebinding", "User-facing product copy in the Chess UI sample."))
                .description(tx("InputSettings captures keyboard bindings, rejects conflicts, resets defaults, and announces status; icons and controller rebinding are not asserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("FontScale", "User-facing product copy in the Chess UI sample."))
                .description(tx("FontScale reflows rows and scales text and controls; persistence and complete screens remain unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("Generated control skin", "User-facing product copy in the Chess UI sample."))
                .description(tx("Generated assets skin controls and labels; interaction behavior, dynamic effects, and screen composition are not asserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("Input icons and settings panel skin", "User-facing product copy in the Chess UI sample."))
                .description(tx("InputBindingIcons and the settings panel render precisely; rebinding behavior and full composition remain unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("Control shine and release bursts", "User-facing product copy in the Chess UI sample."))
                .description(tx("Buttons, checkboxes, and sliders play shine and keyed release bursts; ambient and route effects remain unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("Dropdown animation", "User-facing product copy in the Chess UI sample."))
                .description(tx("Dropdown and options animate presence, stagger, selection flash, and interruption; settings composition remains unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("ArcadeTabTransition", "User-facing product copy in the Chess UI sample."))
                .description(tx("ArcadeTabTransition enters, exits, and sweeps by direction; complete tab contents and routing remain unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("ArcadeModal animation", "User-facing product copy in the Chess UI sample."))
                .description(tx("ArcadeModal animates backdrop, panel, and shine with reduced-motion alternatives; screen composition remains unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("ArcadeAttractMode", "User-facing product copy in the Chess UI sample."))
                .description(tx("ArcadeAttractMode animates seeded grid and particles deterministically; menu controls and audio remain unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("ArcadeFramePulse", "User-facing product copy in the Chess UI sample."))
                .description(tx("ArcadeFramePulse animates border comets around the restored Return cutout; exits and route effects remain unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("BackgroundMusicProvider", "User-facing product copy in the Chess UI sample."))
                .description(tx("BackgroundMusic loops audio, applies effective volume and background mute, and exposes playback context; heartbeat remains unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("Music indicator and heartbeat", "User-facing product copy in the Chess UI sample."))
                .description(tx("MusicPlaybackIndicator mutes or enables sound while controls pulse from audio time; complete menu composition is not asserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("ArcadeMenuTransition", "User-facing product copy in the Chess UI sample."))
                .description(tx("ArcadeMenuTransition swaps keyed screens with beam and reveal effects; complete routed screens remain unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("ArcadeExitSequence", "User-facing product copy in the Chess UI sample."))
                .description(tx("ArcadeExitSequence and frame collapse synchronize dismissal; gameplay, quitting, and routed composition remain unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("Gameplay and Graphics settings", "User-facing product copy in the Chess UI sample."))
                .description(tx("Gameplay and Graphics settings compose matching controls and props; other tabs and final transitions remain unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("SoundSettings", "User-facing product copy in the Chess UI sample."))
                .description(tx("SoundSettings composes three sliders and background mute against shared audio state; Input settings remain unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("InputSettings composition", "User-facing product copy in the Chess UI sample."))
                .description(tx("InputSettings composes bindings, icons, scrolling, rebinding, and its modal; cross-tab integration is not asserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("SettingsScreen", "User-facing product copy in the Chess UI sample."))
                .description(tx("SettingsScreen composes tabs, panels, Return, and both dialogs; main menu and route transition remain unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("MainMenu", "User-facing product copy in the Chess UI sample."))
                .description(tx("MainMenu composes background, header, buttons, music, and exit behavior; the complete router remains unasserted.", "User-facing product copy in the Chess UI sample.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("ArcadeScreenRouter", "User-facing product copy in the Chess UI sample."))
                .description(tx("ArcadeScreenRouter composes every accessible mockup behavior; no player-visible behavior remains outside this page's scope.", "User-facing product copy in the Chess UI sample.")),
        )
}
