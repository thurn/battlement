# 06. Extract shared Reactant core, UI layer, and facade — task group

The existing UI behavior runs through the new crate boundaries without a second
component runtime.

This page is an execution index, not one implementation assignment. Complete and
integrate each leaf serially before continuing to the next numbered task.
The leaf pages assign implementation and evidence; linked topic pages define
the shared contract. Keep all existing callers working at every boundary.

**Group prerequisite:** [Task 05: Prepare iOS and Android release validation
paths](05-mobile-build-paths.md) is integrated.

## Assignments

- [06a. Extract the shared host interface inside Reactant](06a-host-boundary.md)
- [06b. Move core, UI, and facade ownership and update callers](06b-crate-cutover.md)

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Validation](../validation.md)
