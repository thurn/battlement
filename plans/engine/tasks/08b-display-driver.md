# 08b. Wrap real public paths with display and worker controls

[Task group 08](08-public-display-driver.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Validation](../validation.md)
- [Sample migration](../migration.md)
- [Presentation timing](../presentation.md)

**Prerequisite:** [08a: Add observable virtual time to the existing
fake](08a-fake-time.md) is integrated.

**Starting code:** World fake; UI fake; existing game tests; native batches and tween
execution.

## Implementation

1. Create reactant-testing around FakeClient with documented input, observations, wait,
   advance_time, advance_frame, and finite settle operations for the capabilities
   already implemented. Preserve the distinction between virtual time and worker
   scheduling.

2. Add event-driven lifecycle/barrier waits using task 03's exported worker fixture,
   with bounded hang timeouts. App sessions arrive in task 11; add dispatch, prompt
   waits, and session-aware settle incrementally in tasks 09–12 as their real public
   paths exist. Keep fixture fault injection in explicit builder/service objects. A
   driver must run the real exported/public engine path, never call private rules
   directly.

3. Compose the existing App and exported worker fixture. Add no public recording
   connection or alternate rules executor. Later session/prompt features extend this
   driver through their actual APIs in tasks 09–12.

## Acceptance

- Worker barriers do not advance virtual time, advance_time does not invent a rendered
  frame, and settle stops at infinite cosmetic work. Task 10 adds unanswered-prompt
  stopping once interactive prompts exist.

- One UI scenario uses public input/observations; one exported-worker lifecycle scenario
  uses event-driven waits. A held finite fixture produces a bounded timeout with useful
  context.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](08-public-display-driver.md) identifies
the remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Observe worker start/stop without advancing presentation time, and advance a frame
without implicitly settling work.
