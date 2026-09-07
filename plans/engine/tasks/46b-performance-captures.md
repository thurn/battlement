# 46b. Measure release workloads and address demonstrated hotspots

[Task group 46](46-performance-workloads.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Test scenes](../fixtures.md)
- [Validation](../validation.md)
- [Architecture](../architecture.md)

**Prerequisite:** [46a: Build fixed workloads and focused
instrumentation](46a-performance-fixture.md) is integrated.

**Starting code:** Existing motion performance capture; runtime reconciliation/layout;
inspector counters; Hearts AI.

## Implementation

1. Run warmed release captures for at least ten minutes on recorded desktop native and
   threaded WebGL environments. Report both workload sizes and compare to the
   nonblocking targets in fixtures.md.

2. Profile sparse updates and mass reflow to identify actual bottlenecks. Optimize only
   measured hotspots with a focused before/after comparison. Keep geometry observations
   opt-in and ordinary motion host-local. Neither a suspected complexity pattern nor a
   numeric miss mandates an algorithm rewrite.

3. Record missed targets and concrete owning-code follow-ups outside maintained
   guidance, with reproducible inputs. Do not lower workload complexity or disable
   effects to improve the result.

## Acceptance

- Both fixed workload sizes complete with correct rendering/input and machine-readable
  distributions, environment metadata, and AI work counts.

- Sparse-update costs and measured hotspots are reported; ordinary animation remains
  host-local. Any optimization has a measured before/after result. Complexity bounds are
  not separate completion gates.

- Missed numeric targets are reported honestly and do not fail completion; correctness
  defects still do.

- Public simulation primitive benchmarks still meet the separate
  no-extra-primitive-allocation/no-vtable contract, with mode branching allowed and
  owned prompt construction/search costs measured separately.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](46-performance-workloads.md) identifies
the remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Run the required warmed native/Web captures, inspect slow frames, and report misses with
reproducible inputs. Any in-scope optimization needs measured before/after evidence.
