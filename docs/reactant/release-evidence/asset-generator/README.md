# Reactant asset generator release evidence

This directory is the retained release record for the Reactant asset generator.
The validation was run on Apple silicon macOS with Unity 6000.5.8f1 and stable
Google Chrome 152.0.7977.64.

## Automated tiers

The exhaustive command completed successfully from clean Rust target
directories:

```console
python3 scripts/reactant_asset_validation.py exhaustive \
  --evidence docs/reactant/release-evidence/asset-generator/exhaustive
```

It ran the full Rust workspace, every ignored external-tool asset fixture, the
`ReactantGeneratedAssetsExhaustive` Unity category, and native and unthreaded
WebAssembly Reactant sample builds. The NUnit result is retained as
`exhaustive/ReactantGeneratedAssetsExhaustive.xml.txt`; `exhaustive.json`
contains the command and timing transcript.

The public documentation passed both warning-denied generation and doctests;
`documentation.json` retains the command results and timings:

```console
RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps
cargo test --doc --workspace
```

The performance tier created 1,000 Rust files and 100 declarations, then ran 20
warm no-op generations. `performance/performance.json` records a 58.699 ms
median, 59.611 ms p95, 1,152 maximum stat calls, one maximum file open, and
1,034,586 maximum bytes read. Browser, Cargo, source, dependency, PNG, process,
and write counters required to be zero were zero on every invocation.

## Recurring CI budget

`ci-timings.json` retains seven alternating warm baseline/candidate full-CI
pairs. Both sides used the same staged checkout, paths, caches, and player
builds. The baseline suppressed only the consolidated Reactant CLI/browser
lane in the imported CI process.

- Baseline median: 61.952 seconds
- Candidate median: 67.072 seconds
- Median wall-clock delta: 5.120 seconds
- Median fast-tier portion: 5.372 seconds

The result is below the 12-second target and 15-second release cap. The timing
study also exposed the Ditto gate's former 50-second timeout racing its own
50-second wall-clock budget. The timeout and gate now share the existing
60-second added-work budget, and all corrected timing runs passed.

## Release checks

1. A clean sample generation rendered nine deduplicated requests in one Chrome
   context. `manual/cold-generate.json` records one browser launch and one
   context; Unity was not started.
2. The nine generated images were inspected directly. The native and
   WebAssembly gallery captures cover layered clipping, linear/conic/radial
   gradients, masks, inner and outer shadows, filters, transforms, advanced
   text, transparency, and subject bounds.
3. Warm generation reported every request current, launched no browser, opened
   no source, dependency, generated PNG, or browser executable, and wrote
   nothing. Generated modification times were unchanged; see
   `manual/warm-generate.json`.
4. `check` succeeded read-only with no writes and unchanged generated
   modification times; see `manual/check.json`.
5. Preview generated the complete metadata gallery. The preview fixture checks
   its alpha bounds, dependency details, slice data, and large/small nine-slice
   resize controls; see `manual/preview.json`.
6. Exhaustive command fixtures changed source and dependency bytes separately.
   Only affected requests became stale or rerendered, while dependency changes
   retained their public addresses.
7. The exhaustive real-browser fixture changed browser and renderer identity,
   verified cache invalidation, and retained the recorded product, protocol,
   executable, and renderer identities.
8. Public syntax and PNG validation fixtures rejected undeclared edge contact
   with source context and accepted the same paint after the declared clipping
   edge was added.
9. The public native-support matrix rejected a natively expressible rounded
   rectangle and accepted generator-only gradient and clipping compositions.
10. Native and WebAssembly sample builds completed. The native Assets Ditto
    scenario passed all three checkpoints. The running WebAssembly player
    reached 100% load and displayed the generated gallery; see
    `manual/ditto-assets.json`, `manual/webassembly-player.json`, and the PNG
    captures in `manual/`.
11. Exhaustive Unity authoring and build fixtures verified that success,
    failure, and interruption restore user-owned Addressables state.
12. Runtime validation fixtures removed a generated texture and substituted a
    wrong asset type. Both failed before rendering without a placeholder.
13. Initial and authoritative replacement fixtures independently changed the
    linked registration and bundled runtime catalog. Both mismatches failed
    before asset loading or command execution and preserved the authoritative
    snapshot.
14. A release exercise sent `SIGTERM` to the real Chrome renderer while a stale
    request was rendering. Generation failed, the installed root retained SHA
    `b80e0a9e36384a5fa7f2cc236a8c8a80922bdfd4401ff97dea2aec00a4fc02e8`,
    and generate/check recovery passed. Transaction fixtures cover staged and
    preserved-root boundary recovery; see `manual/failure-recovery.json`.
15. Exhaustive fixtures covered unreadable and escaped dependencies, invalid
    browser protocol data, PNG corruption, unavailable output storage, address
    conflict, and Unity import failure. Each compared the installed root and
    Addressables state before and after failure.
16. Native initial, enlarged nine-slice, and restored captures exactly matched
    their accepted baselines. The WebAssembly gallery was also inspected. The
    preview renderer embeds dependencies and the player server log contained
    only local requests.
17. The 20-run performance report meets every latency and work-count budget.
18. Default stable Chrome and explicit Chrome selection ran on macOS. Real
    external-process fixtures rejected a non-Chrome executable and a missing
    explicit executable. Platform-specific selection code enforces the
    documented stable order for macOS, Windows registry application paths, and
    Linux `PATH` candidates.

The repository's single independent review was performed on the asset
generator implementation at commit `9d8d6147`; its confirmed corrections are
included in the implementation. Repository policy permits only one such review
per project session.
