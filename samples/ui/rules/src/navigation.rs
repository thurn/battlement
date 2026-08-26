use battlement::{Button, Command, ParallelCommandGroup};

use crate::{
    APPEARANCE_BUTTON_ID, ASSETS_BUTTON_ID, BACKGROUNDS_BUTTON_ID, BOOLEAN_CONTROLS_BUTTON_ID,
    BUTTONS_BUTTON_ID, CALLBACK_BUTTON_ID, CANVAS_ID, CHOICE_GROUPS_BUTTON_ID,
    COMPLEX_PARTS_BUTTON_ID, COMPLEX_PARTS_TOGGLE_ID, COMPONENTS_BUTTON_ID, CONTAINERS_BUTTON_ID,
    DROPDOWNS_BUTTON_ID, HIERARCHY_BUTTON_ID, INTERACTIONS_BUTTON_ID,
    KEYBOARD_NAVIGATION_BUTTON_ID, LABEL_COMPONENT_ID, LAYOUT_BUTTON_ID, PAGE_ID, PARTS_BUTTON_ID,
    POINTER_ROUTING_BUTTON_ID, RANGES_BUTTON_ID, SCROLL_BUTTON_ID, SLIDERS_BUTTON_ID,
    TABS_BUTTON_ID, TEXT_FIELDS_BUTTON_ID, TRANSFORMS_BUTTON_ID, TYPOGRAPHY_BUTTON_ID,
    boolean_components, choice_group_components, complex_part_components, components,
    container_components, design_system, dropdown_components, keyboard_navigation_components,
    part_components, pointer_routing_components, range_components, routing::Page,
    scroll_components, slider_components, tab_components, text_field_components,
};

pub(crate) fn commands(page: Page) -> Vec<ParallelCommandGroup<Command>> {
    let content = match page {
        Page::Components => components::components_page(PAGE_ID, LABEL_COMPONENT_ID),
        Page::Interactions => components::interactions_page(PAGE_ID, CALLBACK_BUTTON_ID),
        Page::Hierarchy => components::hierarchy_page(PAGE_ID, &crate::hierarchy_ids()),
        Page::Assets => components::assets_page(PAGE_ID, &crate::asset_ids()),
        Page::Layout => components::layout_page(PAGE_ID, &crate::layout_ids()),
        Page::Appearance => components::appearance_page(PAGE_ID, &crate::appearance_ids()),
        Page::Backgrounds => components::backgrounds_page(PAGE_ID, &crate::background_ids()),
        Page::Transforms => components::transforms_page(PAGE_ID, &crate::transform_ids()),
        Page::Typography => components::typography_page(PAGE_ID),
        Page::Buttons => components::buttons_page(PAGE_ID, &crate::button_ids(), 0),
        Page::Containers => {
            container_components::containers_page(PAGE_ID, &crate::container_ids(), false)
        }
        Page::Scroll => scroll_components::scroll_page(PAGE_ID, &scroll_components::ids()),
        Page::Tabs => tab_components::page(PAGE_ID),
        Page::TextFields => text_field_components::page(PAGE_ID),
        Page::BooleanControls => boolean_components::page(PAGE_ID),
        Page::ChoiceGroups => choice_group_components::page(PAGE_ID),
        Page::Dropdowns => dropdown_components::page(PAGE_ID),
        Page::Sliders => slider_components::page(PAGE_ID),
        Page::Ranges => range_components::page(PAGE_ID),
        Page::Parts => part_components::page(PAGE_ID),
        Page::ComplexParts => {
            complex_part_components::page(PAGE_ID, COMPLEX_PARTS_TOGGLE_ID, false)
        }
        Page::PointerRouting => pointer_routing_components::page(PAGE_ID),
        Page::KeyboardNavigation => keyboard_navigation_components::page(PAGE_ID),
    };
    vec![
        ParallelCommandGroup::new(vec![Command::destroy_visual_element(PAGE_ID)]),
        ParallelCommandGroup::new(vec![
            Command::create_visual_element(CANVAS_ID, content),
            self::active(COMPONENTS_BUTTON_ID, page == Page::Components),
            self::active(INTERACTIONS_BUTTON_ID, page == Page::Interactions),
            self::active(HIERARCHY_BUTTON_ID, page == Page::Hierarchy),
            self::active(ASSETS_BUTTON_ID, page == Page::Assets),
            self::active(LAYOUT_BUTTON_ID, page == Page::Layout),
            self::active(APPEARANCE_BUTTON_ID, page == Page::Appearance),
            self::active(BACKGROUNDS_BUTTON_ID, page == Page::Backgrounds),
            self::active(TRANSFORMS_BUTTON_ID, page == Page::Transforms),
            self::active(TYPOGRAPHY_BUTTON_ID, page == Page::Typography),
            self::active(BUTTONS_BUTTON_ID, page == Page::Buttons),
            self::active(CONTAINERS_BUTTON_ID, page == Page::Containers),
            self::active(SCROLL_BUTTON_ID, page == Page::Scroll),
            self::active(TABS_BUTTON_ID, page == Page::Tabs),
            self::active(TEXT_FIELDS_BUTTON_ID, page == Page::TextFields),
            self::active(BOOLEAN_CONTROLS_BUTTON_ID, page == Page::BooleanControls),
            self::active(CHOICE_GROUPS_BUTTON_ID, page == Page::ChoiceGroups),
            self::active(DROPDOWNS_BUTTON_ID, page == Page::Dropdowns),
            self::active(SLIDERS_BUTTON_ID, page == Page::Sliders),
            self::active(RANGES_BUTTON_ID, page == Page::Ranges),
            self::active(PARTS_BUTTON_ID, page == Page::Parts),
            self::active(COMPLEX_PARTS_BUTTON_ID, page == Page::ComplexParts),
            self::active(POINTER_ROUTING_BUTTON_ID, page == Page::PointerRouting),
            self::active(
                KEYBOARD_NAVIGATION_BUTTON_ID,
                page == Page::KeyboardNavigation,
            ),
        ]),
    ]
}

fn active(object_id: battlement::ObjectId, selected: bool) -> Command {
    Command::update_visual_element(
        object_id,
        Button::default().style(design_system::navigation_item(selected)),
    )
}
