# 46a. Build fixed workloads and focused instrumentation

[Task group 46](46-performance-workloads.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Test scenes](../fixtures.md)
- [Validation](../validation.md)
- [Architecture](../architecture.md)

**Prerequisite:** [Task 45: Complete animation, cancellation, and failure test
scenes](45-effects-failures-laboratory.md) is integrated.

**Starting code:** Existing motion performance capture; runtime reconciliation/layout;
inspector counters; Hearts AI.

## Implementation

1. Create the complete neutral card populations across 30 layouts using fixed
   assets/seeds and recorded node/text/material counts. Include sparse/mass updates, 30
   simultaneous tracks, prompts/menus, and concurrent AI.

2. Instrument Rust render/layout/reconcile, complete host CPU, asset loading, command
   generation/execution, GPU, allocations, queue depth, and state-to-visible/input
   latency separately.

3. Keep instrumentation limited to collecting the fixed workload metrics. Use existing
   profilers/counters where possible, mark unavailable measurements explicitly, and
   introduce no permanent inspector dashboard. Record reference environment and capture
   procedure.

## Acceptance

- Both fixed workload sizes render complete cards with correct input and reproducible
  seeds/work counts. A short dry run produces machine-readable timing/counter output and
  environment metadata; sustained capture belongs to 46b.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](46-performance-workloads.md) identifies
the remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Run a short dry capture, confirm actual card/content/layout/track counts, and verify
menu/hover/prompt interactions.
