# Collection components

Reactant collection components publish roles and relationships from ordinary
logical ancestry. Applications compose `ListBox` with `ListBoxOption`, and
`Table` with `TableRow`, `TableCell`, `ColumnHeader`, and `RowHeader`.

```rust
ListBox::new(ls("Quality")).child((
  ListBoxOption::new(ls("Standard"), selected == 0)
    .on_press(|game: &mut Game| game.selected = 0),
  ListBoxOption::new(ls("High"), selected == 1)
    .on_press(|game: &mut Game| game.selected = 1),
))
```

Selection remains controlled. Invalid ancestry and multiple selected options
fail before commit. Logical ancestry is authoritative through transparent hosts
and portals.

`Navigation`, `Region`, and `Group` provide landmark and grouping semantics.
`Link` uses the shared press path and link role; its callback remains
responsible for navigation.
