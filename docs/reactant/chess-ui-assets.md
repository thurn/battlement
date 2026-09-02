# Chess UI assets

The standalone `samples/chess-ui` sample owns its imported assets. Binary inputs
come from `git@github.com:thurn/mockups.git`, commit
`2451ea9cc6f76b356b1102ee37b82c478853122a`. Their bytes are preserved under
`Assets/Original` with the same basenames.

| Original repository path | License |
| --- | --- |
| `public/fonts/barlow-condensed-700.ttf` | SIL Open Font License 1.1; retained in `BarlowCondensed-OFL.txt` |
| `public/fonts/barlow-condensed-800-italic.ttf` | SIL Open Font License 1.1; retained in `BarlowCondensed-OFL.txt` |
| `public/fonts/bebas-neue.ttf` | SIL Open Font License 1.1; retained in `BebasNeue-OFL.txt` |
| `public/audio/drag-and-dread.opus` | No license declaration is supplied by the pinned repository; no additional license is asserted |

The Barlow font metadata identifies the Barlow Project Authors and links to the
SIL Open Font License. The retained license comes from the Reactant sample.
Bebas Neue's embedded copyright identifies the Bebas Neue Project Authors
(2019), and its embedded license URL points to the SIL Open Font License.

The 18 generator declarations are owned by
`samples/chess-ui/rules/src/assets.rs`. Their recipes use the sample's Barlow
Condensed 800 italic file for generated lettering. Generate the assets with
`cargo battlement reactant assets generate --project samples/chess-ui`.
