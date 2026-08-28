use battlement_reactant::{props::Missing, required_props};

struct One<A = Missing> {
  required: (A,),
  optional: (),
}

struct Two<A = Missing, B = Missing> {
  required: (A, B),
  optional: (),
}

struct Three<A = Missing, B = Missing, C = Missing> {
  required: (A, B, C),
  optional: (),
}

struct Four<A = Missing, B = Missing, C = Missing, D = Missing> {
  required: (A, B, C, D),
  optional: (),
}

required_props!(One, a: u8);
required_props!(Two, a: u8, b: u16);
required_props!(Three, a: u8, b: u16, c: u32);
required_props!(Four, a: u8, b: u16, c: u32, d: u64);

impl One {
  fn new() -> Self {
    Self {
      required: (Missing,),
      optional: (),
    }
  }
}

impl Two {
  fn new() -> Self {
    Self {
      required: (Missing, Missing),
      optional: (),
    }
  }
}

impl Three {
  fn new() -> Self {
    Self {
      required: (Missing, Missing, Missing),
      optional: (),
    }
  }
}

impl Four {
  fn new() -> Self {
    Self {
      required: (Missing, Missing, Missing, Missing),
      optional: (),
    }
  }
}

#[test]
fn required_setters_accept_every_order_for_each_supported_arity() {
  assert_eq!(One::new().a(1).required.0, 1);
  let _ = [Two::new().a(1).b(2), Two::new().b(2).a(1)];
  let _ = [
    Three::new().a(1).b(2).c(3),
    Three::new().a(1).c(3).b(2),
    Three::new().b(2).a(1).c(3),
    Three::new().b(2).c(3).a(1),
    Three::new().c(3).a(1).b(2),
    Three::new().c(3).b(2).a(1),
  ];
  let _ = [
    Four::new().a(1).b(2).c(3).d(4),
    Four::new().a(1).b(2).d(4).c(3),
    Four::new().a(1).c(3).b(2).d(4),
    Four::new().a(1).c(3).d(4).b(2),
    Four::new().a(1).d(4).b(2).c(3),
    Four::new().a(1).d(4).c(3).b(2),
    Four::new().b(2).a(1).c(3).d(4),
    Four::new().b(2).a(1).d(4).c(3),
    Four::new().b(2).c(3).a(1).d(4),
    Four::new().b(2).c(3).d(4).a(1),
    Four::new().b(2).d(4).a(1).c(3),
    Four::new().b(2).d(4).c(3).a(1),
    Four::new().c(3).a(1).b(2).d(4),
    Four::new().c(3).a(1).d(4).b(2),
    Four::new().c(3).b(2).a(1).d(4),
    Four::new().c(3).b(2).d(4).a(1),
    Four::new().c(3).d(4).a(1).b(2),
    Four::new().c(3).d(4).b(2).a(1),
    Four::new().d(4).a(1).b(2).c(3),
    Four::new().d(4).a(1).c(3).b(2),
    Four::new().d(4).b(2).a(1).c(3),
    Four::new().d(4).b(2).c(3).a(1),
    Four::new().d(4).c(3).a(1).b(2),
    Four::new().d(4).c(3).b(2).a(1),
  ];
}

mod required_type_collision {
  use battlement_reactant::{props::Missing, required_props};

  type ReactantSecond = u8;

  struct Card<First = Missing, Second = Missing> {
    required: (First, Second),
    optional: (),
  }

  required_props!(Card, first: ReactantSecond, second: u16);

  #[test]
  fn invocation_type_names_are_not_captured() {
    let card = Card {
      required: (Missing, Missing),
      optional: (),
    }
    .second(2)
    .first(1);
    let _: (u8, u16) = card.required;
  }
}

mod component_name_collision {
  use battlement_reactant::{props::Missing, required_props};

  struct ReactantSecond<First = Missing, Second = Missing> {
    required: (First, Second),
    optional: (),
  }

  required_props!(ReactantSecond, first: u8, second: u16);

  #[test]
  fn component_names_are_not_captured() {
    let card = ReactantSecond {
      required: (Missing, Missing),
      optional: (),
    }
    .first(1)
    .second(2);
    let _: (u8, u16) = card.required;
  }
}
