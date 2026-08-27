use battlement::{Button, Command, ObjectId, ParallelCommandGroup, object_id};

use crate::{
    CALLBACK_BUTTON_ID, CANVAS_ID, COMPLEX_PARTS_TOGGLE_ID, LABEL_COMPONENT_ID, PAGE_ID,
    boolean_components, choice_group_components, complex_part_components, components,
    container_components, design_system, dropdown_components, keyboard_navigation_components,
    part_components, pointer_routing_components, range_components, remaining_event_components,
    routing::Page, scroll_components, slider_components, tab_components, text_field_components,
};

pub(crate) const COMPONENTS_BUTTON_ID: ObjectId =
    object_id!("0e95fbc2-b5e9-4e0f-937f-86aab38b6855");
pub(crate) const INTERACTIONS_BUTTON_ID: ObjectId =
    object_id!("4969d46f-c28c-4e5d-85a0-0321f9931f89");
pub(crate) const HIERARCHY_BUTTON_ID: ObjectId = object_id!("02e0f324-4781-4301-9502-93435d7eea7e");
pub(crate) const ASSETS_BUTTON_ID: ObjectId = object_id!("81083fd8-6546-4a11-8765-32592ede0a3e");
pub(crate) const LAYOUT_BUTTON_ID: ObjectId = object_id!("e100c957-35e6-456c-90ef-5b839424a5cf");
pub(crate) const APPEARANCE_BUTTON_ID: ObjectId =
    object_id!("7237e7ab-178f-438e-a457-0106b1899f6d");
pub(crate) const BACKGROUNDS_BUTTON_ID: ObjectId =
    object_id!("bbcd4be5-d6f3-46c3-8605-56fd4669eda0");
pub(crate) const TRANSFORMS_BUTTON_ID: ObjectId =
    object_id!("416cc818-7d31-4d01-8e39-712be437494b");
pub(crate) const TYPOGRAPHY_BUTTON_ID: ObjectId =
    object_id!("879be431-2981-4aa0-8094-603f106bf067");
pub(crate) const BUTTONS_BUTTON_ID: ObjectId = object_id!("b39e6ba8-aa92-4bc5-b52e-acde2cab1c3a");
pub(crate) const CONTAINERS_BUTTON_ID: ObjectId =
    object_id!("b3858e8c-0b75-4c55-b5f1-d2e0a18cf1ef");
pub(crate) const SCROLL_BUTTON_ID: ObjectId = object_id!("b4baa362-1979-4bff-ae2d-d6a736ab4bb4");
pub(crate) const TABS_BUTTON_ID: ObjectId = object_id!("0dbf590c-b821-4ba5-b4a7-426382a96a16");
pub(crate) const TEXT_FIELDS_BUTTON_ID: ObjectId =
    object_id!("d1810adf-f4fa-4eb7-8b44-46d60e22341d");
pub(crate) const BOOLEAN_CONTROLS_BUTTON_ID: ObjectId =
    object_id!("b95de403-9b85-44a2-aebe-acd016c92fa6");
pub(crate) const CHOICE_GROUPS_BUTTON_ID: ObjectId =
    object_id!("bf246175-3572-4a9d-bd1b-fc91946f035e");
pub(crate) const DROPDOWNS_BUTTON_ID: ObjectId = object_id!("feae3645-8809-42f3-b4f6-00afe473b2f4");
pub(crate) const SLIDERS_BUTTON_ID: ObjectId = object_id!("581694e0-ad9e-477d-a776-478169f39c45");
pub(crate) const RANGES_BUTTON_ID: ObjectId = object_id!("69c28345-59e0-4d2c-a374-b302421d3713");
pub(crate) const PARTS_BUTTON_ID: ObjectId = object_id!("cbb9c6db-5248-48db-b150-029776faf162");
pub(crate) const COMPLEX_PARTS_BUTTON_ID: ObjectId =
    object_id!("8da1d1bd-f7a9-420b-a122-f5c75ca3b295");
pub(crate) const POINTER_ROUTING_BUTTON_ID: ObjectId =
    object_id!("8be537d2-16e7-47ee-9a50-31cd36a13522");
pub(crate) const KEYBOARD_NAVIGATION_BUTTON_ID: ObjectId =
    object_id!("2db08d30-a377-40e6-b9a0-a0036833122a");
pub(crate) const REMAINING_EVENTS_BUTTON_ID: ObjectId =
    object_id!("24100000-0000-4000-8000-000000000001");

pub(crate) fn ids() -> components::NavigationIds {
    components::NavigationIds {
        components: COMPONENTS_BUTTON_ID,
        interactions: INTERACTIONS_BUTTON_ID,
        hierarchy: HIERARCHY_BUTTON_ID,
        assets: ASSETS_BUTTON_ID,
        layout: LAYOUT_BUTTON_ID,
        appearance: APPEARANCE_BUTTON_ID,
        backgrounds: BACKGROUNDS_BUTTON_ID,
        transforms: TRANSFORMS_BUTTON_ID,
        typography: TYPOGRAPHY_BUTTON_ID,
        buttons: BUTTONS_BUTTON_ID,
        containers: CONTAINERS_BUTTON_ID,
        scroll: SCROLL_BUTTON_ID,
        tabs: TABS_BUTTON_ID,
        text_fields: TEXT_FIELDS_BUTTON_ID,
        boolean_controls: BOOLEAN_CONTROLS_BUTTON_ID,
        choice_groups: CHOICE_GROUPS_BUTTON_ID,
        dropdowns: DROPDOWNS_BUTTON_ID,
        sliders: SLIDERS_BUTTON_ID,
        ranges: RANGES_BUTTON_ID,
        parts: PARTS_BUTTON_ID,
        complex_parts: COMPLEX_PARTS_BUTTON_ID,
        pointer_routing: POINTER_ROUTING_BUTTON_ID,
        keyboard_navigation: KEYBOARD_NAVIGATION_BUTTON_ID,
        remaining_events: REMAINING_EVENTS_BUTTON_ID,
    }
}

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
        Page::RemainingEvents => remaining_event_components::page(PAGE_ID, false),
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
            self::active(REMAINING_EVENTS_BUTTON_ID, page == Page::RemainingEvents),
        ]),
    ]
}

fn active(object_id: battlement::ObjectId, selected: bool) -> Command {
    Command::update_visual_element(
        object_id,
        Button::default().style(design_system::navigation_item(selected)),
    )
}
