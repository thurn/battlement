//! The ordered examples shown by the chess UI gallery.

use battlement::Style;
use battlement_reactant::portal::PortalTarget;
use trox::tx;

use crate::{
  action_harness::ActionHarness,
  arcade_modal_harness::ArcadeModalHarness,
  frame_harness::FrameHarness,
  gallery::{Demonstration, Gallery},
  header_harness::HeaderHarness,
  input_settings::InputSettings,
  interaction_harness::InteractionHarness,
  portrait_harness::PortraitHarness,
  privacy_harness::PrivacyHarness,
  review_page::ReviewPage,
  select_harness::SelectHarness,
  select_popover_harness::SelectPopoverHarness,
  setting_row_harness::SettingRowHarness,
  tabs_harness::TabsHarness,
  toggle_accessibility_harness::ToggleAccessibilityHarness,
  toggle_harness::ToggleHarness,
  volume_harness::VolumeHarness,
  volume_input_harness::VolumeInputHarness,
};

/// Builds the gallery from component values and their explanations.
///
/// Examples can carry required props and callbacks. Gallery selection keys own
/// their mounted state; registering a value neither renders it nor runs hooks.
pub fn gallery(overlay: PortalTarget) -> Gallery {
  Gallery::new()
        .page(
            ReviewPage::new()
                .title(tx("Gallery shell", "Chess UI showcase title."))
                .description(tx("Scrollable navigation selects one isolated demonstration; migrated mockup content is intentionally not asserted.", "Chess UI showcase description."))
                .child(Demonstration::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("PortraitViewport", "Chess UI showcase title."))
                .description(tx("Fixed stage scales to fit available space; responsive content reflow is not asserted.", "Chess UI showcase description."))
                .child(PortraitHarness::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("ScreenFrame and ConceptFrame", "Chess UI showcase title."))
                .description(tx("Arcade frame and clipped interior render; pulses, exits, generated skin, and controls are not asserted.", "Chess UI showcase description."))
                .child(FrameHarness::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("SettingRow", "Chess UI showcase title."))
                .description(tx("SettingRow aligns label and child horizontally; responsive reflow and interactive controls are not asserted.", "Chess UI showcase description."))
                .child(SettingRowHarness::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("ToggleControl layout and state", "Chess UI showcase title."))
                .description(tx("ToggleControl renders label, checkbox, and controlled toggling; focus, animation, and help remain unasserted.", "Chess UI showcase description."))
                .child(ToggleHarness::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("SelectControl closed state", "Chess UI showcase title."))
                .description(tx("SelectControl renders changing controlled values and its caret; opening, options, focus, and animation remain unasserted.", "Chess UI showcase description."))
                .child(SelectHarness::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("VolumeControl layout", "Chess UI showcase title."))
                .description(tx("VolumeControl renders track, fill, thumb, value, and controlled changes; rich input and effects remain unasserted.", "Chess UI showcase description."))
                .child(VolumeHarness::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("ActionButton", "Chess UI showcase title."))
                .description(tx("ActionButton renders typed children and invokes clicks; interaction states, particles, and navigation remain unasserted.", "Chess UI showcase description."))
                .child(ActionHarness::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("SettingsTabs layout", "Chess UI showcase title."))
                .description(tx("SettingsTabs selects controlled tabs horizontally; directional focus, panel transitions, and responsive labels remain unasserted.", "Chess UI showcase description."))
                .child(TabsHarness::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("ScreenHeader", "Chess UI showcase title."))
                .description(tx("ScreenHeader renders generated heading artwork; text scaling and complete screen composition remain unasserted.", "Chess UI showcase description."))
                .style(Style::new().padding_top(800))
                .child(HeaderHarness::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("useInteraction", "Chess UI showcase title."))
                .description(tx("useInteraction drives hover, press, release, and cancellation visuals; focus modality and particles remain unasserted.", "Chess UI showcase description."))
                .child(InteractionHarness::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("Focus-visible behavior", "Chess UI showcase title."))
                .description(tx("Keyboard and controller focus-visible states render correctly while pointer focus hides the keyboard-only ring; complete controls remain unasserted.", "Chess UI showcase description."))
                .child(InteractionHarness::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("ToggleControl accessibility", "Chess UI showcase title."))
                .description(tx("ToggleControl exposes labeled checkbox semantics and help description; effects, help modal, and composition remain unasserted.", "Chess UI showcase description."))
                .child(ToggleAccessibilityHarness::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("SelectControl pointer popover", "Chess UI showcase title."))
                .description(tx("SelectControl opens one anchored listbox, selects options, and dismisses outside; keyboard behavior remains unasserted.", "Chess UI showcase description."))
                .child(SelectPopoverHarness::new().overlay(overlay.clone())),
        )
        .page(
            ReviewPage::new()
                .title(tx("SelectControl keyboard and controller behavior", "Chess UI showcase title."))
                .description(tx("SelectControl supports arrows, Home, End, typeahead, Escape, restoration, and listbox semantics through handlers and queued ref focus; animation remains unasserted.", "Chess UI showcase description."))
                .child(SelectPopoverHarness::new().overlay(overlay.clone())),
        )
        .page(
            ReviewPage::new()
                .title(tx("VolumeControl input", "Chess UI showcase title."))
                .description(tx("VolumeControl supports drag, keyboard steps, endpoints, pages, and controller input; release effects remain unasserted.", "Chess UI showcase description."))
                .child(VolumeInputHarness::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("SettingsTabs navigation", "Chess UI showcase title."))
                .description(tx("SettingsTabs preserves four Tab stops and adds arrow and controller selection through handlers and ref focus; panel animation remains unasserted.", "Chess UI showcase description."))
                .child(TabsHarness::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("ArcadeModal behavior", "Chess UI showcase title."))
                .description(tx("ArcadeModal traps focus, dismisses safely, restores its opener, and exposes dialog semantics on its modal wrapper; animation remains unasserted.", "Chess UI showcase description."))
                .child(ArcadeModalHarness::new().overlay(overlay.clone())),
        )
        .page(
            ReviewPage::new()
                .title(tx("InfoBadge and Privacy Policy", "Chess UI showcase title."))
                .description(tx("InfoBadge opens accessible crash-report help and activates Privacy Policy; data erasure remains absent.", "Chess UI showcase description."))
                .child(PrivacyHarness::new().overlay(overlay.clone())),
        )
        .page(
            ReviewPage::new()
                .title(tx("Input settings table", "Chess UI showcase title."))
                .description(tx("InputSettings scrolls bindings beneath a sticky header; rebinding, conflicts, and visual icons remain unasserted.", "Chess UI showcase description."))
                .child(InputSettings::new()),
        )
        .page(
            ReviewPage::new()
                .title(tx("Keyboard rebinding", "Chess UI showcase title."))
                .description(tx("InputSettings captures keyboard bindings, rejects conflicts, resets defaults, and announces status; icons and controller rebinding are not asserted.", "Chess UI showcase description."))
                .child(InputSettings::new().overlay(overlay.clone())),
        )
        .page(
            ReviewPage::new()
                .title(tx("FontScale", "Chess UI showcase title."))
                .description(tx("FontScale reflows rows and scales text and controls; persistence and complete screens remain unasserted.", "Chess UI showcase description.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("Generated control skin", "Chess UI showcase title."))
                .description(tx("Generated assets skin controls and labels; interaction behavior, dynamic effects, and screen composition are not asserted.", "Chess UI showcase description.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("Input icons and settings panel skin", "Chess UI showcase title."))
                .description(tx("InputBindingIcons and the settings panel render precisely; rebinding behavior and full composition remain unasserted.", "Chess UI showcase description.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("Control shine and release bursts", "Chess UI showcase title."))
                .description(tx("Buttons, checkboxes, and sliders play shine and keyed release bursts; ambient and route effects remain unasserted.", "Chess UI showcase description.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("Dropdown animation", "Chess UI showcase title."))
                .description(tx("Dropdown and options animate presence, stagger, selection flash, and interruption; settings composition remains unasserted.", "Chess UI showcase description.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("ArcadeTabTransition", "Chess UI showcase title."))
                .description(tx("ArcadeTabTransition enters, exits, and sweeps by direction; complete tab contents and routing remain unasserted.", "Chess UI showcase description.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("ArcadeModal animation", "Chess UI showcase title."))
                .description(tx("ArcadeModal animates backdrop, panel, and shine with reduced-motion alternatives; screen composition remains unasserted.", "Chess UI showcase description.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("ArcadeAttractMode", "Chess UI showcase title."))
                .description(tx("ArcadeAttractMode animates seeded grid and particles deterministically; menu controls and audio remain unasserted.", "Chess UI showcase description.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("ArcadeFramePulse", "Chess UI showcase title."))
                .description(tx("ArcadeFramePulse animates border comets around the restored Return cutout; exits and route effects remain unasserted.", "Chess UI showcase description.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("BackgroundMusicProvider", "Chess UI showcase title."))
                .description(tx("BackgroundMusic loops audio, applies effective volume and background mute, and exposes playback context; heartbeat remains unasserted.", "Chess UI showcase description.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("Music indicator and heartbeat", "Chess UI showcase title."))
                .description(tx("MusicPlaybackIndicator mutes or enables sound while controls pulse from audio time; complete menu composition is not asserted.", "Chess UI showcase description.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("ArcadeMenuTransition", "Chess UI showcase title."))
                .description(tx("ArcadeMenuTransition swaps keyed screens with beam and reveal effects; complete routed screens remain unasserted.", "Chess UI showcase description.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("ArcadeExitSequence", "Chess UI showcase title."))
                .description(tx("ArcadeExitSequence and frame collapse synchronize dismissal; gameplay, quitting, and routed composition remain unasserted.", "Chess UI showcase description.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("Gameplay and Graphics settings", "Chess UI showcase title."))
                .description(tx("Gameplay and Graphics settings compose matching controls and props; other tabs and final transitions remain unasserted.", "Chess UI showcase description.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("SoundSettings", "Chess UI showcase title."))
                .description(tx("SoundSettings composes three sliders and background mute against shared audio state; Input settings remain unasserted.", "Chess UI showcase description.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("InputSettings composition", "Chess UI showcase title."))
                .description(tx("InputSettings composes bindings, icons, scrolling, rebinding, and its modal; cross-tab integration is not asserted.", "Chess UI showcase description.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("SettingsScreen", "Chess UI showcase title."))
                .description(tx("SettingsScreen composes tabs, panels, Return, and both dialogs; main menu and route transition remain unasserted.", "Chess UI showcase description.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("MainMenu", "Chess UI showcase title."))
                .description(tx("MainMenu composes background, header, buttons, music, and exit behavior; the complete router remains unasserted.", "Chess UI showcase description.")),
        )
        .page(
            ReviewPage::new()
                .title(tx("ArcadeScreenRouter", "Chess UI showcase title."))
                .description(tx("ArcadeScreenRouter composes every accessible mockup behavior; no player-visible behavior remains outside this page's scope.", "Chess UI showcase description.")),
        )
}
