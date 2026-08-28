use battlement::{
  Box as UiBox, Button, Command, CommandBody, Label, ObjectId, ToggleButtonGroup, UiDocument,
  UiNode, VisualElement, VisualElementUpdate,
};
use battlement_fake::battlement_ui_fake::UiWorld;

use crate::reconcile;

#[test]
fn reparenting_precedes_destruction_of_the_old_ancestor() {
  let document_id = ObjectId::new_v4();
  let root_id = ObjectId::new_v4();
  let old_parent_id = ObjectId::new_v4();
  let new_parent_id = ObjectId::new_v4();
  let child_id = ObjectId::new_v4();
  let grandchild_id = ObjectId::new_v4();
  let previous = vec![
    UiNode::new(old_parent_id, VisualElement::new()).child(
      UiNode::new(child_id, VisualElement::new())
        .child(UiNode::new(grandchild_id, Label::new("moved"))),
    ),
    UiNode::new(new_parent_id, VisualElement::new()),
  ];
  let desired = vec![
    UiNode::new(new_parent_id, VisualElement::new()).child(
      UiNode::new(child_id, VisualElement::new())
        .child(UiNode::new(grandchild_id, Label::new("moved"))),
    ),
  ];
  let commands = reconcile::commands(root_id, &previous, &desired);
  assert_eq!(commands.len(), 2);
  self::assert_parent_move(&commands[0], child_id, new_parent_id, 0);
  assert!(matches!(
    commands[1].body,
    CommandBody::VisualElementDestroy(value) if value.object_id == old_parent_id
  ));

  let mut world = self::world(document_id, root_id, previous);
  self::apply(&mut world, commands);
  assert!(world.element(old_parent_id).is_none());
  assert_eq!(world.element(new_parent_id).unwrap().children(), [child_id]);
  assert_eq!(world.element(child_id).unwrap().children(), [grandchild_id]);
}

#[test]
fn nested_reparent_completes_before_its_old_ancestor_is_destroyed() {
  let root_id = ObjectId::new_v4();
  let old_ancestor_id = ObjectId::new_v4();
  let old_parent_id = ObjectId::new_v4();
  let new_parent_id = ObjectId::new_v4();
  let child_id = ObjectId::new_v4();
  let previous = vec![
    UiNode::new(old_ancestor_id, VisualElement::new()).child(
      UiNode::new(old_parent_id, VisualElement::new())
        .child(UiNode::new(child_id, Label::new("moved"))),
    ),
    UiNode::new(new_parent_id, VisualElement::new()),
  ];
  let desired = vec![
    UiNode::new(new_parent_id, VisualElement::new())
      .child(UiNode::new(child_id, Label::new("moved"))),
  ];

  let groups = reconcile::command_groups(root_id, &previous, &desired);
  let move_group = groups
    .iter()
    .position(|group| group.iter().any(|body| self::is_move(body, child_id)))
    .expect("move is present");
  let destroy_group = groups
    .iter()
    .position(|group| {
      group.iter().any(
        |body| matches!(body, CommandBody::VisualElementDestroy(value) if value.object_id == old_ancestor_id),
      )
    })
    .expect("destroy is present");
  assert!(move_group < destroy_group);
}

#[test]
fn disjoint_ends_of_a_move_chain_share_the_first_group() {
  let root_id = ObjectId::new_v4();
  let parents = (0..4).map(|_| ObjectId::new_v4()).collect::<Vec<_>>();
  let children = (0..3).map(|_| ObjectId::new_v4()).collect::<Vec<_>>();
  let previous = parents
    .iter()
    .enumerate()
    .map(|(index, parent_id)| {
      let parent = UiNode::new(*parent_id, VisualElement::new());
      if index < children.len() {
        parent.child(UiNode::new(children[index], Label::new("child")))
      } else {
        parent
      }
    })
    .collect::<Vec<_>>();
  let desired = parents
    .iter()
    .enumerate()
    .map(|(index, parent_id)| {
      let parent = UiNode::new(*parent_id, VisualElement::new());
      if index > 0 {
        parent.child(UiNode::new(children[index - 1], Label::new("child")))
      } else {
        parent
      }
    })
    .collect::<Vec<_>>();

  let groups = reconcile::command_groups(root_id, &previous, &desired);
  assert_eq!(groups.len(), 2);
  assert_eq!(groups[0].len(), 2);
  assert!(self::is_move(&groups[0][0], children[0]));
  assert!(self::is_move(&groups[0][1], children[2]));
  assert!(self::is_move(&groups[1][0], children[1]));
}

#[test]
fn non_tail_reparent_is_one_atomic_indexed_move() {
  let document_id = ObjectId::new_v4();
  let root_id = ObjectId::new_v4();
  let source_id = ObjectId::new_v4();
  let destination_id = ObjectId::new_v4();
  let moved_id = ObjectId::new_v4();
  let tail_id = ObjectId::new_v4();
  let previous = vec![
    UiNode::new(source_id, UiBox::new()).child(UiNode::new(moved_id, Label::new("moved"))),
    UiNode::new(destination_id, UiBox::new()).child(UiNode::new(tail_id, Label::new("tail"))),
  ];
  let desired = vec![
    UiNode::new(source_id, UiBox::new()),
    UiNode::new(destination_id, UiBox::new())
      .child(UiNode::new(moved_id, Label::new("moved")))
      .child(UiNode::new(tail_id, Label::new("tail"))),
  ];
  let commands = reconcile::commands(root_id, &previous, &desired);
  assert_eq!(commands.len(), 1);
  self::assert_parent_move(&commands[0], moved_id, destination_id, 0);

  let mut world = self::world(document_id, root_id, previous);
  self::apply(&mut world, commands);
  assert_eq!(
    world.element(destination_id).unwrap().children(),
    [moved_id, tail_id]
  );
}

#[test]
fn departure_frees_a_full_toggle_group_before_the_arrival() {
  let document_id = ObjectId::new_v4();
  let root_id = ObjectId::new_v4();
  let group_id = ObjectId::new_v4();
  let outside_id = ObjectId::new_v4();
  let outgoing_id = ObjectId::new_v4();
  let incoming_id = ObjectId::new_v4();
  let retained = (0..63)
    .map(|index| UiNode::new(ObjectId::new_v4(), Button::new(format!("Button {index}"))))
    .collect::<Vec<_>>();
  let mut old_group_children = vec![UiNode::new(outgoing_id, Button::new("outgoing"))];
  old_group_children.extend(retained.clone());
  let mut new_group_children = vec![UiNode::new(incoming_id, Button::new("incoming"))];
  new_group_children.extend(retained);
  let group = ToggleButtonGroup::new()
    .allow_empty_selection(true)
    .selected_indices([]);
  let previous = vec![
    UiNode::new(group_id, group.clone()).children(old_group_children),
    UiNode::new(outside_id, UiBox::new()).child(UiNode::new(incoming_id, Button::new("incoming"))),
  ];
  let desired = vec![
    UiNode::new(group_id, group).children(new_group_children),
    UiNode::new(outside_id, UiBox::new()).child(UiNode::new(outgoing_id, Button::new("outgoing"))),
  ];
  let commands = reconcile::commands(root_id, &previous, &desired);
  assert_eq!(commands.len(), 3);
  self::assert_parent_move(&commands[0], outgoing_id, outside_id, 1);
  self::assert_parent_move(&commands[1], incoming_id, group_id, 0);

  let mut world = self::world(document_id, root_id, previous);
  self::apply(&mut world, commands);
  assert_eq!(world.element(group_id).unwrap().children()[0], incoming_id);
  assert_eq!(world.element(group_id).unwrap().children().len(), 64);
  assert_eq!(world.element(outside_id).unwrap().children(), [outgoing_id]);
}

#[test]
fn two_full_toggle_groups_exchange_buttons_through_safe_staging() {
  let document_id = ObjectId::new_v4();
  let root_id = ObjectId::new_v4();
  let first_group_id = ObjectId::new_v4();
  let second_group_id = ObjectId::new_v4();
  let first_outgoing_id = ObjectId::new_v4();
  let second_outgoing_id = ObjectId::new_v4();
  let first_retained = (0..63)
    .map(|index| UiNode::new(ObjectId::new_v4(), Button::new(format!("First {index}"))))
    .collect::<Vec<_>>();
  let second_retained = (0..63)
    .map(|index| UiNode::new(ObjectId::new_v4(), Button::new(format!("Second {index}"))))
    .collect::<Vec<_>>();
  let mut old_first = vec![UiNode::new(first_outgoing_id, Button::new("first"))];
  old_first.extend(first_retained.clone());
  let mut old_second = vec![UiNode::new(second_outgoing_id, Button::new("second"))];
  old_second.extend(second_retained.clone());
  let mut new_first = vec![UiNode::new(second_outgoing_id, Button::new("second"))];
  new_first.extend(first_retained);
  let mut new_second = vec![UiNode::new(first_outgoing_id, Button::new("first"))];
  new_second.extend(second_retained);
  let group = ToggleButtonGroup::new()
    .allow_empty_selection(true)
    .selected_indices([]);
  let previous = vec![
    UiNode::new(first_group_id, group.clone()).children(old_first),
    UiNode::new(second_group_id, group.clone()).children(old_second),
  ];
  let desired = vec![
    UiNode::new(first_group_id, group.clone()).children(new_first),
    UiNode::new(second_group_id, group).children(new_second),
  ];
  let commands = reconcile::commands(root_id, &previous, &desired);
  assert_eq!(commands.len(), 5);
  self::assert_parent_move(&commands[0], second_outgoing_id, root_id, 2);
  self::assert_parent_move(&commands[1], first_outgoing_id, second_group_id, 0);
  self::assert_parent_move(&commands[2], second_outgoing_id, first_group_id, 0);

  let mut world = self::world(document_id, root_id, previous);
  self::apply(&mut world, commands);
  assert_eq!(world.element(root_id).unwrap().children().len(), 2);
  assert_eq!(
    world.element(first_group_id).unwrap().children()[0],
    second_outgoing_id
  );
  assert_eq!(
    world.element(second_group_id).unwrap().children()[0],
    first_outgoing_id
  );
}

fn assert_parent_move(command: &Command, target: ObjectId, parent: ObjectId, index: u32) {
  assert!(matches!(
    &command.body,
    CommandBody::VisualElementUpdate(value)
      if matches!(
        value.as_ref(),
        VisualElementUpdate::Parent {
          object_id,
          parent_id,
          child_index: Some(child_index),
        } if *object_id == target && *parent_id == parent && *child_index == index
      )
  ));
}

fn is_move(body: &CommandBody, target: ObjectId) -> bool {
  matches!(
    body,
    CommandBody::VisualElementUpdate(value)
      if matches!(
        value.as_ref(),
        VisualElementUpdate::Parent { object_id, .. } if *object_id == target
      )
  )
}

fn world(document_id: ObjectId, root_id: ObjectId, children: Vec<UiNode>) -> UiWorld {
  let mut document = UiDocument::with_root_id(document_id, root_id);
  document.children = children;
  let mut world = UiWorld::default();
  world
    .replace(vec![document])
    .expect("initial tree is valid");
  world
}

fn apply(world: &mut UiWorld, commands: Vec<Command>) {
  for command in commands {
    match command.body {
      CommandBody::VisualElementCreate(value) => world.create(*value).unwrap(),
      CommandBody::VisualElementUpdate(value) => world.update(*value).unwrap(),
      CommandBody::VisualElementDestroy(value) => world.destroy(value.object_id).unwrap(),
      _ => panic!("unexpected hierarchy command"),
    }
  }
}
