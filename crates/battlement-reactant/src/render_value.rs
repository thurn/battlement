//! Private render-value traversal implementations.

#![allow(private_interfaces)]

use std::{any::TypeId, rc::Rc};

use crate::{
  component::Component,
  context,
  key::StructuralRender,
  render::{Either, Fragment, Node, Render, RenderSink, RenderTree},
  render_error::SharedRenderError,
  runtime::RenderError,
};

pub trait Sealed {
  fn descriptor(&self) -> TypeId;
  fn render_into(&self, sink: &mut RenderSink<'_>);

  fn render_owned(self, sink: &mut RenderSink<'_>)
  where
    Self: Sized,
  {
    self.render_into(sink);
  }

  fn render_shared(self: Rc<Self>, sink: &mut RenderSink<'_>) {
    self.render_into(sink);
  }
}

pub(crate) trait ErasedRender {
  fn descriptor(&self) -> TypeId;
  fn render_into(self: Rc<Self>, sink: &mut RenderSink<'_>);
}

pub(crate) fn lower<R: Render>(
  value: R,
  committed: &RenderTree,
) -> Result<RenderTree, RenderError> {
  let mut sink = RenderSink::new(committed);
  value.render_owned(&mut sink);
  sink.finish()
}

impl<R: Render> ErasedRender for R {
  fn descriptor(&self) -> TypeId {
    Sealed::descriptor(self)
  }

  fn render_into(self: Rc<Self>, sink: &mut RenderSink<'_>) {
    Sealed::render_shared(self, sink);
  }
}

impl Sealed for () {
  fn descriptor(&self) -> TypeId {
    TypeId::of::<Self>()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    sink.push_empty::<Self>();
  }
}

impl<R: Render> Sealed for Option<R> {
  fn descriptor(&self) -> TypeId {
    TypeId::of::<OptionMarker>()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    sink.push_nested::<OptionMarker>(|children| {
      if let Some(value) = self {
        value.render_into(children);
      }
    });
  }

  fn render_owned(self, sink: &mut RenderSink<'_>) {
    sink.push_nested::<OptionMarker>(|children| {
      if let Some(value) = self {
        value.render_owned(children);
      }
    });
  }
}

impl<R: Render, const N: usize> Sealed for [R; N] {
  fn descriptor(&self) -> TypeId {
    TypeId::of::<Self>()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    for value in self {
      value.render_into(sink);
    }
  }

  fn render_owned(self, sink: &mut RenderSink<'_>) {
    for value in self {
      value.render_owned(sink);
    }
  }
}

impl<R: Render> Sealed for Vec<R> {
  fn descriptor(&self) -> TypeId {
    TypeId::of::<Self>()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    for value in self {
      value.render_into(sink);
    }
  }

  fn render_owned(self, sink: &mut RenderSink<'_>) {
    for value in self {
      value.render_owned(sink);
    }
  }
}

impl<R: Render> Sealed for Rc<R> {
  fn descriptor(&self) -> TypeId {
    self.as_ref().descriptor()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    self.as_ref().render_into(sink);
  }

  fn render_owned(self, sink: &mut RenderSink<'_>) {
    self.render_shared(sink);
  }
}

impl<R, E> Sealed for Result<R, E>
where
  R: Render,
  E: std::error::Error + 'static,
{
  fn descriptor(&self) -> TypeId {
    match self {
      Ok(value) => value.descriptor(),
      Err(_) => TypeId::of::<ResultMarker>(),
    }
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    match self {
      Ok(value) => value.render_into(sink),
      Err(_) => panic!("shared Result render values cannot own their error"),
    }
  }

  fn render_owned(self, sink: &mut RenderSink<'_>) {
    match self {
      Ok(value) => value.render_owned(sink),
      Err(error) => sink.fail(RenderError::new(error)),
    }
  }

  fn render_shared(self: Rc<Self>, sink: &mut RenderSink<'_>) {
    match self.as_ref() {
      Ok(value) => value.render_into(sink),
      Err(_) => sink.fail(RenderError::from_shared_render(self)),
    }
  }
}

impl<R: 'static, E: std::error::Error + 'static> SharedRenderError for Result<R, E> {
  fn error(&self) -> &(dyn std::error::Error + 'static) {
    match self {
      Ok(_) => panic!("shared render error is missing"),
      Err(error) => error,
    }
  }
}

impl<R: Render> Sealed for Fragment<R> {
  fn descriptor(&self) -> TypeId {
    TypeId::of::<FragmentMarker>()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    sink.push_nested::<FragmentMarker>(|children| self.children.render_into(children));
  }

  fn render_owned(self, sink: &mut RenderSink<'_>) {
    sink.push_nested::<FragmentMarker>(|children| self.children.render_owned(children));
  }
}

impl<L: Render, R: Render> Sealed for Either<L, R> {
  fn descriptor(&self) -> TypeId {
    match self {
      Either::Left(value) => value.descriptor(),
      Either::Right(value) => value.descriptor(),
    }
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    match self {
      Either::Left(value) => value.render_into(sink),
      Either::Right(value) => value.render_into(sink),
    }
  }

  fn render_owned(self, sink: &mut RenderSink<'_>) {
    match self {
      Either::Left(value) => value.render_owned(sink),
      Either::Right(value) => value.render_owned(sink),
    }
  }
}

impl Sealed for Node {
  fn descriptor(&self) -> TypeId {
    self.descriptor
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    debug_assert_eq!(self.descriptor, self.render.descriptor());
    Rc::clone(&self.render).render_into(sink);
  }
}

impl<C: Component> Sealed for C {
  fn descriptor(&self) -> TypeId {
    TypeId::of::<C>()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    sink.push_component::<C>(|children| {
      debug_assert!(context::hooks_allowed());
      self.render().render_owned(children);
    });
  }
}

impl<C: Component> Render for C {}

macro_rules! tuple_render {
  ($($name:ident),+) => {
    impl<$($name: Render),+> Sealed for ($($name,)+) {
      fn descriptor(&self) -> TypeId {
        TypeId::of::<Self>()
      }

      fn render_into(&self, sink: &mut RenderSink<'_>) {
        #[allow(non_snake_case)]
        let ($($name,)+) = self;
        $($name.render_into(sink);)+
      }

      fn render_owned(self, sink: &mut RenderSink<'_>) {
        #[allow(non_snake_case)]
        let ($($name,)+) = self;
        $($name.render_owned(sink);)+
      }
    }

    impl<$($name: Render),+> Render for ($($name,)+) {}
    impl<$($name: Render),+> StructuralRender for ($($name,)+) {}
  };
}

struct FragmentMarker;
struct OptionMarker;
struct ResultMarker;

tuple_render!(A);
tuple_render!(A, B);
tuple_render!(A, B, C);
tuple_render!(A, B, C, D);
tuple_render!(A, B, C, D, E);
tuple_render!(A, B, C, D, E, F);
tuple_render!(A, B, C, D, E, F, G);
tuple_render!(A, B, C, D, E, F, G, H);
tuple_render!(A, B, C, D, E, F, G, H, I);
tuple_render!(A, B, C, D, E, F, G, H, I, J);
tuple_render!(A, B, C, D, E, F, G, H, I, J, K);
tuple_render!(A, B, C, D, E, F, G, H, I, J, K, L);
