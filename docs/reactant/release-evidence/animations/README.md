# Reactant Animations release evidence

This directory is the retained release record for Reactant Animations. The
reference run uses the pre-Task-01 release `ea5d1f881efa459f00934d9e39a1d871eb295c89`
and the final Task 12 candidate recorded in `environment.json`.

The fast lane renders the full `transform-200` structure under virtual time and
checks 200 hosts, 120 graph nodes, zero default subscriptions, and 320 authored
timeline slots. Real CPU, presentation-clock, and allocation assertions are not
made by EditMode or shortened sample tests.

The on-demand release pass builds non-development native macOS and unthreaded
desktop WebGL players. Each of `transform-200`, `mixed-200`, and mixed
interaction warms for five seconds and retains the complete following 30
seconds. A valid profile has Motion CPU p95 below 4 ms, average presentation
rate at least 59 fps, 99 percent of intervals at most 18.337 ms, no interval
above 33.34 ms, and zero steady-state managed allocation. It also verifies
actual lifecycle bytes, subscription coalescing, and baseline counters after
leaving the screen.

`ci-timings.json` records the required isolated-cache, alternating baseline and
final ordinary-CI comparison. `manual-qa.json` maps every item in the design's
14-point Manual QA list to retained focused records. `profiles/` contains the
machine-readable native and WebGL profiler results.
