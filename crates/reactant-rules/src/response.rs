use std::{
  any::{Any, TypeId},
  marker::PhantomData,
  sync::{
    Arc, Condvar, Mutex, Weak,
    atomic::{AtomicBool, Ordering},
  },
};

use crate::{ChoiceOwner, Game, PromptData, worker};

/// Owned display data and the response connection for that exact request.
pub struct PresentedPrompt<T> {
  pub prompt: T,
  pub handle: ResponseHandle<T>,
}

/// A typed connection to one request, valid independently of display consumption.
pub struct ResponseHandle<T> {
  request: Weak<dyn Reply>,
  owner: ChoiceOwner,
  prompt_type: PhantomData<fn() -> T>,
}

pub(crate) trait Reply: Send + Sync {
  fn submit(&self, game: TypeId, prompt: TypeId, response: Box<dyn Any + Send>);
  fn cancel(&self);
  fn active(&self) -> bool;
  fn owner(&self) -> ChoiceOwner;
}

pub(crate) struct Request<G: Game, P: PromptData<G>> {
  pub(crate) prompt: Arc<P>,
  owner: ChoiceOwner,
  abandoned: Arc<AtomicBool>,
  answer: Mutex<Answer<P::ResponseType>>,
  changed: Condvar,
  game: PhantomData<fn() -> G>,
}

struct Answer<R> {
  ended: bool,
  response: Option<R>,
}

impl<T> Clone for ResponseHandle<T> {
  fn clone(&self) -> Self {
    Self {
      request: self.request.clone(),
      owner: self.owner,
      prompt_type: PhantomData,
    }
  }
}

impl<T> ResponseHandle<T> {
  pub(crate) fn new(request: &Arc<dyn Reply>) -> Self {
    Self {
      request: Arc::downgrade(request),
      owner: request.owner(),
      prompt_type: PhantomData,
    }
  }

  /// Validates against the retained request and resumes it once.
  ///
  /// Ended requests ignore every reply. Active misuse panics at this Rust
  /// boundary; host callbacks must use the application's panic boundary.
  /// The supplied prompt selects a type, never the legal choice set.
  ///
  /// ```compile_fail
  /// use reactant_rules::{Game, PromptData, ResponseHandle};
  /// fn wrong<G: Game, P: PromptData<G, ResponseType = u8>>(
  ///   handle: &ResponseHandle<G::Prompt<'static>>, prompt: &P,
  /// ) {
  ///   handle.submit::<G, P>(prompt, [1_u8, 2_u8]);
  /// }
  /// ```
  pub fn submit<G, P>(&self, _prompt: &P, response: P::ResponseType)
  where
    G: Game<Prompt<'static> = T>,
    P: PromptData<G>,
  {
    if let Some(request) = self.request.upgrade() {
      request.submit(TypeId::of::<G>(), TypeId::of::<P>(), Box::new(response));
    }
  }

  /// Returns the immutable request owner, including after resolution.
  pub fn owner(&self) -> ChoiceOwner {
    self.owner
  }

  /// Reports an unanswered human decision, where finite display settling stops.
  pub fn is_waiting_for_human(&self) -> bool {
    self
      .request
      .upgrade()
      .is_some_and(|request| request.owner() == ChoiceOwner::Human && request.active())
  }

  /// Reports whether this request still awaits its owner's answer.
  pub fn is_active(&self) -> bool {
    self
      .request
      .upgrade()
      .is_some_and(|request| request.active())
  }
}

impl<G: Game, P: PromptData<G>> Request<G, P> {
  pub(crate) fn new(prompt: P, owner: ChoiceOwner, abandoned: Arc<AtomicBool>) -> Self {
    Self {
      prompt: Arc::new(prompt),
      owner,
      abandoned,
      answer: Mutex::new(Answer {
        ended: false,
        response: None,
      }),
      changed: Condvar::new(),
      game: PhantomData,
    }
  }

  pub(crate) fn wait(&self) -> P::ResponseType {
    let mut answer = self.answer.lock().unwrap();
    loop {
      if self.abandoned.load(Ordering::Acquire) {
        drop(answer);
        worker::unwind_cancelled();
      }
      if let Some(response) = answer.response.take() {
        return response;
      }
      answer = self.changed.wait(answer).unwrap();
    }
  }

  pub(crate) fn finish_policy(&self) {
    self.answer.lock().unwrap().ended = true;
  }
}

impl<G: Game, P: PromptData<G>> Reply for Request<G, P> {
  fn submit(&self, game: TypeId, prompt: TypeId, response: Box<dyn Any + Send>) {
    {
      let answer = self.answer.lock().unwrap();
      if answer.ended || self.abandoned.load(Ordering::Acquire) {
        return;
      }
    }
    // Game-owned validation runs without communication locks. Concurrent replies
    // may validate, but only the first still-active answer is installed below.
    assert!(
      self.owner == ChoiceOwner::Human,
      "human reply to policy-owned request"
    );
    assert!(game == TypeId::of::<G>(), "response game type mismatch");
    assert!(prompt == TypeId::of::<P>(), "response prompt type mismatch");
    let response = response
      .downcast::<P::ResponseType>()
      .unwrap_or_else(|_| panic!("response payload type mismatch"));
    assert!(
      self.prompt.is_valid_response(&response),
      "illegal prompt response"
    );
    let mut answer = self.answer.lock().unwrap();
    if answer.ended || self.abandoned.load(Ordering::Acquire) {
      return;
    }
    answer.response = Some(*response);
    answer.ended = true;
    self.changed.notify_all();
  }

  fn cancel(&self) {
    self.answer.lock().unwrap().ended = true;
    self.changed.notify_all();
  }

  fn owner(&self) -> ChoiceOwner {
    self.owner
  }

  fn active(&self) -> bool {
    let answer = self.answer.lock().unwrap();
    !answer.ended && !self.abandoned.load(Ordering::Acquire)
  }
}
