# Reviewer protocol

[Plan and reading guide](../chess-ui-implementation-plan.md)

## Reviewer inputs

The reviewer receives:

- The complete page diff and rendered evidence
- The page description and acceptance checks
- The complete relevant TypeScript files
- The current source-line ownership table
- The Reactant documentation and public tests used by the implementation
- The relevant Reactant implementation and native Unity API evidence, which
  the reviewer inspects independently of the implementer's explanation

Several source files are intentionally divided across pages. The reviewer still
receives the complete file, but every line has one disposition:

- Implemented by this task
- Implemented by an earlier task
- Intentionally assigned to a named later task
- Approved platform substitution

A later-task disposition is valid only when the current task's Deferred
paragraph or the explicit [visual ownership rules](visual-fidelity.md#what-must-look-finished-at-each-step) assign that feature to
that named later task. A short gallery caption alone cannot justify deferral.
Task 40 audits every source line and requires a terminal disposition.

## Mandatory architectural challenge

Review the largest differences in responsibility and composition before minor
style issues. First run a separate, fresh-context subagent using the
[blind idealized Rust port prompt](../idealized-rust-port-prompt.md). Give it only
the selected TypeScript snapshots and the prompt's fixed authoring guide,
including its brief generated-asset and Motion API context. It must not read
other Reactant project files, the existing port, this plan, or earlier audits.
The reviewer may inspect the implementation, but must not pass that context to
the blind subagent.

Record the independent draft and its proposed API contracts before comparing
them with the actual port. Then investigate what prevents that simpler version
and what changes would make it possible. Use typed Rust styles throughout;
runtime CSS strings or a CSS-string styling API are not acceptable. The existing
static asset-generator declaration grammar remains available for generated
PNGs, and runtime animation uses typed Motion builders. The goal is equivalent
expressive power and behavior, not matching token counts or manufacturing a
one-to-one translation of JSX syntax.

For every relevant component, answer these questions with concrete evidence:

1. **Are we preserving the source's actual behavior?** Trace defaults, ignored
   props, callbacks, state ownership, and event propagation. Identify every
   behavior added or changed by the port, even when it seems useful or harmless.
2. **What does React or the browser already do for the source?** Identify the
   work supplied by native elements and built-in relationships. Ask why
   Reactant or Unity cannot own the equivalent work. Do not compare a native
   HTML control with a hand-built Rust control and declare the extra wiring
   inevitable.
3. **Does Unity already provide this control or behavior?** Inspect native
   controls, styling of their internal parts, value events, and Reactant's
   existing facades. For a checkbox, examine `Toggle` before accepting a
   `Button` plus manually attached toggle behavior. If the native control is
   unsuitable, identify the exact missing capability and consider improving
   its Reactant facade before rebuilding it.
4. **Why is each hook, ID, ref, or association authored here?** Compare
   `use_label()` with the source's `useId()` and label relationship. Compare
   explicit focus and activation wiring with the source's wrapping `<label>`.
   Could a stable ID API, associated-label primitive, or composed control own
   these mechanics? Distinguish required internal bookkeeping from a required
   public call. A low-level React Aria hook is one API option, not proof that
   every application needs low-level hooks.
5. **Why can't the render tree be written inline and declaratively?** Challenge
   one-use `control` and `row` variables, mutable builders, conditional setters,
   callback adapters, and repeated clones. Try existing option-aware setters,
   consuming builders, and direct child expressions. If those are insufficient,
   propose the API or ownership change that would remove the friction. Retain
   a local binding when it adds clear meaning or supports real reuse, not
   because the first implementation happened to introduce it.
6. **What major API change would remove the largest remaining difference?**
   Consider replacing an abstraction, moving responsibility into Reactant, or
   revisiting an earlier design decision. Compare a concrete simpler call site
   with the current one. Do not stop at extracting boilerplate into a
   sample-specific helper, renaming a hook, or making a cumbersome pattern
   reusable when a better primitive would eliminate the pattern.

Every claimed unavoidable difference carries a burden of proof. Cite the exact
Rust rule or Unity API constraint, show the relevant implementation, and use a
minimal compile or runtime probe when feasibility is uncertain. Explain which
simpler alternatives were considered and why each fails. A missing Reactant
feature is not a Unity limitation. An ownership issue in the present callback
API is not automatically a language limitation. A design document that mandates
explicit semantics does not rule out a control abstraction that supplies those
semantics internally.

Unverified explanations remain unresolved findings. The reviewer must not mark
them justified, classify them as unavoidable, or issue a no-follow-up result.
Passing CI, matching screenshots, prior promotion, and fixing several trivial
issues do not discharge this architectural review.

## Required evidence and follow-up

The reviewer produces:

- The blind subagent's complete Rust draft, proposed contracts, supplied prompt,
  and TypeScript source revision, preserved before the feasibility review
- A line-by-line TypeScript-to-Rust correspondence table
- A reason for every source line without a direct counterpart
- An inventory of extra Rust hosts, hooks, refs, state, mutation, and glue,
  including whether each belongs in application code or inside Reactant
- A classification of each divergence as a sample defect, generalized Reactant
  friction, proven Unity limitation, proven language constraint, or unresolved
- Answers to the architectural questions above, led by the most consequential
  differences; explain any question that is not applicable
- A concrete simpler Rust call-site sketch and Reactant improvement for every
  generalized divergence, including major API changes where appropriate
- Evidence and rejected alternatives for every claimed unavoidable difference
- Black-box acceptance evidence for the page
- A written no-follow-up rationale only when there are no unresolved findings
  and the strongest simpler designs have been investigated and ruled out

Task 1 has no TypeScript counterpart. Its reviewer instead examines gallery
registration, scrolling, reset, current-page state, and authoring
ergonomics.

Every confirmed sample defect is corrected before the next migration. Resolve
architectural findings before advancing; do not carry them forward as optional
cleanup or defer them to an unspecified redesign. Accept an improvement when
it removes unnecessary application work while preserving the source contract
and framework correctness. Reject it only with a concrete explanation of why
the proposed authoring model is worse or infeasible. Implementation effort alone
is not a rejection reason. All accepted improvements from one page are grouped
into zero or one immediate follow-up commit containing:

- The generalized Reactant change
- Its public tests and documentation
- The page refactor proving the improvement
- No work belonging to the next migration

The follow-up receives ordinary correctness review, CI, candidate validation,
and a separate explicit promotion. It does not receive another specialized
port-ergonomics review. This prevents recursive review while ensuring every
confirmed improvement lands before the next page begins.

Correspondence tables, line ownership, architectural findings, simpler API
sketches, constraint evidence, and no-follow-up rationales are Markdown
attachments to the Tollgate candidate handoff. Screenshots are PNG attachments,
and the blind draft is a Rust source attachment. Automated evidence is the named
Ditto result plus CI run. The handoff stores the source and tested commit IDs so
a later reviewer can retrieve the exact artifact set. Do not embed planning or
historical migration commentary in code or repository documentation.
