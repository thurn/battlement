# Battlement UI lab

This standalone Unity project renders the Battlement UI capability lab from a
native Rust rules engine. The integrated screen-space document contains the
persistent navigation, active specimen canvas, and state/event/command
inspector used by the implementation plan.

The sample contains no game-specific C#. Its authored content scene is the
minimal Addressables scene required by the core snapshot contract; the visible
hierarchy is built entirely through public Rust Battlement APIs.
