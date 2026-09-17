//! Physical object attachments are projections of the shared logical tree.

use battlement::{GameObject, ObjectId};

use crate::host_node::HostNode;

pub(crate) fn extract(roots: &mut [Vec<HostNode>]) -> Vec<HostNode> {
  let mut objects = Vec::new();
  for root in roots {
    self::extract_from(root, &mut objects);
  }
  objects
}

pub(crate) fn snapshot(roots: &[HostNode]) -> Vec<GameObject> {
  let mut objects = Vec::new();
  self::append_objects(roots, None, &mut objects);
  objects
}

fn extract_from(hosts: &mut Vec<HostNode>, objects: &mut Vec<HostNode>) {
  let mut index = 0;
  while index < hosts.len() {
    if hosts[index].scene_root() {
      let mut root = hosts.remove(index);
      self::validate_children(&mut root, objects);
      objects.push(root);
    } else {
      assert!(
        hosts[index].is_ui(),
        "world hosts require an explicit SceneRoot attachment"
      );
      self::extract_from(&mut hosts[index].children, objects);
      index += 1;
    }
  }
}

fn validate_children(host: &mut HostNode, objects: &mut Vec<HostNode>) {
  let mut index = 0;
  while index < host.children.len() {
    assert!(
      !host.children[index].is_ui(),
      "UI beneath a world host requires a UI portal attachment"
    );
    if host.children[index].scene_root() {
      let mut root = host.children.remove(index);
      self::validate_children(&mut root, objects);
      objects.push(root);
    } else {
      self::validate_children(&mut host.children[index], objects);
      index += 1;
    }
  }
}

fn append_objects(hosts: &[HostNode], parent: Option<ObjectId>, objects: &mut Vec<GameObject>) {
  for host in hosts {
    objects.push(
      host
        .object(parent)
        .expect("object attachment has an object adapter"),
    );
    self::append_objects(&host.children, Some(host.object_id), objects);
  }
}
