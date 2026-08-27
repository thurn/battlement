use serde::{Deserialize, Serialize};

use crate::{
    LanguageDirection, PickingMode, Style, UsageHint, Vector, VisualElement,
    VisualElementProperties,
    elements::parts::{self, Part, PartStyle},
};

/// Axes along which a [`ScrollView`] lays out and scrolls its content.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ScrollViewMode {
    /// Lays content out vertically and scrolls on the vertical axis.
    Vertical,
    /// Lays content out horizontally and scrolls on the horizontal axis.
    Horizontal,
    /// Allows independent horizontal and vertical scrolling.
    VerticalAndHorizontal,
}

/// How a nested [`ScrollView`] handles input after reaching a scroll boundary.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum NestedInteraction {
    /// Uses Unity's normal nested scrolling behavior for the input device.
    Default,
    /// Consumes the gesture and stops scrolling at this view's boundary.
    StopScrolling,
    /// Forwards the remaining gesture to an eligible ancestor scroll view.
    ForwardScrolling,
}

/// Visibility policy for one of a [`ScrollView`]'s native scrollers.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ScrollerVisibility {
    /// Shows the scroller only when content exceeds the viewport on its axis.
    Auto,
    /// Reserves and displays the scroller even when no scrolling is necessary.
    AlwaysVisible,
    /// Hides the scroller while retaining programmatic scrolling.
    Hidden,
}

/// Boundary behavior for touch-driven scrolling.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TouchScrollBehavior {
    /// Allows the offset to move beyond the content boundaries without springing back.
    Unrestricted,
    /// Temporarily stretches past a boundary and springs back using [`ScrollView::elasticity`].
    Elastic,
    /// Keeps the offset inside the content boundaries.
    Clamped,
}

/// A viewport that displays arbitrary child content through a scrollable frame.
///
/// Children are inserted into Unity's unbounded content container; wrap text in
/// a finite-width child container when line wrapping is required. Scroll offsets
/// use upper-left-origin panel pixels. [`crate::UiEventKind::ScrollChanged`]
/// reports user-driven motion, while [`crate::UiEventKind::ScrollSettled`]
/// reports the final offset after 100 milliseconds without motion or pointer capture.
///
/// The mode controls both scrolling axes and the content container's layout
/// direction. Scroller visibility is independent per axis. Touch-specific
/// deceleration, elasticity, and boundary behavior do not change mouse-wheel or
/// scrollbar interaction. For nested views, [`Self::nested_interaction`]
/// determines whether motion can continue into an ancestor after this view
/// reaches a boundary.
///
/// See Unity's [ScrollView manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-ScrollView.html)
/// and [scripting API](https://docs.unity3d.com/6000.5/Documentation/ScriptReference/UIElements.ScrollView.html).
///
/// # Example
///
/// ```
/// use battlement_types::ObjectId;
/// use battlement_ui::{Label, ScrollView, ScrollViewMode, UiEventKind, UiNode};
///
/// let log = UiNode::new(
///     ObjectId::new_v4(),
///     ScrollView::new()
///         .mode(ScrollViewMode::Vertical)
///         .events([UiEventKind::ScrollSettled]),
/// )
/// .children([
///     UiNode::new(ObjectId::new_v4(), Label::new("Encounter started")),
///     UiNode::new(ObjectId::new_v4(), Label::new("Initiative rolled")),
/// ]);
///
/// assert_eq!(log.children.len(), 2);
/// ```
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ScrollView {
    /// Shared visual properties, inline style, and event subscriptions.
    #[serde(flatten)]
    pub element: VisualElement,
    /// Content layout and enabled scrolling axes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<ScrollViewMode>,
    /// Gesture propagation behavior when this view reaches a boundary inside another view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nested_interaction: Option<NestedInteraction>,
    /// Visibility policy for the horizontal scrollbar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizontal_scroller_visibility: Option<ScrollerVisibility>,
    /// Visibility policy for the vertical scrollbar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_scroller_visibility: Option<ScrollerVisibility>,
    /// Current horizontal and vertical content displacement in panel pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scroll_offset: Option<Vector>,
    /// Horizontal button and keyboard step as a proportion of viewport width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizontal_page_size: Option<f32>,
    /// Vertical button and keyboard step as a proportion of viewport height.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_page_size: Option<f32>,
    /// Mouse-wheel displacement in panel pixels per input line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mouse_wheel_scroll_size: Option<f32>,
    /// Boundary behavior used for touch scrolling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub touch_scroll_behavior: Option<TouchScrollBehavior>,
    /// Fraction of touch-scroll velocity retained each second after release.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scroll_deceleration_rate: Option<f32>,
    /// Spring strength used by elastic touch scrolling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elasticity: Option<f32>,
    /// Minimum interval in milliseconds between elastic spring updates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elastic_animation_interval: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) parts: Option<Vec<PartStyle>>,
}

impl ScrollView {
    /// Creates a vertical view using Unity's scrolling defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    impl_common_visual_element_methods!();

    /// Applies sparse inline declarations to the native `ScrollViewContentAndVerticalScrollContainer` part.
    #[must_use]
    pub fn content_and_vertical_scroll_container_style(mut self, value: Style) -> Self {
        parts::append(
            &mut self.parts,
            Part::ScrollViewContentAndVerticalScrollContainer,
            value,
        );
        self
    }

    /// Applies sparse inline declarations to the native `ScrollViewViewport` part.
    #[must_use]
    pub fn viewport_style(mut self, value: Style) -> Self {
        parts::append(&mut self.parts, Part::ScrollViewViewport, value);
        self
    }

    /// Applies sparse inline declarations to the native `ScrollViewContentContainer` part.
    #[must_use]
    pub fn content_container_style(mut self, value: Style) -> Self {
        parts::append(&mut self.parts, Part::ScrollViewContentContainer, value);
        self
    }

    /// Applies sparse inline declarations to the native `ScrollViewHorizontalScroller` part.
    #[must_use]
    pub fn horizontal_scroller_style(mut self, value: Style) -> Self {
        parts::append(&mut self.parts, Part::ScrollViewHorizontalScroller, value);
        self
    }

    /// Applies sparse inline declarations to the native `ScrollViewHorizontalSlider` part.
    #[must_use]
    pub fn horizontal_slider_style(mut self, value: Style) -> Self {
        parts::append(&mut self.parts, Part::ScrollViewHorizontalSlider, value);
        self
    }

    /// Applies sparse inline declarations to the native `ScrollViewHorizontalLowButton` part.
    #[must_use]
    pub fn horizontal_low_button_style(mut self, value: Style) -> Self {
        parts::append(&mut self.parts, Part::ScrollViewHorizontalLowButton, value);
        self
    }

    /// Applies sparse inline declarations to the native `ScrollViewHorizontalHighButton` part.
    #[must_use]
    pub fn horizontal_high_button_style(mut self, value: Style) -> Self {
        parts::append(&mut self.parts, Part::ScrollViewHorizontalHighButton, value);
        self
    }

    /// Applies sparse inline declarations to the native `ScrollViewHorizontalTrack` part.
    #[must_use]
    pub fn horizontal_track_style(mut self, value: Style) -> Self {
        parts::append(&mut self.parts, Part::ScrollViewHorizontalTrack, value);
        self
    }

    /// Applies sparse inline declarations to the native `ScrollViewHorizontalDragger` part.
    #[must_use]
    pub fn horizontal_dragger_style(mut self, value: Style) -> Self {
        parts::append(&mut self.parts, Part::ScrollViewHorizontalDragger, value);
        self
    }

    /// Applies sparse inline declarations to the native `ScrollViewHorizontalDraggerBorder` part.
    #[must_use]
    pub fn horizontal_dragger_border_style(mut self, value: Style) -> Self {
        parts::append(
            &mut self.parts,
            Part::ScrollViewHorizontalDraggerBorder,
            value,
        );
        self
    }

    /// Applies sparse inline declarations to the native `ScrollViewVerticalScroller` part.
    #[must_use]
    pub fn vertical_scroller_style(mut self, value: Style) -> Self {
        parts::append(&mut self.parts, Part::ScrollViewVerticalScroller, value);
        self
    }

    /// Applies sparse inline declarations to the native `ScrollViewVerticalSlider` part.
    #[must_use]
    pub fn vertical_slider_style(mut self, value: Style) -> Self {
        parts::append(&mut self.parts, Part::ScrollViewVerticalSlider, value);
        self
    }

    /// Applies sparse inline declarations to the native `ScrollViewVerticalLowButton` part.
    #[must_use]
    pub fn vertical_low_button_style(mut self, value: Style) -> Self {
        parts::append(&mut self.parts, Part::ScrollViewVerticalLowButton, value);
        self
    }

    /// Applies sparse inline declarations to the native `ScrollViewVerticalHighButton` part.
    #[must_use]
    pub fn vertical_high_button_style(mut self, value: Style) -> Self {
        parts::append(&mut self.parts, Part::ScrollViewVerticalHighButton, value);
        self
    }

    /// Applies sparse inline declarations to the native `ScrollViewVerticalTrack` part.
    #[must_use]
    pub fn vertical_track_style(mut self, value: Style) -> Self {
        parts::append(&mut self.parts, Part::ScrollViewVerticalTrack, value);
        self
    }

    /// Applies sparse inline declarations to the native `ScrollViewVerticalDragger` part.
    #[must_use]
    pub fn vertical_dragger_style(mut self, value: Style) -> Self {
        parts::append(&mut self.parts, Part::ScrollViewVerticalDragger, value);
        self
    }

    /// Applies sparse inline declarations to the native `ScrollViewVerticalDraggerBorder` part.
    #[must_use]
    pub fn vertical_dragger_border_style(mut self, value: Style) -> Self {
        parts::append(
            &mut self.parts,
            Part::ScrollViewVerticalDraggerBorder,
            value,
        );
        self
    }

    /// Selects the content layout and scrolling axes.
    #[must_use]
    pub fn mode(mut self, value: ScrollViewMode) -> Self {
        self.mode = Some(value);
        self
    }
    /// Selects how remaining motion crosses nested scroll boundaries.
    #[must_use]
    pub fn nested_interaction(mut self, value: NestedInteraction) -> Self {
        self.nested_interaction = Some(value);
        self
    }
    /// Selects the horizontal scrollbar visibility policy.
    #[must_use]
    pub fn horizontal_scroller_visibility(mut self, value: ScrollerVisibility) -> Self {
        self.horizontal_scroller_visibility = Some(value);
        self
    }
    /// Selects the vertical scrollbar visibility policy.
    #[must_use]
    pub fn vertical_scroller_visibility(mut self, value: ScrollerVisibility) -> Self {
        self.vertical_scroller_visibility = Some(value);
        self
    }
    /// Sets the content displacement in upper-left-origin panel pixels.
    #[must_use]
    pub fn scroll_offset(mut self, value: Vector) -> Self {
        self.scroll_offset = Some(value);
        self
    }
    /// Sets the horizontal keyboard and scrollbar-button step relative to viewport width.
    #[must_use]
    pub fn horizontal_page_size(mut self, value: f32) -> Self {
        self.horizontal_page_size = Some(value);
        self
    }
    /// Sets the vertical keyboard and scrollbar-button step relative to viewport height.
    #[must_use]
    pub fn vertical_page_size(mut self, value: f32) -> Self {
        self.vertical_page_size = Some(value);
        self
    }
    /// Sets mouse-wheel movement in panel pixels per input line.
    #[must_use]
    pub fn mouse_wheel_scroll_size(mut self, value: f32) -> Self {
        self.mouse_wheel_scroll_size = Some(value);
        self
    }
    /// Selects touch behavior at content boundaries.
    #[must_use]
    pub fn touch_scroll_behavior(mut self, value: TouchScrollBehavior) -> Self {
        self.touch_scroll_behavior = Some(value);
        self
    }
    /// Sets the per-second retained fraction of touch-scroll velocity.
    #[must_use]
    pub fn scroll_deceleration_rate(mut self, value: f32) -> Self {
        self.scroll_deceleration_rate = Some(value);
        self
    }
    /// Sets elastic overscroll spring strength.
    #[must_use]
    pub fn elasticity(mut self, value: f32) -> Self {
        self.elasticity = Some(value);
        self
    }
    /// Sets the minimum elastic animation interval in milliseconds.
    #[must_use]
    pub fn elastic_animation_interval(mut self, value: u32) -> Self {
        self.elastic_animation_interval = Some(value);
        self
    }

    pub(crate) fn apply_update(&mut self, value: &Self) {
        self.element.apply_update(&value.element);
        if value.mode.is_some() {
            self.mode = value.mode;
        }
        if value.nested_interaction.is_some() {
            self.nested_interaction = value.nested_interaction;
        }
        if value.horizontal_scroller_visibility.is_some() {
            self.horizontal_scroller_visibility = value.horizontal_scroller_visibility;
        }
        if value.vertical_scroller_visibility.is_some() {
            self.vertical_scroller_visibility = value.vertical_scroller_visibility;
        }
        if value.scroll_offset.is_some() {
            self.scroll_offset = value.scroll_offset;
        }
        if value.horizontal_page_size.is_some() {
            self.horizontal_page_size = value.horizontal_page_size;
        }
        if value.vertical_page_size.is_some() {
            self.vertical_page_size = value.vertical_page_size;
        }
        if value.mouse_wheel_scroll_size.is_some() {
            self.mouse_wheel_scroll_size = value.mouse_wheel_scroll_size;
        }
        if value.touch_scroll_behavior.is_some() {
            self.touch_scroll_behavior = value.touch_scroll_behavior;
        }
        if value.scroll_deceleration_rate.is_some() {
            self.scroll_deceleration_rate = value.scroll_deceleration_rate;
        }
        if value.elasticity.is_some() {
            self.elasticity = value.elasticity;
        }
        if value.elastic_animation_interval.is_some() {
            self.elastic_animation_interval = value.elastic_animation_interval;
        }
        parts::merge(&mut self.parts, &value.parts);
    }
}

impl VisualElementProperties for ScrollView {
    fn visual_element(&self) -> &VisualElement {
        &self.element
    }
    fn visual_element_mut(&mut self) -> &mut VisualElement {
        &mut self.element
    }
}
