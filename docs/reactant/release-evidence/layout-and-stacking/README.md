# Reactant Layout and Stacking release evidence

This directory is the retained release record for Reactant Layout and Stacking.
The reference revision predates Task 1, and the final revision is the Task 10
candidate identified as `self` until Tollgate certifies it.

The checked-in gallery exercises the public Flex, Grid, Stack, Sticky, Overlay,
presence, ref, event, key, and layout-animation facades. Its black-box flow
changes responsive track lists, preserves keyed component state across those
changes and reconnect, routes events through a portaled clipped menu, and
authors modal focus restoration through public refs.

The fixed performance fixture contains 1,000 Grid children, 100 sticky rows,
12 nested Stack specimens, and 10 anchored overlays. Unity EditMode validation
records a single dirty layout pass, 2,000 item-axis measurements, stable-frame
silence, zero allocation while polling settled layout, zero allocation across
100 settled overlay refreshes, and no Rust traffic in the native scroll and
overlay path.

`environment.json` identifies the tested platform. `performance.json` records
the fixed workload and counters. `manual-qa.json` maps the design's twelve-item
checklist to retained automation and interaction evidence. `release-checklist.json`
records the required build, documentation, and clean-session gates.
