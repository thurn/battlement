use std::collections::{HashMap, HashSet};

use battlement::{
  AccessibilityActionSet, AccessibilityNodeSnapshot, AccessibilitySnapshot, ObjectId, SemanticRole,
};

use crate::semantics::SemanticMembership;
use crate::{
  element_ref::AttachmentSet,
  overlay::OverlayReference,
  render::{RenderPosition, RenderTree},
  semantic_validation,
  semantics::{AccessibleDescription, AccessibleName, SemanticProps, SemanticVisibility},
};

struct Draft<'a> {
  id: ObjectId,
  parent: Option<ObjectId>,
  children: Vec<ObjectId>,
  semantic: &'a SemanticProps,
  position: &'a RenderPosition,
}

#[derive(Clone, Copy)]
struct Source<'a> {
  id: ObjectId,
  semantic: &'a SemanticProps,
  position: &'a RenderPosition,
}

pub(crate) fn build(
  trees: &[RenderTree],
  runtime_id: u64,
  attachments: &AttachmentSet,
  commit_sequence: u64,
) -> AccessibilitySnapshot {
  let mut drafts = Vec::new();
  let mut roots = Vec::new();
  for tree in trees {
    collect(tree, None, &mut drafts, &mut roots);
  }
  let mut sources = HashMap::new();
  for tree in trees {
    collect_sources(tree, &mut sources);
  }
  if trees.iter().any(has_semantic_declaration) {
    validate_modals(trees);
  }
  for tree in trees {
    validate_tree_declarations(tree);
  }
  let indexes = drafts
    .iter()
    .enumerate()
    .map(|(index, draft)| (draft.id, index))
    .collect::<HashMap<_, _>>();
  for index in 0..drafts.len() {
    if let Some(parent) = drafts[index].parent {
      let child_id = drafts[index].id;
      let parent_index = indexes
        .get(&parent)
        .copied()
        .expect("semantic parent must be exposed");
      drafts[parent_index].children.push(child_id);
    }
  }
  validate_memberships(&drafts, runtime_id, attachments, &indexes, &roots);
  let mut nodes = Vec::with_capacity(drafts.len());
  for draft in &drafts {
    let mut resolving = HashSet::new();
    let label = draft.semantic.name.as_ref().map(|name| {
      resolve_name(
        name,
        Source {
          id: draft.id,
          semantic: draft.semantic,
          position: draft.position,
        },
        &sources,
        runtime_id,
        attachments,
        &mut resolving,
      )
    });
    let hint = draft.semantic.description.as_ref().map(|description| {
      resolve_description(
        description,
        Source {
          id: draft.id,
          semantic: draft.semantic,
          position: draft.position,
        },
        &sources,
        runtime_id,
        attachments,
        &mut HashSet::new(),
      )
    });
    if requires_name(draft.semantic.role) {
      assert!(
        label.as_ref().is_some_and(|value| !value.is_empty()),
        "accessible role {:?} requires a nonempty name",
        draft.semantic.role
      );
    }
    nodes.push(AccessibilityNodeSnapshot {
      object_id: draft.id,
      parent_id: draft.parent,
      children: draft.children.clone(),
      role: draft.semantic.role,
      label,
      hint,
      state: draft.semantic.state.clone(),
      value: draft.semantic.value.clone(),
      actions: draft.semantic.actions.clone(),
      heading_level: draft.semantic.heading_level,
      scroll_axis: draft.semantic.scroll_axis,
    });
  }
  AccessibilitySnapshot {
    commit_sequence,
    roots,
    nodes,
  }
}

fn collect_sources<'a>(tree: &'a RenderTree, sources: &mut HashMap<ObjectId, Source<'a>>) {
  for position in &tree.positions {
    let Some(semantic) = &position.semantic else {
      collect_sources(&position.children, sources);
      continue;
    };
    if semantic.visibility == SemanticVisibility::Hidden {
      continue;
    }
    if let Some(host) = &position.host {
      assert!(
        sources
          .insert(
            host.object_id,
            Source {
              id: host.object_id,
              semantic,
              position,
            },
          )
          .is_none(),
        "semantic host identity must be unique"
      );
    }
    collect_sources(&position.children, sources);
  }
}

fn collect<'a>(
  tree: &'a RenderTree,
  semantic_parent: Option<ObjectId>,
  drafts: &mut Vec<Draft<'a>>,
  roots: &mut Vec<ObjectId>,
) {
  for position in &tree.positions {
    if position
      .semantic
      .as_ref()
      .is_some_and(|semantic| semantic.visibility == SemanticVisibility::Hidden)
    {
      continue;
    }
    let mut child_parent = semantic_parent;
    if let (Some(host), Some(semantic)) = (&position.host, &position.semantic)
      && semantic.visibility == SemanticVisibility::Exposed
    {
      if semantic_parent.is_none() {
        roots.push(host.object_id);
      }
      child_parent = Some(host.object_id);
      drafts.push(Draft {
        id: host.object_id,
        parent: semantic_parent,
        children: Vec::new(),
        semantic,
        position,
      });
    }
    collect(&position.children, child_parent, drafts, roots);
  }
}

fn resolve_name(
  name: &AccessibleName,
  owner: Source<'_>,
  sources: &HashMap<ObjectId, Source<'_>>,
  runtime_id: u64,
  attachments: &AttachmentSet,
  resolving: &mut HashSet<ObjectId>,
) -> String {
  assert!(
    resolving.insert(owner.id),
    "cyclic accessible name reference"
  );
  let result = match name {
    AccessibleName::Text(value) => value.resolved(),
    AccessibleName::LabelledBy(element_ref) => {
      let target = attachments.reference_target(runtime_id, element_ref);
      let target = sources
        .get(&target)
        .copied()
        .expect("accessible name source must be a live exposed or name-source host");
      let name = target
        .semantic
        .name
        .as_ref()
        .expect("accessible name source must declare text");
      resolve_name(name, target, sources, runtime_id, attachments, resolving)
    }
    AccessibleName::Contents => contents_text(&owner.position.children),
  };
  resolving.remove(&owner.id);
  normalize(&result)
}

fn resolve_description(
  description: &AccessibleDescription,
  owner: Source<'_>,
  sources: &HashMap<ObjectId, Source<'_>>,
  runtime_id: u64,
  attachments: &AttachmentSet,
  resolving: &mut HashSet<ObjectId>,
) -> String {
  match description {
    AccessibleDescription::Text(value) => value.resolved(),
    AccessibleDescription::DescribedBy(element_ref) => {
      let target = attachments.reference_target(runtime_id, element_ref);
      assert!(target != owner.id, "a semantic node cannot describe itself");
      let target = sources
        .get(&target)
        .copied()
        .expect("accessible description source must be a live exposed or name-source host");
      let name = target
        .semantic
        .name
        .as_ref()
        .expect("accessible description source must declare text");
      resolve_name(name, target, sources, runtime_id, attachments, resolving)
    }
  }
}

fn contents_text(tree: &RenderTree) -> String {
  let mut fragments = Vec::new();
  append_contents(tree, &mut fragments);
  normalize(&fragments.join(" "))
}

fn append_contents(tree: &RenderTree, fragments: &mut Vec<String>) {
  for position in &tree.positions {
    let Some(semantic) = &position.semantic else {
      append_contents(&position.children, fragments);
      continue;
    };
    if semantic.visibility == SemanticVisibility::Hidden {
      continue;
    }
    if semantic.role == SemanticRole::StaticText
      && let Some(AccessibleName::Text(value)) = &semantic.name
    {
      fragments.push(value.resolved());
    }
    if !is_actionable(semantic) {
      append_contents(&position.children, fragments);
    }
  }
}

fn validate_tree_declarations(tree: &RenderTree) {
  for position in &tree.positions {
    if position.host.is_some()
      && let Some(semantic) = &position.semantic
    {
      semantic_validation::validate(
        semantic,
        matches!(
          position.overlay_reference,
          Some(OverlayReference::Modal { .. })
        ),
      );
    }
    validate_tree_declarations(&position.children);
  }
}

fn validate_modals(trees: &[RenderTree]) {
  fn walk(tree: &RenderTree) {
    for position in &tree.positions {
      if matches!(
        position.overlay_reference,
        Some(OverlayReference::Modal { .. })
      ) {
        assert!(
          position.semantic.as_ref().is_some_and(|semantic| {
            semantic.role == SemanticRole::Dialog
              && semantic.visibility == SemanticVisibility::Exposed
          }),
          "each modal wrapper requires one exposed dialog semantic declaration"
        );
      }
      walk(&position.children);
    }
  }
  for tree in trees {
    walk(tree);
  }
}

fn has_semantic_declaration(tree: &RenderTree) -> bool {
  tree
    .positions
    .iter()
    .any(|position| position.semantic.is_some() || has_semantic_declaration(&position.children))
}

fn validate_memberships(
  drafts: &[Draft<'_>],
  runtime_id: u64,
  attachments: &AttachmentSet,
  indexes: &HashMap<ObjectId, usize>,
  _roots: &[ObjectId],
) {
  let mut selected_radios = HashMap::<ObjectId, usize>::new();
  let mut selected_tabs = HashMap::<ObjectId, usize>::new();
  let mut tab_panels = HashMap::<ObjectId, usize>::new();
  for draft in drafts {
    let Some(membership) = &draft.semantic.membership else {
      continue;
    };
    let (reference, expected) = match membership {
      SemanticMembership::Radio(reference) => (reference, SemanticRole::RadioGroup),
      SemanticMembership::Tab(reference) | SemanticMembership::TabPanel(reference) => {
        (reference, SemanticRole::TabList)
      }
    };
    let owner = attachments.reference_target(runtime_id, reference);
    let owner_draft = indexes
      .get(&owner)
      .map(|index| &drafts[*index])
      .expect("semantic membership handle must target a live exposed host");
    assert_eq!(
      owner_draft.semantic.role, expected,
      "semantic membership handle targets the wrong role"
    );
    if !matches!(membership, SemanticMembership::TabPanel(_)) {
      let nearest = nearest_role(draft.parent, expected, drafts, indexes);
      assert_eq!(
        nearest,
        Some(owner),
        "radio and tab membership must match the nearest semantic container"
      );
    } else {
      assert_eq!(
        canonical_root(draft.id, drafts, indexes),
        canonical_root(owner, drafts, indexes),
        "tab panel membership cannot cross a canonical semantic root"
      );
    }
    if draft.semantic.state.selected == Some(true) {
      let counts = match membership {
        SemanticMembership::Radio(_) => &mut selected_radios,
        SemanticMembership::Tab(_) => {
          assert!(
            !draft.semantic.state.disabled,
            "selected tab must be enabled"
          );
          &mut selected_tabs
        }
        SemanticMembership::TabPanel(_) => continue,
      };
      *counts.entry(owner).or_default() += 1;
    }
    if matches!(membership, SemanticMembership::TabPanel(_)) {
      *tab_panels.entry(owner).or_default() += 1;
    }
  }
  assert!(
    selected_radios.values().all(|count| *count <= 1),
    "radio group cannot contain multiple selected radios"
  );
  for draft in drafts {
    if draft.semantic.role == SemanticRole::TabList {
      assert_eq!(
        selected_tabs.get(&draft.id).copied().unwrap_or_default(),
        1,
        "tab list requires exactly one selected enabled tab"
      );
      assert_eq!(
        tab_panels.get(&draft.id).copied().unwrap_or_default(),
        1,
        "tab list requires exactly one exposed panel"
      );
    }
  }
}

fn nearest_role(
  mut cursor: Option<ObjectId>,
  role: SemanticRole,
  drafts: &[Draft<'_>],
  indexes: &HashMap<ObjectId, usize>,
) -> Option<ObjectId> {
  while let Some(id) = cursor {
    let draft = &drafts[indexes[&id]];
    if draft.semantic.role == role {
      return Some(id);
    }
    cursor = draft.parent;
  }
  None
}

fn canonical_root(
  mut cursor: ObjectId,
  drafts: &[Draft<'_>],
  indexes: &HashMap<ObjectId, usize>,
) -> ObjectId {
  while let Some(parent) = drafts[indexes[&cursor]].parent {
    cursor = parent;
  }
  cursor
}

fn requires_name(role: SemanticRole) -> bool {
  !matches!(
    role,
    SemanticRole::Group | SemanticRole::TabPanel | SemanticRole::ScrollArea
  )
}

fn is_actionable(value: &SemanticProps) -> bool {
  value.actions != AccessibilityActionSet::default()
}

fn normalize(value: &str) -> String {
  value.split_whitespace().collect::<Vec<_>>().join(" ")
}
