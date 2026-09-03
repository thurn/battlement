# Same-struct builder measurements

The sample migration makes warm incremental `cargo check` about **30 ms slower
for Reactant** and approximately flat for chess-ui on this machine. The
synthetic 16-required-prop case costs about **40 ms more than handwritten
any-order builders** across sixteen component definitions. Neither measurement
supports a universal compile-time percentage.

Related: [builder API](../../crates/battlement-builder/README.md),
[component authoring](component-authoring.md),
[sample measurement script](../../scripts/builder_compile_benchmark.py), and
[synthetic measurement script](../../scripts/builder_synthetic_benchmark.py).

## Conditions and limits

- Apple M5 Max, aarch64 macOS 26.5.2; Rust 1.94.0
  (`4a4ef493e3a1488c6e321570238084b38948f6db`), LLVM 21.1.8.
- Ten observations per sample, mode, and edit, with warm dependencies. Other
  task compilation was paused during the timed runs. These are local timings,
  not a wall-clock CI gate.
- The baseline is release commit `20b4e5034faf1cd8915d1f26822297ed8eaa81c6`.
  Baseline and migrated observations are paired by edit and iteration, but were
  captured in separate before/after sessions, not randomized A/B alternation.
  Background load, cache history, and the complete migration can affect the
  difference. Do not attribute the debug-build improvements to the macro alone.
- `cargo build` means the samples' native debug library build, not a Unity,
  WebAssembly, application relink, or clean dependency build. `cargo check` is
  measured separately. No-op runs are not incremental-edit measurements.
- Each changed sample is restored and rebuilt between observations. Body edits
  change a render literal; chain edits add an unused construction function;
  prop-definition edits replace a `bool` field type with an equivalent alias.
  The latter does not simulate adding a new required prop to many callers.
- Both samples have relatively few required props. The synthetic cases cover
  larger required sets but cannot predict a particular downstream application.

Raw observations: [before](builder-measurements/before.json),
[after](builder-measurements/after.json), and
[synthetic](builder-measurements/synthetic.json). Formatting, documentation, and
an equivalent unit-component `default()` to `new()` call cleanup, and raw-name
hygiene fixes followed the sample timing capture. The hygiene fixes do not
change the generated code for these samples or synthetic declarations; their
effect on macro execution time is not separately measured.

## Sample incremental results

Milliseconds: **median [minimum–maximum]**. Delta is the difference of medians.

| Sample | Command | Edit | Before | After | Delta |
| --- | --- | --- | ---: | ---: | ---: |
| Reactant | check | none | 51 [50–54] | 53 [52–55] | +2 |
| Reactant | check | body | 178 [167–181] | 207 [198–212] | +29 |
| Reactant | check | chain | 175 [173–178] | 206 [200–221] | +32 |
| Reactant | check | prop | 173 [169–182] | 205 [202–220] | +33 |
| Reactant | build | none | 63 [56–80] | 55 [53–66] | -8 |
| Reactant | build | body | 747 [629–898] | 690 [651–724] | -57 |
| Reactant | build | chain | 720 [700–896] | 682 [671–724] | -38 |
| Reactant | build | prop | 793 [756–1133] | 734 [720–773] | -59 |
| Chess-ui | check | none | 70 [68–73] | 60 [60–66] | -10 |
| Chess-ui | check | body | 167 [158–239] | 162 [157–193] | -5 |
| Chess-ui | check | chain | 153 [144–195] | 155 [152–161] | +2 |
| Chess-ui | check | prop | 146 [144–156] | 156 [153–163] | +9 |
| Chess-ui | build | none | 74 [61–116] | 58 [57–65] | -16 |
| Chess-ui | build | body | 703 [585–808] | 544 [505–560] | -159 |
| Chess-ui | build | chain | 625 [565–830] | 581 [548–591] | -44 |
| Chess-ui | build | prop | 836 [625–1540] | 591 [584–632] | -245 |

## Synthetic comparison

Each fixture defines sixteen components and constructs each twice. The
any-order variants use forward and reverse required-setter order in the
“varied” cases; the fixed-order reference necessarily uses forward order both
times. Optional setters are interleaved and remain available at every stage.

The handwritten any-order reference uses the same field-slot design and shared
`Missing<T>`. Its numeric defaults are literal zeroes; generated builders use
`Default`. The fixed-order reference has a concrete struct for each stage and
duplicates optional setters across stages. It is not a fixed-order API that
forbids optional setters before completion.

Dependencies are warmed before each mode. Each measurement changes a literal
inside the fixture's exercise function. Variant order rotates by iteration.
Times include Cargo startup, downstream expansion/checking, and, for builds,
debug code generation. They exclude cold compilation of the proc-macro crate.

Milliseconds: **median [minimum–maximum]**.

| Required / optional | Order | Command | Generated | Handwritten any-order | Fixed-order |
| --- | --- | --- | ---: | ---: | ---: |
| 0 / 8 | same | check | 70 [66–148] | 68 [60–130] | 70 [60–139] |
| 0 / 8 | same | build | 82 [81–91] | 75 [74–93] | 75 [74–78] |
| 2 / 8 | varied | check | 75 [71–167] | 67 [63–150] | 71 [68–169] |
| 2 / 8 | varied | build | 85 [85–90] | 77 [74–82] | 91 [87–95] |
| 8 / 8 | varied | check | 109 [104–164] | 113 [87–143] | 115 [105–167] |
| 8 / 8 | varied | build | 132 [126–151] | 109 [108–126] | 148 [144–156] |
| 16 / 8 | varied | check | 187 [178–352] | 146 [142–220] | 183 [178–372] |
| 16 / 8 | varied | build | 221 [216–239] | 184 [176–192] | 258 [254–274] |
| 8 / 32 | varied | check | 155 [147–306] | 115 [113–185] | 220 [215–426] |
| 8 / 32 | varied | build | 202 [195–206] | 162 [158–174] | 315 [310–338] |
| 16 / 8 | same | check | 180 [177–333] | 144 [137–253] | 183 [177–358] |
| 16 / 8 | same | build | 213 [211–227] | 176 [172–184] | 263 [257–294] |

The fixed-order reference is not uniformly cheaper. In particular, optional
setters repeated across all concrete stages become costly for optional-heavy
components. The any-order design emits linearly many setters, but each required
setter moves every field, and more used type states still add compiler work.

## Source and expansion volume

The migration covers **64 sample-defined components**: 63 production components
and the test-only `Counter`. Counts below cover Rust source in both sample rule
packages, including tests, but not Markdown examples.

| Measure | Count |
| --- | ---: |
| Handwritten construction/setter definitions removed | 53 methods / 297 nonblank lines |
| Construction callsite lines before → after | 627 → 788 (**+161**) |
| All sample Rust lines before → after | 16,161 → 16,316 (**+155**) |
| All sample nonblank Rust lines before → after | 15,184 → 15,334 (**+150**) |
| Shared support + macro implementation | 903 physical lines |
| Net repository Rust source change, including tests | +1,652 physical lines |
| Generated production construction methods | 245 |
| Generated method impls in Reactant sample | 1,697 lines / 69,159 bytes |
| Generated method impls in chess-ui sample | 1,086 lines / 42,515 bytes |

Method-removal counts use signature-through-body source spans, excluding
rustdoc. They exclude the renamed `ReviewPage::title` getter, and include the
viewport constructors whose composition moves into rendering; not all removed
method lines disappear from the program. Appending helpers remain handwritten.

Callsite counts are the union of source lines spanned by sample-component
construction expressions and their chained methods, including expressions in
parseable comma-separated macros. Construction inside removed methods is
excluded. Multi-line enclosing expressions and nested constructions are counted
once per physical line. This is a syntactic estimate, not a token reduction.

The sample migration is consequently **not a net source-line saving**. Many
Reactant components previously had no constructors or setters at all; converting
literal construction into named method chains adds lines. The macro eliminates
repeated implementation work while making these components consistently
constructible. The shared implementation, tests, and measurement scripts are
additional repository code, not sample boilerplate savings.

Expansion counts use `rustc -Zunpretty=expanded` after timing, with
`RUSTC_BOOTSTRAP=1` only for that inspection. Sample counts include generated
method impls, not transformed declarations or unrelated expansions. Synthetic
counts below include the entire fixture. Printed lines depend on compiler
formatting; bytes also include documentation and identifier length, and neither
is a direct measure of typechecking cost.

| Required / optional | Generated lines / bytes | Handwritten lines / bytes | Fixed-order lines / bytes |
| --- | ---: | ---: | ---: |
| 0 / 8 | 1,210 / 37,112 | 442 / 16,184 | 442 / 16,184 |
| 2 / 8 | 1,866 / 59,516 | 1,162 / 33,260 | 1,866 / 55,092 |
| 8 / 8 | 5,738 / 193,992 | 3,872 / 109,088 | 7,242 / 199,416 |
| 16 / 8 | 13,868 / 525,712 | 10,208 / 293,648 | 17,994 / 473,864 |
| 8 / 32 | 11,690 / 374,472 | 8,096 / 236,576 | 17,610 / 578,040 |

## Reproduction

Run the sample script on the baseline before migration without `--generated`,
then on the migrated source with it. Do not run other compilation concurrently.
Both scripts use ignored target directories for measurements and fixtures.

```sh
python3 scripts/builder_compile_benchmark.py --generated --runs 10 \
  --output target/builder-benchmark/after.json
python3 scripts/builder_synthetic_benchmark.py --runs 10 --components 16 \
  --output target/builder-benchmark/synthetic.json
```
