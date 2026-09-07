# 46. Measure complete-card workloads and repair structural hotspots

Fixed 300/500-card release workloads produce reproducible
CPU/GPU/latency/allocation evidence and concrete bottleneck follow-ups.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Test scenes](../fixtures.md)
- [Validation](../validation.md)
- [Architecture](../architecture.md)

**Prerequisite:** [Task 45: Complete animation, cancellation, and failure test
scenes](45-effects-failures-laboratory.md) and all its required follow-ups must
be integrated.

**Starting code:** Existing motion performance capture; runtime
reconciliation/layout; inspector counters; Hearts AI.

## Example

Record the unchanged workload alongside measurements so results are comparable:

```text
cards: 300 and 500 complete composed views
layouts: 30; concurrent movement/effect tracks: 30
duration: at least ten minutes after warmup; AI enabled
report CPU/GPU/frame/input distributions and every missed target
```

## Implementation

1. Create the complete neutral card populations across 30 layouts using fixed
   assets/seeds and recorded node/text/material counts. Include sparse/mass
   updates, 30 simultaneous tracks, prompts/menus, and concurrent AI.

2. Instrument Rust render/layout/reconcile, complete host CPU, asset loading,
   command generation/execution, GPU, allocations, queue depth, and
   state-to-visible/input latency separately.

3. Remove unconditional quadratic sibling/mutation planning on this workload
   using indexed matching and localized invalidation. Avoid serializing
   unchanged properties and keep geometry observations opt-in; verify these
   structural requirements directly.

4. Run warmed release captures for at least ten minutes on recorded desktop
   native and threaded WebGL environments. Report both workload sizes and
   compare to the nonblocking targets in fixtures.md.

5. Record missed targets and concrete owning-code follow-ups outside maintained
   guidance, with reproducible inputs. Do not lower workload complexity or
   disable effects to improve the result.

## Acceptance

- Both fixed workload sizes complete with correct rendering/input and
  machine-readable distributions, environment metadata, and AI work counts.

- Sparse changes avoid rebuilding/serializing unrelated subtrees; ordinary
  animation remains host-local.

- No unconditional quadratic sibling/mutation path remains on the reference
  workload.

- Missed numeric targets are reported honestly and do not fail completion;
  correctness defects still do.

- Public simulation primitive benchmarks still meet the separate
  no-extra-primitive-allocation/no-vtable contract, with mode branching allowed
  and owned prompt construction/search costs measured separately.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Physical sustained device captures remain separate certification. Final
cross-platform regression consolidation is task 47.

## Manual QA

During each sustained capture, repeatedly open menus/inspection and answer
prompts. Inspect worst frames and input delays, not only average FPS.
