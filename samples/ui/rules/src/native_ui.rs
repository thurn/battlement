use battlement::{ActionId, BatchId, CommandId, ObjectId, SessionId};
use battlement_native::{
  CoreCommandOffset, EngineError, MessageWriter, NativeBatchStart, NativeResponse, UiElementOffset,
  UiNodeOffset,
};

pub(crate) struct NativeUiResponseBuilder {
  writer: MessageWriter,
  groups: Vec<Vec<CoreCommandOffset>>,
}

impl NativeUiResponseBuilder {
  pub(crate) fn new() -> Self {
    Self {
      writer: MessageWriter::default(),
      groups: vec![Vec::new()],
    }
  }

  pub(crate) fn writer(&mut self) -> &mut MessageWriter {
    &mut self.writer
  }

  pub(crate) fn update(
    &mut self,
    object_id: ObjectId,
    element: UiElementOffset,
  ) -> Result<(), EngineError> {
    let command = self
      .writer
      .update_visual_element(
        *CommandId::new_v4().as_uuid().as_bytes(),
        false,
        *object_id.as_uuid().as_bytes(),
        element,
      )
      .map_err(protocol)?;
    self.push(command);
    Ok(())
  }

  pub(crate) fn label(&mut self, object_id: ObjectId, text: &str) -> Result<(), EngineError> {
    let element = self.writer.label(text);
    self.update(object_id, element)
  }

  pub(crate) fn destroy(&mut self, object_id: ObjectId) -> Result<(), EngineError> {
    let command = self
      .writer
      .destroy_visual_element(
        *CommandId::new_v4().as_uuid().as_bytes(),
        false,
        *object_id.as_uuid().as_bytes(),
      )
      .map_err(protocol)?;
    self.push(command);
    Ok(())
  }

  pub(crate) fn index(&mut self, object_id: ObjectId, index: u32) -> Result<(), EngineError> {
    let command = self
      .writer
      .update_visual_element_index(
        *CommandId::new_v4().as_uuid().as_bytes(),
        false,
        *object_id.as_uuid().as_bytes(),
        index,
      )
      .map_err(protocol)?;
    self.push(command);
    Ok(())
  }

  pub(crate) fn parent(
    &mut self,
    object_id: ObjectId,
    parent_id: ObjectId,
  ) -> Result<(), EngineError> {
    let command = self
      .writer
      .update_visual_element_parent(command_id(), false, bytes(object_id), bytes(parent_id))
      .map_err(protocol)?;
    self.push(command);
    Ok(())
  }

  pub(crate) fn create(
    &mut self,
    parent_id: ObjectId,
    root_id: ObjectId,
    nodes: &[UiNodeOffset],
  ) -> Result<(), EngineError> {
    let command = self
      .writer
      .create_visual_element(
        command_id(),
        false,
        bytes(parent_id),
        None,
        bytes(root_id),
        nodes,
      )
      .map_err(protocol)?;
    self.push(command);
    Ok(())
  }

  pub(crate) fn focus(&mut self, object_id: ObjectId) -> Result<(), EngineError> {
    let command = self
      .writer
      .focus_visual_element(command_id(), false, bytes(object_id))
      .map_err(protocol)?;
    self.push(command);
    Ok(())
  }

  pub(crate) fn blur(&mut self, object_id: ObjectId) -> Result<(), EngineError> {
    let command = self
      .writer
      .blur_visual_element(command_id(), false, bytes(object_id))
      .map_err(protocol)?;
    self.push(command);
    Ok(())
  }

  pub(crate) fn capture_pointer(
    &mut self,
    object_id: ObjectId,
    pointer_id: i32,
  ) -> Result<(), EngineError> {
    let command = self
      .writer
      .capture_visual_element_pointer(command_id(), false, bytes(object_id), pointer_id)
      .map_err(protocol)?;
    self.push(command);
    Ok(())
  }

  pub(crate) fn release_pointer(
    &mut self,
    object_id: ObjectId,
    pointer_id: i32,
  ) -> Result<(), EngineError> {
    let command = self
      .writer
      .release_visual_element_pointer(command_id(), false, bytes(object_id), pointer_id)
      .map_err(protocol)?;
    self.push(command);
    Ok(())
  }

  pub(crate) fn scroll_to(
    &mut self,
    object_id: ObjectId,
    descendant_id: ObjectId,
  ) -> Result<(), EngineError> {
    let command = self
      .writer
      .scroll_visual_element_to(command_id(), false, bytes(object_id), bytes(descendant_id))
      .map_err(protocol)?;
    self.push(command);
    Ok(())
  }

  pub(crate) fn select_text(
    &mut self,
    object_id: ObjectId,
    cursor_index: u32,
    selection_index: u32,
  ) -> Result<(), EngineError> {
    let command = self
      .writer
      .select_visual_element_text(
        command_id(),
        false,
        bytes(object_id),
        cursor_index,
        selection_index,
      )
      .map_err(protocol)?;
    self.push(command);
    Ok(())
  }

  pub(crate) fn set_input_enabled(&mut self, enabled: bool) -> Result<(), EngineError> {
    let command = self
      .writer
      .set_input_enabled(command_id(), false, enabled)
      .map_err(protocol)?;
    self.push(command);
    Ok(())
  }

  pub(crate) fn next_group(&mut self) {
    if self
      .groups
      .last()
      .is_some_and(|commands| !commands.is_empty())
    {
      self.groups.push(Vec::new());
    }
  }

  pub(crate) fn finish(
    mut self,
    session_id: SessionId,
    action_id: ActionId,
  ) -> Result<NativeResponse, EngineError> {
    self.groups.retain(|commands| !commands.is_empty());
    if self.groups.is_empty() {
      return NativeResponse::empty(*session_id.as_uuid().as_bytes());
    }
    let mut groups = Vec::with_capacity(self.groups.len());
    for commands in &self.groups {
      groups.push(self.writer.parallel_group(commands).map_err(protocol)?);
    }
    let session = *session_id.as_uuid().as_bytes();
    let batch = self
      .writer
      .batch(
        *BatchId::new_v4().as_uuid().as_bytes(),
        session,
        Some(*action_id.as_uuid().as_bytes()),
        NativeBatchStart::Now,
        &groups,
      )
      .map_err(protocol)?;
    let message = self.writer.finish(session, &[batch]).map_err(protocol)?;
    NativeResponse::from_core(session, message)
  }

  fn push(&mut self, command: CoreCommandOffset) {
    self
      .groups
      .last_mut()
      .expect("native UI response always has a current group")
      .push(command);
  }
}

fn protocol(error: battlement_native::ProtocolError) -> EngineError {
  EngineError::new(error.to_string())
}

fn command_id() -> [u8; 16] {
  *CommandId::new_v4().as_uuid().as_bytes()
}

fn bytes(object_id: ObjectId) -> [u8; 16] {
  *object_id.as_uuid().as_bytes()
}
