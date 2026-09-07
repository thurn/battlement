# 32a. Complete chess flow in the Reactant fixture

[Task group 32](32-chess-cutover.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Sample migration](../migration.md)
- [Rules and choices](../execution.md)
- [Animation](../motion.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 31: Port chess board composition and move presentation in a
fixture](31-chess-reactant-fixture.md) is integrated.

**Starting code:** Task 31 chess fixture; existing chess AI, persistence, diagnostics,
audio, spawn and input code.

## Implementation

1. Connect the existing AI search policy through bounded off-main-thread action
   execution. Preserve difficulty/think-time behavior. Existing game-owned search
   cancellation may remain; do not add an explicit engine cancellation primitive.

2. Port play/start/reset, spawn beats, selection effects, music and
   move/capture/check/castle sounds to checkpoint registrations and shared sequences.
   Reuse existing licensed assets and fallback selection logic in Rust.

3. Adapt existing persistence through accepted_state and app-owned status scheduling,
   without an engine acceptance callback. Preserve diagnostics/controller/global-key
   behavior. Save failure must not invalidate an already accepted in-memory action.

4. Keep the default old factory working. Complete the alternate Reactant factory's
   app-owned flow, reuse the same rules/AI implementation, and make existing behavior
   assertions runnable against it. No second permanent engine remains after 32b.

## Acceptance

- Opening spawn beats, capture timing, castling sound composition, AI response, and
  reset match native evidence.

- Exiting during AI or required presentation leaves the replacement responsive and
  rejects old results.

- The Reactant fixture preserves AI settings, save triggers, diagnostics, input
  mappings, and audio. Existing default behavior remains functional until cutover.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](32-chess-cutover.md) identifies the
remaining assignments; do not implement later acceptance criteria here.

## Manual QA

In the alternate factory, play, save/reload, and reset during AI while comparing
retained behavior evidence.
