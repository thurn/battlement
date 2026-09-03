mod app_support;

use std::{
  future::Future,
  pin::Pin,
  sync::{Arc, Mutex},
  task::{Context, Poll, Waker},
};

use battlement::{ActionId, ResponseMessage, UiEventAction};
use battlement_fake::client::FakeClient;
use battlement_native::Engine;
use battlement_reactant::{app::App, prelude::*};

#[derive(Default)]
struct Source {
  ready: bool,
  starts: usize,
  canceled: usize,
  polls: usize,
  wake: Option<Waker>,
}
struct Pending {
  source: Arc<Mutex<Source>>,
  completed: bool,
}
#[derive(Clone, PartialEq)]
struct Screen {
  resource: Resource<(), u32>,
}

impl Future for Pending {
  type Output = u32;
  fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<u32> {
    let ready = {
      let mut source = self.source.lock().unwrap();
      source.polls += 1;
      source.wake = Some(context.waker().clone());
      source.ready
    };
    if ready {
      self.completed = true;
      Poll::Ready(42)
    } else {
      Poll::Pending
    }
  }
}

impl Drop for Pending {
  fn drop(&mut self) {
    if !self.completed {
      self.source.lock().unwrap().canceled += 1;
    }
  }
}

impl Component for Screen {
  fn render(&self) -> impl Render {
    let control = use_resource_control(&self.resource);
    View::new().child((
      Button::new("Other").on_click(|| {}),
      Button::new("Refetch")
        .name("refetch")
        .on_click(move || control.invalidate(())),
      Suspense::new(Label::new("Loading").name("status")).child(
        use_resource(&self.resource, ())
          .then(|value| Label::new(format!("Value {value}")).name("status")),
      ),
    ))
  }
}

#[test]
fn pending_resources_wake_refetch_and_cancel_without_an_author_executor() {
  let source = Arc::new(Mutex::new(Source::default()));
  let loader = Arc::clone(&source);
  let resource = Resource::new(move |()| {
    loader.lock().unwrap().starts += 1;
    Pending {
      source: Arc::clone(&loader),
      completed: false,
    }
  });
  let app = App::new("app/content").ui(memo(Screen { resource }));
  let root = app.root_document().root_id;
  let mut client = FakeClient::connect(app, app_support::catalog());
  client.poll();
  assert_eq!(app_support::text(&mut client, root, "status"), "Loading");
  assert_eq!(source.lock().unwrap().polls, 1);
  client.poll();
  assert_eq!(
    source.lock().unwrap().polls,
    1,
    "pending work waits for its waker"
  );
  let wake = {
    let mut source = source.lock().unwrap();
    source.ready = true;
    source.wake.take().unwrap()
  };
  wake.wake();
  client.poll();
  assert_eq!(app_support::text(&mut client, root, "status"), "Value 42");
  source.lock().unwrap().ready = false;
  let refetch = app_support::named(&mut client, root, "refetch");
  client.ui().click(refetch);
  assert_eq!(app_support::text(&mut client, root, "status"), "Loading");
  let old_wake = source.lock().unwrap().wake.take().unwrap();
  client.reconnect();
  client.poll();
  assert_eq!(source.lock().unwrap().canceled, 1);
  assert_eq!(
    source.lock().unwrap().starts,
    3,
    "memoized consumer restarts after reconnect"
  );
  old_wake.wake();
  client.poll();
  assert_eq!(app_support::text(&mut client, root, "status"), "Loading");
  drop(client);
  assert_eq!(source.lock().unwrap().canceled, 2);
}

#[test]
fn refetch_completion_keeps_its_action_when_polled_or_serviced_by_another_event() {
  for another_event in [false, true] {
    let source = Arc::new(Mutex::new(Source::default()));
    let loader = Arc::clone(&source);
    let resource = Resource::new(move |()| Pending {
      source: Arc::clone(&loader),
      completed: false,
    });
    let mut app = App::new("app/content").ui(memo(Screen { resource }));
    let initial = app.connect(app_support::connect()).unwrap();
    let ResponseMessage::Snapshot(snapshot) = &initial.messages[0] else {
      panic!("snapshot")
    };
    let other = snapshot.ui[0].children[0].children[0].object_id;
    let refetch = snapshot.ui[0].children[0].children[1].object_id;
    app.poll().unwrap();
    let action = ActionId::new_v4();
    app
      .submit_ui_event(UiEventAction::new(
        action,
        initial.session_id,
        app_support::click(refetch),
      ))
      .unwrap();
    {
      let mut source = source.lock().unwrap();
      source.ready = true;
      source.wake.take().unwrap().wake();
    }
    let response = if another_event {
      app
        .submit_ui_event(UiEventAction::new(
          ActionId::new_v4(),
          initial.session_id,
          app_support::click(other),
        ))
        .unwrap()
        .response
    } else {
      app.poll().unwrap().expect("completed resource response")
    };
    let batches: Vec<_> = response
      .messages
      .iter()
      .filter_map(|message| match message {
        ResponseMessage::Batch(batch) => Some(batch),
        _ => None,
      })
      .collect();
    assert!(!batches.is_empty());
    assert!(
      batches
        .iter()
        .all(|batch| batch.caused_by_action_id == Some(action))
    );
  }
}
