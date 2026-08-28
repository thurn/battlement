use battlement::{
  Choice, DropdownField, F32Range, MinMaxSlider, RadioButton, RadioButtonGroup, ScrollView,
  Scroller, Slider, SliderInt, TabView, TextField, Toggle, ToggleButtonGroup, UiEventBody,
  UiEventKind, UiValue,
};

use crate::{
  event::{EventHandler, EventHost},
  primitive::{Children, private::Host},
  render::Render,
};

pub(crate) trait TextEventHost: EventHost {}
pub(crate) trait ScrollEventHost: EventHost {}
pub(crate) trait TabEventHost: EventHost {}
pub(crate) trait ValueChangingHost: EventHost {}
pub(crate) trait ValueCommittedHost: EventHost {}

pub(crate) trait ChangeHost: EventHost {
  type Value: 'static;

  fn change_kind() -> UiEventKind;
  fn change_payload(body: UiEventBody) -> Self::Value;
}

macro_rules! capability {
  ($capability:ident: $($host:ty),+ $(,)?) => {
    $(impl $capability for $host {})+

    impl<R: $capability> $capability for EventHandler<R> {}

    impl<H, C> $capability for Children<H, C>
    where
      H: $capability + Host,
      C: Render,
    {
    }
  };
}

macro_rules! change_host {
  ($host:ty, $value:ty, $kind:ident, $body:ident => $payload:expr) => {
    impl ChangeHost for $host {
      type Value = $value;

      fn change_kind() -> UiEventKind {
        UiEventKind::$kind
      }

      fn change_payload($body: UiEventBody) -> Self::Value {
        $payload
      }
    }
  };
}

capability!(TextEventHost: TextField);
capability!(ScrollEventHost: ScrollView);
capability!(TabEventHost: TabView);
capability!(ValueChangingHost: Scroller, Slider, SliderInt, MinMaxSlider);
capability!(
  ValueCommittedHost:
    TextField,
    Toggle,
    RadioButton,
    RadioButtonGroup,
    ToggleButtonGroup,
    DropdownField,
    Scroller,
    Slider,
    SliderInt,
    MinMaxSlider,
);

change_host!(TextField, String, Input, body => match body {
  UiEventBody::Input(value) => value.value,
  _ => panic!("Reactant Input change handler received another event kind"),
});
change_host!(Scroller, f32, ValueChanging, body => self::changing_f32(body));
change_host!(Slider, f32, ValueChanging, body => self::changing_f32(body));
change_host!(SliderInt, i32, ValueChanging, body => match body {
  UiEventBody::ValueChanging(value) => match value.proposed {
    UiValue::I32(value) => value,
    _ => panic!("Reactant SliderInt change handler received another value type"),
  },
  _ => panic!("Reactant SliderInt change handler received another event kind"),
});
change_host!(MinMaxSlider, F32Range, ValueChanging, body => match body {
  UiEventBody::ValueChanging(value) => match value.proposed {
    UiValue::F32Range(value) => value,
    _ => panic!("Reactant MinMaxSlider change handler received another value type"),
  },
  _ => panic!("Reactant MinMaxSlider change handler received another event kind"),
});
change_host!(Toggle, bool, ValueCommitted, body => self::committed_bool(body));
change_host!(RadioButton, bool, ValueCommitted, body => self::committed_bool(body));
change_host!(RadioButtonGroup, Option<u32>, ValueCommitted, body => match body {
  UiEventBody::ValueCommitted(value) => match value.proposed {
    UiValue::Index(value) => value,
    _ => panic!("Reactant RadioButtonGroup change handler received another value type"),
  },
  _ => panic!("Reactant RadioButtonGroup change handler received another event kind"),
});
change_host!(ToggleButtonGroup, Vec<u32>, ValueCommitted, body => match body {
  UiEventBody::ValueCommitted(value) => match value.proposed {
    UiValue::Indices(value) => value,
    _ => panic!("Reactant ToggleButtonGroup change handler received another value type"),
  },
  _ => panic!("Reactant ToggleButtonGroup change handler received another event kind"),
});
change_host!(DropdownField, Choice, ValueCommitted, body => match body {
  UiEventBody::ValueCommitted(value) => match value.proposed {
    UiValue::Choice(value) => value,
    _ => panic!("Reactant DropdownField change handler received another value type"),
  },
  _ => panic!("Reactant DropdownField change handler received another event kind"),
});
change_host!(TabView, u32, TabSelectionRequested, body => match body {
  UiEventBody::TabSelectionRequested(value) => value.proposed_index,
  _ => panic!("Reactant TabView change handler received another event kind"),
});

impl<R: ChangeHost> ChangeHost for EventHandler<R> {
  type Value = R::Value;

  fn change_kind() -> UiEventKind {
    R::change_kind()
  }

  fn change_payload(body: UiEventBody) -> Self::Value {
    R::change_payload(body)
  }
}

impl<H, C> ChangeHost for Children<H, C>
where
  H: ChangeHost + Host,
  C: Render,
{
  type Value = H::Value;

  fn change_kind() -> UiEventKind {
    H::change_kind()
  }

  fn change_payload(body: UiEventBody) -> Self::Value {
    H::change_payload(body)
  }
}

fn changing_f32(body: UiEventBody) -> f32 {
  match body {
    UiEventBody::ValueChanging(value) => match value.proposed {
      UiValue::F32(value) => value,
      _ => panic!("Reactant floating-point change handler received another value type"),
    },
    _ => panic!("Reactant floating-point change handler received another event kind"),
  }
}

fn committed_bool(body: UiEventBody) -> bool {
  match body {
    UiEventBody::ValueCommitted(value) => match value.proposed {
      UiValue::Bool(value) => value,
      _ => panic!("Reactant Boolean change handler received another value type"),
    },
    _ => panic!("Reactant Boolean change handler received another event kind"),
  }
}
