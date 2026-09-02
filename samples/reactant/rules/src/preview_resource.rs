use std::sync::{Arc, Mutex};

use battlement_reactant::resource::Resource;
use futures_channel::oneshot::{self, Sender};

/// A demonstration data source completed explicitly by its Resolve button.
#[derive(Clone)]
pub(crate) struct Preview {
  pub(crate) resource: Resource<u32, u32>,
  completion: Arc<Mutex<Option<Sender<u32>>>>,
}

impl Preview {
  pub(crate) fn new() -> Self {
    let completion = Arc::new(Mutex::new(None));
    let pending = Arc::clone(&completion);
    let resource = Resource::new(move |_| {
      let (sender, receiver) = oneshot::channel();
      *pending.lock().expect("preview request lock") = Some(sender);
      async move { receiver.await.expect("preview source remains alive") }
    });
    Self {
      resource,
      completion,
    }
  }

  pub(crate) fn resolve(&self) {
    if let Some(sender) = self.completion.lock().expect("preview request lock").take() {
      let _ = sender.send(1);
    }
  }
}
