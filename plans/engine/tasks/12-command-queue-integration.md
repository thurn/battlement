# 12. Connect snapshot rendering to the existing command queue — task group

Render queued snapshots ahead of playback and append ordinary ordered batches.
Unity's existing queue determines when their commands execute.

This page is an execution index, not one implementation assignment. Complete and
integrate each leaf serially before continuing to the next numbered task.
The leaf pages assign implementation and evidence; linked topic pages define
the shared contract. Keep all existing callers working at every boundary.

**Group prerequisite:** [Task 11: Start game sessions, accept actions, and expose
recovery](11-accepted-action-runtime.md) is integrated.

## Assignments

- [12a. Submit snapshot output through ordered gameplay batches](12a-snapshot-batches.md)
- [12b. Bind queued prompt controls and independent menu updates](12b-queued-input.md)

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Validation](../validation.md)
