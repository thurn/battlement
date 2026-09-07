# 44a. Finish demonstrated identity and composition gaps

[Task group 44](44-identity-composition-laboratory.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Test scenes](../fixtures.md)
- [Identity and state](../identity.md)
- [World objects and input](../world.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 43: Save Hearts explicitly and resume accepted
state](43-hearts-save-resume.md) is integrated.

**Starting code:** Reactant laboratory/inspector; world primitives; UI/world projection;
public display driver.

## Implementation

1. Review existing identity-transfer, duplicate-identity, visibility-transition,
   ui-world-transfer, mixed-input, stores, composed-card, and contained-layout scenes.
   Complete only missing behavior; reuse the feature owners' public scenarios and native
   evidence.

2. Use neutral synthetic rich text, badges, outline visuals, face variants, UI preview,
   nested cards, and conditional action controls. Add a reusable generic host capability
   if a fixture exposes a real engine gap.

3. Reuse representative layout transfers plus cross-domain, portal, active-root,
   removed-ancestor, and interrupted-movement boundaries. One scene may cover several
   boundaries. No exhaustive source/destination layout matrix.

4. Assert both state/ref retention and changed ancestry/context through public fixture
   output. Validate material isolation and changed hit geometry in native captures.

5. Keep test scene reset deterministic and assert cleanup rather than accumulating
   retained objects across selections.

## Acceptance

- Every distinct transfer boundary has evidence, reusing earlier public-driver scenarios
  and native captures wherever they already establish the behavior.

- Rich card faces, badges/text/outline ordering, contained layout, conditional controls,
  and independent inspection are visibly correct.

- Duplicate rejection and single-visual hide/show identity pass across UI/world roots;
  stale callbacks cannot reach replacement native handles.

- Native geometry confirms screen-space continuity, context-dependent hit regions, and
  resize/reorientation behavior.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](44-identity-composition-laboratory.md)
identifies the remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Reset and exercise the representative transfer boundaries; inspect a rich card with
text, material, hit geometry, and contained layout.
