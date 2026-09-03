# Same-struct builder validation

Related: [API guide](../../crates/battlement-builder/README.md) and
[compile-time measurements](builder-performance.md).

## Compiler and interaction checks

The repository aggregate is `./scripts/ci.py`: workspace and sample formatting,
Clippy, Rust tests and doctests, tooling checks, Unity Edit Mode tests, and .NET
diagnostics. The sample assertions remain intact.

Additional downstream fixtures exercise required-field order and repetition,
incomplete types, defaults and conversions, callback signatures, resources with
destructors, generics, conditional compilation, support overrides, and raw
identifiers. Eight- and sixteen-required-field fixtures compile successfully.
The Reactant integration tests dispatch ordinary and model-aware stored event
callbacks through generated props and host setters, reject the wrong model,
and verify clearing does not invoke or subscribe callbacks.

## Editor inspection

Inspected through the installed rust-analyzer 0.3.3033 language server with
procedural macros enabled, using a downstream component with required `width`,
optional `title`, and optional `on_focus`.

- Partial-value completion includes `width`, `title`, `on_focus`, and
  `clear_on_focus`.
- Completed-value completion excludes the already-supplied `width`, while the
  optional methods remain available.
- Hover on `title` includes its field documentation on both partial and
  completed values.
- Hover on `clear_on_focus` contains the clearing explanation and the callback
  property's documentation.

The language server was shut down after inspection. Generated internal generic
names remain visible in expanded hover signatures; the ordinary component name
still denotes its completed specialization.

## Browser inspection

Both WebAssembly samples were built and served with cross-origin isolation,
and their Cloudflare review URLs loaded the intended applications. Browser
interaction used the configured isolated Playwright context.

- Chess-ui: desktop at 1,440 × 1,000, portrait at approximately 768 × 1,024.
  Navigated from Gallery shell to PortraitViewport and ToggleControl. The
  portrait canvas retained its authored 1,024 × 1,536 proportions. Clicking
  VSync changed the checkbox and its proposal count from zero to one;
  reselecting the same gallery page restored the unchecked value and zero count.
- Reactant: desktop at approximately 1,440 × 1,000, phone portrait at
  approximately 390 × 844. Phone navigation switched through Events to State.
  Keyboard activation changed the batched value from zero to three. Reorder
  reversed the keyed children while retaining reducer revision one. Restore
  returned the value, order, and reducer revisions to their initial state.
- Desktop and portrait captures were inspected after application startup.
  Some canvas captures required a one-pixel viewport change to obtain a fresh
  rendered frame. The State screen's existing fixed-width title and token row
  clip horizontally at phone width; their layout rules are unchanged.

Unity's canvas appears as one DOM node, so the browser DOM is not a semantic
tree audit. Accessibility names, roles, focus restoration, and component state
identity are additionally covered by the existing sample interaction suites.
Development builds logged Emscripten's `exit(0)`/keepalive startup message and
slow-frame warnings; no application panic or callback-dispatch failure was
observed during the interactions above.

## Independent review

One independent read-only implementation review reported one finding.
**Accepted:** raw identifiers were not normalized before reserving generated
names, so `r#__BuilderField0` could collide with a generated `__BuilderField0`.
The failing downstream declaration was reproduced, identifier keys were
normalized, and a compile-pass regression was added. The same normalization
also covers generic-reference tracking and associated-type bound lookup; a
mixed raw/ordinary generic-spelling failure was separately reproduced and fixed.
Both regression fixtures pass. No findings remain unresolved.
