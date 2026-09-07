# 08a. Add observable virtual time to the existing fake

[Task group 08](08-public-display-driver.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Validation](../validation.md)
- [Sample migration](../migration.md)
- [Presentation timing](../presentation.md)

**Prerequisite:** [Task 07: Unify Reactant project tooling under
`rt`](07-asset-tooling-boundary.md) is integrated.

**Starting code:** World fake; UI fake; existing game tests; native batches and tween
execution.

## Implementation

1. Extend the world fake's low-level operation scheduling to represent finite tween
   interpolation and waits, instead of jumping to endpoints. Observe audio/effect
   occurrences through public history.

2. Adapt assertions affected by the fake timing change against the still-unmigrated
   samples. Remaining sample-specific rewrites belong to their migrations, before
   cutover. Reuse native baseline evidence for comparison.

3. Keep direct samples working through explicit settle behavior; do not fabricate
   command compatibility or advance time inside a click helper.

4. Keep advance_time, advance_frame, and finite settle explicit on the existing fake. Do
   not require App game sessions or new worker APIs for low-level temporal validation.

## Acceptance

- A 250 ms move has an observable intermediate pose at 125 ms; TimeWait delays its
  successor; duplicate audio delivery creates one occurrence.

- The original tic-tac-toe 99/100 ms and chess capture/spawn timing guarantees are now
  expressed as observable behavior and pass before migration.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](08-public-display-driver.md) identifies
the remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Step an existing move halfway, then settle it; compare the retained native capture and
run affected legacy tests.
