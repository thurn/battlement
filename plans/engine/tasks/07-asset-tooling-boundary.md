# 07. Unify Reactant project tooling under `rt` — task group

Reactant users have one general `rt` command for project workflows. Battlement
keeps reusable lower-level build support, while this repository's `just`
recipes own sample selection and defaults.

This page is an execution index, not one implementation assignment. Complete and
integrate each leaf serially before continuing to the next numbered task.
The leaf pages assign implementation and evidence; linked topic pages define
the shared contract. Keep all existing callers working at every boundary.

**Group prerequisite:** [Task 06: Extract shared Reactant core, UI layer, and
facade](06-crate-boundaries.md) is integrated.

## Assignments

- [07a. Extract reusable tooling and Unity editor ownership](07a-tooling-libraries.md)
- [07b. Add standalone rt project resolution and shared build/run/author](07b-project-build-run.md)
- [07c. Expose assets, Addressables, plugins, and Ditto through rt](07c-rt-support-commands.md)
- [07d. Switch repository callers and remove superseded executables](07d-tooling-cutover.md)

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Validation](../validation.md)
