# Google FlatBuffers runtime

The files under `Google/` are derived from Google FlatBuffers v25.12.19
(commit `7e16302`) and are licensed under Apache-2.0. Battlement enables
`ENABLE_SPAN_T` and `UNSAFE_BYTEBUFFER`; bounds checks remain enabled.
The upstream license is reproduced in `LICENSE.txt`.

`FlatBufferVerify.cs` also fixes the upstream identifier predicate and absolute
vtable addressing, and adds a 64 MiB apparent-traversal limit that charges
repeated references.

Upstream source: https://github.com/google/flatbuffers/tree/v25.12.19/net/FlatBuffers
