use battlement::{
  BackgroundPositionKeyword, BackgroundRepeatMode, BackgroundSize, BackgroundSource, Color, Cursor,
  Display, FlexDirection, FlexWrap, FocusDirection, FocusEvent, GameObjectKind, ImageSource,
  KeyEvent, KeyModifier, KeyModifiers, NavigationDirection, NavigationEvent, NavigationMoveEvent,
  ObjectId, Overflow, PanelPoint, PanelRenderMode, PhysicalKey, PointerButton, PointerButtonEvent,
  PointerCaptureEvent, PointerType, Position, Prop, Rect, StyleValue, TextGenerator,
  TransitionEvent, TransitionProperty, UiElement, UiElementKind, UiEvent, UiEventBody, Vector,
  Visibility, object_id,
};
use battlement_fake::{
  assets::FakeAssetCatalog,
  client::{FakeClient, ui::UiClient},
};
use battlement_rules::asset_catalog::ui::{self as ui_assets, assets};

const INTERACTIONS_BUTTON_ID: ObjectId = object_id!("4969d46f-c28c-4e5d-85a0-0321f9931f89");
const CALLBACK_BUTTON_ID: ObjectId = object_id!("7e0b078e-13d9-43c3-a491-84178e157fb2");
const WORLD_SPACE_BUTTON_ID: ObjectId = object_id!("27100000-0000-4000-8000-000000000000");
const RENDER_MODE_DETAILS_BUTTON_ID: ObjectId = object_id!("26100000-0000-4000-8000-000000000004");
const RENDER_MODE_DETAILS_ID: ObjectId = object_id!("26100000-0000-4000-8000-000000000005");
const COVERAGE_BUTTON_ID: ObjectId = object_id!("28100000-0000-4000-8000-000000000000");
const COVERAGE_BACK_ID: ObjectId = object_id!("28000000-0000-4000-8000-000000000100");
const COVERAGE_GROUP_IDS: [ObjectId; 7] = [
  object_id!("28000000-0000-4000-8000-000000000101"),
  object_id!("28000000-0000-4000-8000-000000000102"),
  object_id!("28000000-0000-4000-8000-000000000103"),
  object_id!("28000000-0000-4000-8000-000000000104"),
  object_id!("28000000-0000-4000-8000-000000000105"),
  object_id!("28000000-0000-4000-8000-000000000106"),
  object_id!("28000000-0000-4000-8000-000000000107"),
];
const WORLD_DOCUMENT_ID: ObjectId = object_id!("27100000-0000-4000-8000-000000000001");
const WORLD_BUTTON_ID: ObjectId = object_id!("27100000-0000-4000-8000-000000000003");
const WORLD_STATUS_ID: ObjectId = object_id!("27100000-0000-4000-8000-000000000004");
const LABEL_COMPONENT_ID: ObjectId = object_id!("5768cfee-a137-49c0-b76c-5ebfa6c227c1");
const GREETING_ID: ObjectId = object_id!("2d8ac61c-49bb-43ce-9656-faa11238351f");
const TRANSIENT_CARD_ID: ObjectId = object_id!("45a1a00c-2624-4e40-b675-3c5f59c62f53");
const COMPONENTS_BUTTON_ID: ObjectId = object_id!("0e95fbc2-b5e9-4e0f-937f-86aab38b6855");
const HIERARCHY_BUTTON_ID: ObjectId = object_id!("02e0f324-4781-4301-9502-93435d7eea7e");
const HIERARCHY_BRANCH_ID: ObjectId = object_id!("53e9582f-36c9-47fb-91c7-a6f7c7b3dd50");
const HIERARCHY_PRIMARY_ID: ObjectId = object_id!("f48e306d-ec3a-4881-abeb-ae685b0bb956");
const HIERARCHY_SECONDARY_ID: ObjectId = object_id!("45ee68d7-72bf-4d1b-bba3-e0a2834c5f06");
const HIERARCHY_MOVABLE_ID: ObjectId = object_id!("0121bbc8-ceb1-42ea-bea0-a7601543851e");
const HIERARCHY_DESTINATION_ID: ObjectId = object_id!("98ec6daa-7faa-41aa-a157-afb9beca284d");
const HIERARCHY_ACTION_ID: ObjectId = object_id!("51e73f5f-1af1-4f54-bcf6-288cde0f45ee");
const PAGE_ID: ObjectId = object_id!("28951e4f-6f61-491e-8548-84b9d4a356e4");
const ASSETS_BUTTON_ID: ObjectId = object_id!("81083fd8-6546-4a11-8765-32592ede0a3e");
const TEXTURE_IMAGE_ID: ObjectId = object_id!("d4e9b4cf-cb57-4fd7-8d92-ee8420b095c4");
const SPRITE_IMAGE_ID: ObjectId = object_id!("0665cd59-2629-4ded-92eb-65413a5374ad");
const VECTOR_IMAGE_ID: ObjectId = object_id!("f48633c5-ca86-4c1c-a907-ae2eafa639ac");
const RENDER_IMAGE_ID: ObjectId = object_id!("41ce020f-64c1-4b6a-b8ee-b0d15115e958");
const SWITCHED_IMAGE_ID: ObjectId = object_id!("b64232bb-97c1-4a00-95cf-01b8bc8a27f8");
const ACTIVE_ADDRESS_ID: ObjectId = object_id!("4e0386da-f6ed-46fe-be94-5b1fd9f056e2");
const SOURCE_SWITCH_ID: ObjectId = object_id!("6a383965-6837-4898-946e-5aa76d49f193");
const LAYOUT_BUTTON_ID: ObjectId = object_id!("e100c957-35e6-456c-90ef-5b839424a5cf");
const LAYOUT_PLAYGROUND_ID: ObjectId = object_id!("419ee1dc-73f8-4968-a9ad-552d38592398");
const LAYOUT_ALPHA_ID: ObjectId = object_id!("9d2ae871-2ce9-4707-85a7-bc8263cb0e37");
const LAYOUT_GAMMA_ID: ObjectId = object_id!("3dbc8a14-b4b2-42b5-83f0-f83f564dadc4");
const LAYOUT_ACTION_ID: ObjectId = object_id!("274aa2af-5b70-4079-a260-25fadd46f339");
const APPEARANCE_BUTTON_ID: ObjectId = object_id!("7237e7ab-178f-438e-a457-0106b1899f6d");
const APPEARANCE_SLICED_ID: ObjectId = object_id!("2b6868b0-042c-4258-b7fe-d594c788cf5d");
const APPEARANCE_CLIPPED_ID: ObjectId = object_id!("1da43df8-2db8-4975-b6a7-2f84abb9f5ae");
const APPEARANCE_HIDDEN_ID: ObjectId = object_id!("3658659b-69e6-4c1e-bf96-6ba1473d0ac2");
const APPEARANCE_REMOVED_ID: ObjectId = object_id!("f2360cdc-c121-41af-8ae2-486eb817669f");
const APPEARANCE_ACTION_ID: ObjectId = object_id!("876cec21-9d24-40e3-ba85-f27e0262112c");
const BACKGROUNDS_BUTTON_ID: ObjectId = object_id!("bbcd4be5-d6f3-46c3-8605-56fd4669eda0");
const BACKGROUND_TEXTURE_ID: ObjectId = object_id!("f7220234-b7ae-4dc1-adda-8b360959c718");
const BACKGROUND_SPRITE_ID: ObjectId = object_id!("e8209c63-12d6-4dcb-b225-2418727d02d6");
const BACKGROUND_VECTOR_ID: ObjectId = object_id!("f0612329-0788-46ad-a2cb-62243fd041c3");
const BACKGROUND_RENDER_ID: ObjectId = object_id!("3479b397-ae71-4b0e-8cdf-d43fd68449db");
const BACKGROUND_ACTION_ID: ObjectId = object_id!("62f5c910-67fa-4eb1-b54b-040022f63ab7");
const TRANSFORMS_BUTTON_ID: ObjectId = object_id!("416cc818-7d31-4d01-8e39-712be437494b");
const TRANSFORM_TARGET_ID: ObjectId = object_id!("066af04d-a6d7-46e1-b7ac-a62001a90239");
const TRANSFORM_STATUS_ID: ObjectId = object_id!("6274737d-8539-4991-ad00-a20b3a5a9fc2");
const TRANSFORM_ACTION_ID: ObjectId = object_id!("6277a6b7-b774-4302-9d06-81c1991c214f");
const TYPOGRAPHY_BUTTON_ID: ObjectId = object_id!("879be431-2981-4aa0-8094-603f106bf067");
const COMPLEX_PARTS_BUTTON_ID: ObjectId = object_id!("8da1d1bd-f7a9-420b-a122-f5c75ca3b295");
const COMPLEX_PARTS_TOGGLE_ID: ObjectId = object_id!("9321c5a3-9b82-462d-9f68-26da56edcbb7");
const COMPLEX_PARTS_SLIDER_ID: ObjectId = object_id!("0121421b-c595-4eb8-9689-88e02dd62669");
const COMPLEX_PARTS_TITLE_ID: ObjectId = object_id!("139c41bc-e97b-4da9-9f70-1c58f9136953");
const CONTAINERS_BUTTON_ID: ObjectId = object_id!("b3858e8c-0b75-4c55-b5f1-d2e0a18cf1ef");
const TITLED_GROUP_ID: ObjectId = object_id!("9ab84d41-dd5f-4202-a62b-da4643222ac8");
const EMPTY_GROUP_ID: ObjectId = object_id!("05acfc99-c92d-46cd-93cd-3738ff025e62");
const DYNAMIC_GROUP_ID: ObjectId = object_id!("3a9d57df-b920-4ec3-b170-3afbc6ce0494");
const DYNAMIC_GROUP_CHILD_ID: ObjectId = object_id!("7ceac51e-b580-4e67-b995-191216cbff88");
const DYNAMIC_GROUP_ACTION_ID: ObjectId = object_id!("c21e285f-6999-4df7-8a6b-559339520962");
const POPUP_WINDOW_ID: ObjectId = object_id!("71347582-7a69-4270-a76f-c4c25546e086");
const SCROLL_BUTTON_ID: ObjectId = object_id!("b4baa362-1979-4bff-ae2d-d6a736ab4bb4");
const PRIMARY_SCROLL_ID: ObjectId = object_id!("d24fec17-cb8a-4b9c-a604-da4113d6ef9b");
const CONTROLLED_SCROLLER_ID: ObjectId = object_id!("df12adf3-3a6c-4900-bb15-1f53117f1a8e");
const SCROLL_STATUS_ID: ObjectId = object_id!("898a986b-893d-48d8-bd68-5d39ef58c086");
const SCROLLER_STATUS_ID: ObjectId = object_id!("a7338149-f968-40a3-9bdd-e7640546e2fe");
const TABS_BUTTON_ID: ObjectId = object_id!("0dbf590c-b821-4ba5-b4a7-426382a96a16");
const TEXT_FIELDS_BUTTON_ID: ObjectId = object_id!("d1810adf-f4fa-4eb7-8b44-46d60e22341d");
const TAB_VIEW_ID: ObjectId = object_id!("aa1bd60d-71e5-4f3a-a7ba-13f456621b9c");
const BOARD_TAB_ID: ObjectId = object_id!("e7491a26-c97e-4668-9b72-0aba2f8920c1");
const NOTES_TAB_ID: ObjectId = object_id!("1560af93-b7eb-489e-983b-768747b9db49");
const LOADOUT_TAB_ID: ObjectId = object_id!("9fca8e31-3f73-4245-8fbf-523b1094ef0a");
const TIMELINE_TAB_ID: ObjectId = object_id!("d3f27972-0998-4e83-ad01-3125540ad95a");
const SIGNAL_TAB_ID: ObjectId = object_id!("abbb5697-bb75-4f18-85ca-f5bb706dc59f");
const TAB_STATUS_ID: ObjectId = object_id!("752743e9-cb89-4148-ad40-e5076f78f6e1");
const ACCEPTED_TEXT_ID: ObjectId = object_id!("fd496f77-d46e-4bf9-8f5e-5cba8229d94f");
const NORMALIZED_TEXT_ID: ObjectId = object_id!("df0c6d77-9ff1-40cb-8ae3-a01353df5c73");
const REJECTED_TEXT_ID: ObjectId = object_id!("c20ac846-5730-48ab-89ea-9c943d5e385b");
const TEXT_STATUS_ID: ObjectId = object_id!("8a83987f-581f-4f32-8ce8-e0a99c70174d");
const TEXT_DRAFT_ID: ObjectId = object_id!("f93c739b-a044-44ed-89de-05a343937df6");
const TEXT_COMMITTED_ID: ObjectId = object_id!("b6ce5ac8-1923-4470-a2a1-b9d9ad8fe7d1");
const BOOLEAN_CONTROLS_BUTTON_ID: ObjectId = object_id!("b95de403-9b85-44a2-aebe-acd016c92fa6");
const ACCEPTED_TOGGLE_ID: ObjectId = object_id!("93ecbf8e-5be7-4087-b292-6f68903436c1");
const REJECTED_TOGGLE_ID: ObjectId = object_id!("d18a9439-619d-4ca8-ac58-d82d999b3bf1");
const ACCEPTED_RADIO_ID: ObjectId = object_id!("bfe98ac4-cfa5-4f56-8e6a-253837c66c05");
const REJECTED_RADIO_ID: ObjectId = object_id!("174b5d07-dd4f-4fe6-a264-3863ea6bc318");
const BOOLEAN_STATUS_ID: ObjectId = object_id!("1745a91d-06f7-460c-bd3b-bd1f432332c0");
const BOOLEAN_HISTORY_ID: ObjectId = object_id!("65cba5dd-fc33-49e3-a636-a6d4fc59e73d");
const CHOICE_GROUPS_BUTTON_ID: ObjectId = object_id!("bf246175-3572-4a9d-bd1b-fc91946f035e");
const FORMATION_ID: ObjectId = object_id!("34ee78d0-a503-4d77-b61d-bbd86cf39e41");
const FILTER_ID: ObjectId = object_id!("17805693-79d9-46ac-97db-1694047f8a9e");
const FILTER_SUMMARY_ID: ObjectId = object_id!("01d7f042-cdae-4e9c-8020-817d5e83ae18");
const CHOICE_STATUS_ID: ObjectId = object_id!("6553e506-c92a-4f50-995e-58380393bb6f");
const CHOICE_HISTORY_ID: ObjectId = object_id!("84a701b8-cce9-4165-9637-9b7a24856d7d");
const DROPDOWNS_BUTTON_ID: ObjectId = object_id!("feae3645-8809-42f3-b4f6-00afe473b2f4");
const THEME_DROPDOWN_ID: ObjectId = object_id!("ae31830c-672e-4e99-b409-02ba8383d452");
const LOADOUT_DROPDOWN_ID: ObjectId = object_id!("2d5a2b47-1e52-45c2-b454-a178157133f0");
const CLEAR_LOADOUT_ID: ObjectId = object_id!("c1834769-2048-40f4-953d-0268561883b5");
const THEME_SUMMARY_ID: ObjectId = object_id!("727e62a9-ebce-48cb-876f-20f86784b8cc");
const LOADOUT_SUMMARY_ID: ObjectId = object_id!("2e8fdeee-8310-4173-9daf-87905506c15c");
const DROPDOWN_STATUS_ID: ObjectId = object_id!("4c864234-bb34-43fd-bf0a-634a44111156");
const DROPDOWN_HISTORY_ID: ObjectId = object_id!("948fd3dd-ac76-4761-8831-e9abb02db7d5");
const SLIDERS_BUTTON_ID: ObjectId = object_id!("581694e0-ad9e-477d-a776-478169f39c45");
const CONTINUOUS_SLIDER_ID: ObjectId = object_id!("08e45324-236a-469d-a4f8-f2f40922a9b8");
const STEPPED_SLIDER_ID: ObjectId = object_id!("c1ad6472-f8ae-40cb-9d21-60f6e544db53");
const CONTINUOUS_VALUE_ID: ObjectId = object_id!("27420acd-df31-45fa-99c2-4bf6bde37f7e");
const STEPPED_VALUE_ID: ObjectId = object_id!("12988004-2b5a-4d6d-9eb6-4960f656394b");
const SLIDER_LIVE_STATUS_ID: ObjectId = object_id!("13ba592a-5f70-4a64-892a-21a919479e5d");
const SLIDER_COMMIT_STATUS_ID: ObjectId = object_id!("0d1be49a-b9fc-437d-8d48-d2724e7efe1f");
const RANGES_BUTTON_ID: ObjectId = object_id!("69c28345-59e0-4d2c-a374-b302421d3713");
const RESOURCE_RANGE_ID: ObjectId = object_id!("4be5cd99-a70d-4dca-af82-57dc73f91eea");
const RANGE_STATUS_ID: ObjectId = object_id!("cb0e1e49-857d-4a3b-a95e-f0dce69060d8");
const POINTER_ROUTING_BUTTON_ID: ObjectId = object_id!("8be537d2-16e7-47ee-9a50-31cd36a13522");
const POINTER_TARGET_ID: ObjectId = object_id!("22100000-0000-4000-8000-000000000003");
const POINTER_PAYLOAD_ID: ObjectId = object_id!("22100000-0000-4000-8000-000000000004");
const POINTER_CAPTURE_ID: ObjectId = object_id!("22100000-0000-4000-8000-000000000005");
const KEYBOARD_NAVIGATION_BUTTON_ID: ObjectId = object_id!("2db08d30-a377-40e6-b9a0-a0036833122a");
const KEYBOARD_ALPHA_ID: ObjectId = object_id!("23100000-0000-4000-8000-000000000001");
const KEYBOARD_BRAVO_ID: ObjectId = object_id!("23100000-0000-4000-8000-000000000002");
const KEYBOARD_INSPECTOR_ID: ObjectId = object_id!("23100000-0000-4000-8000-000000000005");
const REMAINING_EVENTS_BUTTON_ID: ObjectId = object_id!("24100000-0000-4000-8000-000000000001");
const REMAINING_LINK_ID: ObjectId = object_id!("24100000-0000-4000-8000-000000000002");
const REMAINING_LINK_INSPECTOR_ID: ObjectId = object_id!("24100000-0000-4000-8000-000000000003");
const REMAINING_TARGET_ID: ObjectId = object_id!("24100000-0000-4000-8000-000000000004");
const REMAINING_LIFECYCLE_ID: ObjectId = object_id!("24100000-0000-4000-8000-000000000005");
const REMAINING_ACTION_ID: ObjectId = object_id!("24100000-0000-4000-8000-000000000006");
const REMAINING_TARGET_LABEL_ID: ObjectId = object_id!("24100000-0000-4000-8000-000000000007");
const ACTIONS_BUTTON_ID: ObjectId = object_id!("25100000-0000-4000-8000-000000000000");
const ACTION_RUN_ID: ObjectId = object_id!("25100000-0000-4000-8000-000000000001");
const ACTION_SCROLL_TARGET_ID: ObjectId = object_id!("25100000-0000-4000-8000-000000000003");
const ACTION_SELECTABLE_ID: ObjectId = object_id!("25100000-0000-4000-8000-000000000004");
const ACTION_STATUS_ID: ObjectId = object_id!("25100000-0000-4000-8000-000000000005");
const ACTION_ACCEPTED_ID: ObjectId = object_id!("25100000-0000-4000-8000-000000000006");
const ACTION_REJECTED_ID: ObjectId = object_id!("25100000-0000-4000-8000-000000000007");
const ACTION_DRAFT_ID: ObjectId = object_id!("25100000-0000-4000-8000-000000000008");
const ACTION_DRAG_ID: ObjectId = object_id!("25100000-0000-4000-8000-000000000009");
const ACTION_CLEANUP_ID: ObjectId = object_id!("25100000-0000-4000-8000-00000000000a");
const ACTION_CONTROL_STATUS_ID: ObjectId = object_id!("25100000-0000-4000-8000-00000000000b");
const RENDER_MODES_BUTTON_ID: ObjectId = object_id!("26100000-0000-4000-8000-000000000000");

#[path = "ui_commands/control_flows.rs"]
mod control_flows;
#[path = "ui_commands/event_flows.rs"]
mod event_flows;
#[path = "ui_commands/hierarchy_presentation_flows.rs"]
mod hierarchy_presentation_flows;

fn sample_assets() -> FakeAssetCatalog {
  let mut catalog = FakeAssetCatalog::new();
  catalog.add_scene(ui_assets::CONTENT.clone());
  catalog.add_texture(assets::TEXTURE.clone());
  catalog.add_sprite(assets::SPRITE.clone());
  catalog.add_vector_image(assets::VECTOR.clone());
  catalog.add_render_texture(assets::RENDER_TEXTURE.clone());
  catalog.add_texture(assets::CURSOR.clone());
  catalog.add_ui_font(assets::UI_FONT.clone());
  catalog
}
