use std::{
  cell::RefCell,
  fmt,
  rc::{Rc, Weak},
  time::Duration,
};

use battlement::{
  AudioBufferingPayload, AudioClipAddress, AudioPlayPayload, AudioPlaybackPayload,
  AudioReplacePayload, AudioSeekPayload, AudioStopPayload, AudioVolumePayload, Command,
  CommandBody, CommandId, MotionPlaybackCommand, MotionValueDescriptor,
  MotionValuePlaybackOperation, ObjectId, PropertyCommand,
};

use crate::{
  motion_value::{
    AnimationPlayback, AudioPlayback, AudioPlaybackOptions, ErasedMotionValue,
    MotionValueRuntimeHandle, PlaybackInner, PlaybackOutcome, duration_micros, duration_millis,
  },
  motion_value_runtime::{self, MotionValueRuntime},
};

impl AudioPlayback {
  /// Creates a typed audio-playback identity from an operation UUID.
  #[must_use]
  pub const fn new(operation_id: ObjectId) -> Self {
    Self { operation_id }
  }

  /// Returns the stable operation identity shared with native audio.
  #[must_use]
  pub const fn id(self) -> ObjectId {
    self.operation_id
  }

  /// Creates a nonblocking play command and its shared stable identity.
  pub fn play(address: AudioClipAddress, options: AudioPlaybackOptions) -> (Self, Command) {
    let command_id = CommandId::new_v4();
    let playback = Self {
      operation_id: ObjectId::from_uuid(command_id.into_uuid())
        .expect("generated audio command identity is nonzero"),
    };
    (playback, playback.play_command(address, options))
  }

  /// Creates the play command for this preallocated stable identity.
  pub fn play_command(self, address: AudioClipAddress, options: AudioPlaybackOptions) -> Command {
    Command::new(
      self.command_id(),
      CommandBody::AudioPlay(AudioPlayPayload {
        address,
        volume: options.volume,
        pitch: options.pitch,
        r#loop: options.looping,
        fade_in_ms: duration_millis(options.fade_in),
      }),
    )
    .nonblocking()
  }

  /// Creates an immediate pause command.
  pub fn pause(self) -> Command {
    Command::new_v4(CommandBody::AudioPause(self.playback_payload()))
  }

  /// Creates an immediate resume command.
  pub fn resume(self) -> Command {
    Command::new_v4(CommandBody::AudioResume(self.playback_payload()))
  }

  /// Creates an exact seek command.
  pub fn seek(self, position: Duration) -> Command {
    Command::new_v4(CommandBody::AudioSeek(AudioSeekPayload {
      audio_command_id: self.command_id(),
      position_ms: duration_millis(position),
    }))
  }

  /// Creates a buffering-state command.
  pub fn set_buffering(self, buffering: bool) -> Command {
    Command::new_v4(CommandBody::AudioSetBuffering(AudioBufferingPayload {
      audio_command_id: self.command_id(),
      buffering,
    }))
  }

  /// Creates a clip-replacement command without changing identity.
  pub fn replace(self, address: AudioClipAddress) -> Command {
    Command::new_v4(CommandBody::AudioReplace(AudioReplacePayload {
      audio_command_id: self.command_id(),
      address,
    }))
  }

  /// Creates an immediate volume command.
  pub fn set_volume(self, volume: f64) -> Command {
    Command::new_v4(CommandBody::AudioSetVolume(PropertyCommand::canceling(
      AudioVolumePayload {
        audio_command_id: self.command_id(),
        volume,
      },
    )))
  }

  /// Creates a stop command with an optional fade.
  pub fn stop(self, fade_out: Duration) -> Command {
    Command::new_v4(CommandBody::AudioStop(AudioStopPayload {
      audio_command_id: self.command_id(),
      fade_out_ms: duration_millis(fade_out),
    }))
  }

  fn command_id(self) -> CommandId {
    CommandId::from_uuid(self.operation_id.into_uuid()).expect("audio playback identity is nonzero")
  }

  fn playback_payload(self) -> AudioPlaybackPayload {
    AudioPlaybackPayload {
      audio_command_id: self.command_id(),
    }
  }
}

impl AudioPlaybackOptions {
  /// Creates ordinary one-shot playback options.
  #[must_use]
  pub const fn new() -> Self {
    Self {
      volume: 1.0,
      pitch: 1.0,
      looping: false,
      fade_in: Duration::ZERO,
    }
  }

  /// Sets initial volume.
  #[must_use]
  pub fn volume(mut self, value: f64) -> Self {
    assert!((0.0..=1.0).contains(&value), "audio volume is out of range");
    self.volume = value;
    self
  }

  /// Sets playback pitch.
  #[must_use]
  pub fn pitch(mut self, value: f64) -> Self {
    assert!(value > 0.0 && value <= 3.0, "audio pitch is out of range");
    self.pitch = value;
    self
  }

  /// Sets looping playback.
  #[must_use]
  pub const fn looping(mut self, value: bool) -> Self {
    self.looping = value;
    self
  }

  /// Sets fade-in duration.
  #[must_use]
  pub const fn fade_in(mut self, value: Duration) -> Self {
    self.fade_in = value;
    self
  }
}

impl Default for AudioPlaybackOptions {
  fn default() -> Self {
    Self::new()
  }
}

impl AnimationPlayback {
  pub(crate) fn new(runtime_id: u64, runtime: Weak<RefCell<MotionValueRuntime>>) -> Self {
    let inner = Rc::new(PlaybackInner {
      runtime_id,
      runtime: runtime.clone(),
      playback_id: ObjectId::new_v4(),
      generation: 1,
      terminal: RefCell::new(None),
      reported: RefCell::new(None),
      callbacks: RefCell::new(Default::default()),
    });
    if let Some(runtime) = runtime.upgrade() {
      let playback = Rc::downgrade(&inner);
      runtime
        .borrow_mut()
        .register_playback(inner.playback_id, inner.generation, move |outcome| {
          playback
            .upgrade()
            .is_some_and(|playback| playback.finish(outcome.into()))
        });
    }
    Self { inner }
  }

  pub(crate) fn from_handle(handle: &MotionValueRuntimeHandle) -> Self {
    Self::new(handle.runtime_id, handle.runtime.clone())
  }

  pub(crate) fn protocol_identity(&self) -> (ObjectId, u32) {
    (self.inner.playback_id, self.inner.generation)
  }

  /// Resumes logical time.
  pub fn play(&self) {
    self.queue(MotionPlaybackCommand::Play);
  }

  /// Pauses at the current logical time.
  pub fn pause(&self) {
    self.queue(MotionPlaybackCommand::Pause);
  }

  /// Stops and freezes the current presentation value.
  pub fn stop(&self) {
    self.terminal(PlaybackOutcome::Stopped, MotionPlaybackCommand::Stop);
  }

  /// Cancels and exposes the lower animation layer.
  pub fn cancel(&self) {
    self.terminal(PlaybackOutcome::Cancelled, MotionPlaybackCommand::Cancel);
  }

  /// Applies the terminal target immediately.
  pub fn complete(&self) {
    self.terminal(PlaybackOutcome::Completed, MotionPlaybackCommand::Complete);
  }

  /// Samples an exact logical time and leaves playback paused.
  pub fn seek(&self, elapsed: Duration) {
    self.queue(MotionPlaybackCommand::Seek {
      elapsed_micros: duration_micros(elapsed),
    });
  }

  /// Sets a finite nonnegative playback rate.
  pub fn set_speed(&self, speed: f32) {
    assert!(
      speed.is_finite() && speed >= 0.0,
      "playback speed must be finite and nonnegative"
    );
    self.queue(MotionPlaybackCommand::SetSpeed {
      value: f64::from(speed),
    });
  }

  /// Sets explicit playback direction.
  pub fn set_direction(&self, direction: crate::motion_value::PlaybackDirection) {
    self.queue(MotionPlaybackCommand::SetDirection { value: direction });
  }

  /// Registers a callback for successful completion.
  pub fn on_complete(&self, callback: impl FnOnce() + 'static) {
    if *self.inner.reported.borrow() == Some(PlaybackOutcome::Completed) {
      callback();
    } else {
      self.inner.callbacks.borrow_mut().complete = Some(Box::new(callback));
    }
  }

  /// Registers a callback for explicit stop.
  pub fn on_stop(&self, callback: impl FnOnce() + 'static) {
    if *self.inner.reported.borrow() == Some(PlaybackOutcome::Stopped) {
      callback();
    } else {
      self.inner.callbacks.borrow_mut().stop = Some(Box::new(callback));
    }
  }

  /// Registers a callback for cancellation.
  pub fn on_cancel(&self, callback: impl FnOnce() + 'static) {
    if *self.inner.reported.borrow() == Some(PlaybackOutcome::Cancelled) {
      callback();
    } else {
      self.inner.callbacks.borrow_mut().cancel = Some(Box::new(callback));
    }
  }

  fn terminal(&self, outcome: PlaybackOutcome, command: MotionPlaybackCommand) {
    if self.inner.terminal.borrow().is_some() {
      return;
    }
    *self.inner.terminal.borrow_mut() = Some(outcome);
    self.queue(command);
  }

  fn queue(&self, command: MotionPlaybackCommand) {
    if self.inner.terminal.borrow().is_some()
      && !matches!(
        command,
        MotionPlaybackCommand::Stop
          | MotionPlaybackCommand::Cancel
          | MotionPlaybackCommand::Complete
      )
    {
      return;
    }
    motion_value_runtime::queue(
      self.inner.runtime_id,
      &self.inner.runtime,
      CommandBody::MotionValuePlayback(MotionValuePlaybackOperation {
        playback_id: self.inner.playback_id,
        generation: self.inner.generation,
        command,
      }),
    );
  }
}

impl MotionValueRuntimeHandle {
  pub(crate) fn current() -> Self {
    let (runtime_id, runtime) = motion_value_runtime::current_runtime();
    Self {
      runtime_id,
      runtime,
    }
  }

  pub(crate) fn queue(&self, body: CommandBody) {
    motion_value_runtime::queue(self.runtime_id, &self.runtime, body);
  }
}

impl ErasedMotionValue {
  pub(crate) fn id(&self) -> ObjectId {
    self.inner.descriptor.value_id
  }

  pub(crate) fn collect(&self, values: &mut Vec<MotionValueDescriptor>) {
    if values.iter().any(|value| value.value_id == self.id()) {
      return;
    }
    for dependency in &self.inner.dependencies {
      dependency.collect(values);
    }
    values.push(self.inner.descriptor.clone());
  }

  pub(crate) fn collect_subscriptions(
    &self,
    subscriptions: &mut Vec<battlement::MotionValueSubscription>,
  ) {
    for subscription in self.inner.subscriptions.borrow().iter().copied() {
      if !subscriptions
        .iter()
        .any(|value| value.subscription_id == subscription.subscription_id)
      {
        subscriptions.push(subscription);
      }
    }
    for dependency in &self.inner.dependencies {
      dependency.collect_subscriptions(subscriptions);
    }
  }
}

impl fmt::Debug for ErasedMotionValue {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_tuple("MotionValue")
      .field(&self.id())
      .finish()
  }
}

impl PartialEq for ErasedMotionValue {
  fn eq(&self, other: &Self) -> bool {
    self.id() == other.id()
  }
}

impl Eq for ErasedMotionValue {}

#[cfg(test)]
mod tests {
  use std::{cell::Cell, rc::Rc};

  use battlement::{MotionPlaybackEvent, MotionPlaybackOutcome};

  use super::*;

  #[test]
  fn playback_callbacks_wait_for_the_matching_native_terminal_event() {
    let runtime = MotionValueRuntime::new(7);
    let playback = AnimationPlayback::new(7, Rc::downgrade(&runtime));
    let completed = Rc::new(Cell::new(0));
    let observed = completed.clone();
    playback.on_complete(move || observed.set(observed.get() + 1));
    let (playback_id, generation) = playback.protocol_identity();

    assert!(
      runtime
        .borrow_mut()
        .take_playback_events(&[MotionPlaybackEvent {
          playback_id,
          generation: generation + 1,
          outcome: MotionPlaybackOutcome::Completed,
        },])
        .is_empty()
    );
    assert_eq!(completed.get(), 0);
    let [invocation] = runtime
      .borrow_mut()
      .take_playback_events(&[MotionPlaybackEvent {
        playback_id,
        generation,
        outcome: MotionPlaybackOutcome::Completed,
      }])
      .try_into()
      .unwrap_or_else(|_| panic!("matching playback callback is missing"));
    assert!(invocation.invoke());
    assert_eq!(completed.get(), 1);
    assert!(
      runtime
        .borrow_mut()
        .take_playback_events(&[MotionPlaybackEvent {
          playback_id,
          generation,
          outcome: MotionPlaybackOutcome::Completed,
        },])
        .is_empty()
    );
    assert_eq!(completed.get(), 1);
  }
}
