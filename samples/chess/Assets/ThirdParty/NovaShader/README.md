# NOVA Shader

These assets were created by CyberAgent, Inc.

Source:
https://github.com/CyberAgentGameEntertainment/NovaShader

License:
MIT
https://opensource.org/license/mit

This directory contains only the NOVA Shader 3.6.0 assets required by two
particle effects: `pfb_eff_heal01` from `Assets/Samples/Scenes/Sample08.unity`
and `pfb_eff_thunder_dist` from `Assets/Samples/Scenes/Sample03.unity`. Alongside
their prefabs, materials, textures, and meshes, it includes only the Uber Unlit
particle shader and includes referenced by those materials. Sample03's optional
screen-space distortion layer is disabled because its framebuffer compositor is
not WebGL-compatible. Its two multiplicative background layers are also disabled
because they obscure the chessboard. Their materials, the distortion material,
renderer feature, C# code, editor code, demos, package metadata, and unrelated
runtime code are omitted.

The particle-system child transforms are scaled to 40% so the effect is
approximately the footprint of one square in the Battlement chess sample.
The thunder prefab root is scaled to 10% for the same one-square footprint.

The assets in this directory are distributed under the MIT License and are not
subject to the repository's Apache-2.0 license. The complete license text is in
`Nova/LICENSE.md` and the repository-level `THIRD_PARTY_LICENSES.md`.
