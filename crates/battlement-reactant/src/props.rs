//! Required component property typestate.

/// Marks one required component property that has not been supplied.
///
/// Complete component specializations implement [`Component`](crate::component::Component);
/// incomplete specializations remain ordinary non-renderable builder values.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Missing;

/// Generates type-changing setters for one through four required properties.
///
/// The component declares a `required` tuple in macro argument order and one
/// `optional` field. Its complete specialization implements `Component`; the
/// macro emits setters only for states where the corresponding property is
/// [`Missing`]. Optional setters remain hand-written over every state.
///
/// ```
/// use battlement::UiLabel;
/// use battlement_reactant::prelude::*;
///
/// struct Card<Title = Missing, Child = Missing> {
///     required: (Title, Child),
///     optional: bool,
/// }
///
/// impl Card<Missing, Missing> {
///     fn new() -> Self {
///         Self { required: (Missing, Missing), optional: false }
///     }
/// }
///
/// required_props!(Card, title: String, child: UiLabel);
///
/// let _complete = Card::new()
///     .child(UiLabel::new("body"))
///     .title("Citadel".to_owned());
/// ```
///
/// Incomplete values do not render.
///
/// ```compile_fail
/// use battlement_reactant::prelude::*;
///
/// struct Card<Title = Missing> { required: (Title,), optional: () }
/// impl Card<Missing> {
///     fn new() -> Self { Self { required: (Missing,), optional: () } }
/// }
/// required_props!(Card, title: String);
///
/// fn accepts_render(_value: impl Render) {}
/// accepts_render(Card::new());
/// ```
///
/// A required child cannot be supplied twice.
///
/// ```compile_fail
/// use battlement::UiLabel;
/// use battlement_reactant::prelude::*;
///
/// struct Frame<Child = Missing> { required: (Child,), optional: () }
/// impl Frame<Missing> {
///     fn new() -> Self { Self { required: (Missing,), optional: () } }
/// }
/// required_props!(Frame, child: UiLabel);
///
/// let _invalid = Frame::new()
///     .child(UiLabel::new("first"))
///     .child(UiLabel::new("second"));
/// ```
#[macro_export]
macro_rules! required_props {
  (@one $component:ident, $setter:ident: $value_type:ty) => {
    impl $component<$crate::props::Missing> {
      #[doc = concat!("Sets the required `", stringify!($setter), "` property.")]
      #[must_use]
      pub fn $setter(self, value: $value_type) -> $component<$value_type> {
        $component {
          required: (value,),
          optional: self.optional,
        }
      }
    }
  };
  (@two_all $component:ident, $setter:ident: $value_type:ty, $index:tt, $other:ty) => {
    $crate::required_props!(@two_at $component, $setter: $value_type, $index,
      $crate::props::Missing);
    $crate::required_props!(@two_at $component, $setter: $value_type, $index, $other);
  };
  (@two_at $component:ident, $setter:ident: $value_type:ty, 0, $other:ty) => {
    impl $component<$crate::props::Missing, $other> {
      #[doc = concat!("Sets the required `", stringify!($setter), "` property.")]
      #[must_use]
      pub fn $setter(self, value: $value_type) -> $component<$value_type, $other> {
        $component {
          required: (value, self.required.1),
          optional: self.optional,
        }
      }
    }
  };
  (@two_at $component:ident, $setter:ident: $value_type:ty, 1, $other:ty) => {
    impl $component<$other, $crate::props::Missing> {
      #[doc = concat!("Sets the required `", stringify!($setter), "` property.")]
      #[must_use]
      pub fn $setter(self, value: $value_type) -> $component<$other, $value_type> {
        $component {
          required: (self.required.0, value),
          optional: self.optional,
        }
      }
    }
  };
  (
    @three_all $component:ident, $setter:ident: $value_type:ty, $index:tt,
    ($first_other:ty, $second_other:ty)
  ) => {
    $crate::required_props!(@three_at $component, $setter: $value_type, $index,
      ($crate::props::Missing, $crate::props::Missing));
    $crate::required_props!(@three_at $component, $setter: $value_type, $index,
      ($first_other, $crate::props::Missing));
    $crate::required_props!(@three_at $component, $setter: $value_type, $index,
      ($crate::props::Missing, $second_other));
    $crate::required_props!(@three_at $component, $setter: $value_type, $index,
      ($first_other, $second_other));
  };
  (@three_at $component:ident, $setter:ident: $value_type:ty, 0, ($one:ty, $two:ty)) => {
    impl $component<$crate::props::Missing, $one, $two> {
      #[doc = concat!("Sets the required `", stringify!($setter), "` property.")]
      #[must_use]
      pub fn $setter(self, value: $value_type) -> $component<$value_type, $one, $two> {
        $component {
          required: (value, self.required.1, self.required.2),
          optional: self.optional,
        }
      }
    }
  };
  (@three_at $component:ident, $setter:ident: $value_type:ty, 1, ($one:ty, $two:ty)) => {
    impl $component<$one, $crate::props::Missing, $two> {
      #[doc = concat!("Sets the required `", stringify!($setter), "` property.")]
      #[must_use]
      pub fn $setter(self, value: $value_type) -> $component<$one, $value_type, $two> {
        $component {
          required: (self.required.0, value, self.required.2),
          optional: self.optional,
        }
      }
    }
  };
  (@three_at $component:ident, $setter:ident: $value_type:ty, 2, ($one:ty, $two:ty)) => {
    impl $component<$one, $two, $crate::props::Missing> {
      #[doc = concat!("Sets the required `", stringify!($setter), "` property.")]
      #[must_use]
      pub fn $setter(self, value: $value_type) -> $component<$one, $two, $value_type> {
        $component {
          required: (self.required.0, self.required.1, value),
          optional: self.optional,
        }
      }
    }
  };
  (
    @four_all $component:ident, $setter:ident: $value_type:ty, $index:tt,
    ($first_other:ty, $second_other:ty, $third_other:ty)
  ) => {
    $crate::required_props!(@four_at $component, $setter: $value_type, $index,
      ($crate::props::Missing, $crate::props::Missing, $crate::props::Missing));
    $crate::required_props!(@four_at $component, $setter: $value_type, $index,
      ($first_other, $crate::props::Missing, $crate::props::Missing));
    $crate::required_props!(@four_at $component, $setter: $value_type, $index,
      ($crate::props::Missing, $second_other, $crate::props::Missing));
    $crate::required_props!(@four_at $component, $setter: $value_type, $index,
      ($first_other, $second_other, $crate::props::Missing));
    $crate::required_props!(@four_at $component, $setter: $value_type, $index,
      ($crate::props::Missing, $crate::props::Missing, $third_other));
    $crate::required_props!(@four_at $component, $setter: $value_type, $index,
      ($first_other, $crate::props::Missing, $third_other));
    $crate::required_props!(@four_at $component, $setter: $value_type, $index,
      ($crate::props::Missing, $second_other, $third_other));
    $crate::required_props!(@four_at $component, $setter: $value_type, $index,
      ($first_other, $second_other, $third_other));
  };
  (@four_at $component:ident, $setter:ident: $value_type:ty, 0,
    ($one:ty, $two:ty, $three:ty)
  ) => {
    impl $component<$crate::props::Missing, $one, $two, $three> {
      #[doc = concat!("Sets the required `", stringify!($setter), "` property.")]
      #[must_use]
      pub fn $setter(
        self,
        value: $value_type,
      ) -> $component<$value_type, $one, $two, $three> {
        $component {
          required: (value, self.required.1, self.required.2, self.required.3),
          optional: self.optional,
        }
      }
    }
  };
  (@four_at $component:ident, $setter:ident: $value_type:ty, 1,
    ($one:ty, $two:ty, $three:ty)
  ) => {
    impl $component<$one, $crate::props::Missing, $two, $three> {
      #[doc = concat!("Sets the required `", stringify!($setter), "` property.")]
      #[must_use]
      pub fn $setter(
        self,
        value: $value_type,
      ) -> $component<$one, $value_type, $two, $three> {
        $component {
          required: (self.required.0, value, self.required.2, self.required.3),
          optional: self.optional,
        }
      }
    }
  };
  (@four_at $component:ident, $setter:ident: $value_type:ty, 2,
    ($one:ty, $two:ty, $three:ty)
  ) => {
    impl $component<$one, $two, $crate::props::Missing, $three> {
      #[doc = concat!("Sets the required `", stringify!($setter), "` property.")]
      #[must_use]
      pub fn $setter(
        self,
        value: $value_type,
      ) -> $component<$one, $two, $value_type, $three> {
        $component {
          required: (self.required.0, self.required.1, value, self.required.3),
          optional: self.optional,
        }
      }
    }
  };
  (@four_at $component:ident, $setter:ident: $value_type:ty, 3,
    ($one:ty, $two:ty, $three:ty)
  ) => {
    impl $component<$one, $two, $three, $crate::props::Missing> {
      #[doc = concat!("Sets the required `", stringify!($setter), "` property.")]
      #[must_use]
      pub fn $setter(
        self,
        value: $value_type,
      ) -> $component<$one, $two, $three, $value_type> {
        $component {
          required: (self.required.0, self.required.1, self.required.2, value),
          optional: self.optional,
        }
      }
    }
  };
  ($component:ident, $first:ident: $first_type:ty $(,)?) => {
    $crate::required_props!(@one $component, $first: $first_type);
  };
  (
    $component:ident,
    $first:ident: $first_type:ty,
    $second:ident: $second_type:ty $(,)?
  ) => {
    $crate::required_props!(@two_all $component, $first: $first_type, 0, $second_type);
    $crate::required_props!(@two_all $component, $second: $second_type, 1, $first_type);
  };
  (
    $component:ident,
    $first:ident: $first_type:ty,
    $second:ident: $second_type:ty,
    $third:ident: $third_type:ty $(,)?
  ) => {
    $crate::required_props!(@three_all $component, $first: $first_type, 0,
      ($second_type, $third_type));
    $crate::required_props!(@three_all $component, $second: $second_type, 1,
      ($first_type, $third_type));
    $crate::required_props!(@three_all $component, $third: $third_type, 2,
      ($first_type, $second_type));
  };
  (
    $component:ident,
    $first:ident: $first_type:ty,
    $second:ident: $second_type:ty,
    $third:ident: $third_type:ty,
    $fourth:ident: $fourth_type:ty $(,)?
  ) => {
    $crate::required_props!(@four_all $component, $first: $first_type, 0,
      ($second_type, $third_type, $fourth_type));
    $crate::required_props!(@four_all $component, $second: $second_type, 1,
      ($first_type, $third_type, $fourth_type));
    $crate::required_props!(@four_all $component, $third: $third_type, 2,
      ($first_type, $second_type, $fourth_type));
    $crate::required_props!(@four_all $component, $fourth: $fourth_type, 3,
      ($first_type, $second_type, $third_type));
  };
}
