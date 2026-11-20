# cartoon_explosion

A stylised, anime-style smoke explosion (grenade class), authored procedurally in Houdini (seed 6).
- **Onset:** a burst of dark spikes, then a lumpy dark-brown mass that pops in and churns slowly.
- **Dissolve (4.8–14 s):** the mass loosens and rises. Puffs subdivide twice into smaller puffs and shrink out, so nothing erodes or fades. The last piece is gone by about 13.8 s.

This is an independent effect, not part of `vfx/dust`.

## Files

- `cartoon_explosion.explosion.json`
- `cartoon_explosion.track.bin`
- `meshes/puff_0.glb`
- `meshes/puff_1.glb`
- `meshes/puff_2.glb`
- `meshes/puff_3.glb`
- `meshes/puff_4.glb`
- `meshes/puff_5.glb`
- `meshes/puff_6.glb`
- `meshes/puff_7.glb`
- `meshes/puff_8.glb`
- `meshes/puff_9.glb`
- `meshes/puff_small_0.glb`
- `meshes/puff_small_1.glb`
- `meshes/puff_small_2.glb`
- `meshes/puff_small_3.glb`
- `meshes/puff_small_4.glb`
- `meshes/puff_small_5.glb`
- `meshes/spike_0.glb`
- `reference.json`
- `README.md`
- `cartoon_explosion.provenance.json`

- `cartoon_explosion.explosion.json` is the manifest: elements, meshes, lobes, per-sample lobe centroids and bounds, LOD, wind and budgets.
- `cartoon_explosion.track.bin` is the animation track (sparse; see below).
- `reference.json` holds interpolated element states at 37 query times, computed from the binary for runtime tests.
- `cartoon_explosion.provenance.json` records the Houdini build, script and file hashes, parameters and validation results.

## Space and units

- Metres, right-handed, **+Y up**, ground at Y = 0, origin = detonation point.
- Authored height is about 5 m at full development; the dissolve rises to about 9.4 m.
- Larger explosions are made by scaling the whole effect uniformly at runtime.
- Key light `L = normalize([-3.8, 5.5, 3.0])`.

## Track

- **Rate and duration:** 30 Hz, 14.0 s, 421 sample times `t = i / 30`.
- **Layout:** sparse and element-major. For each element in id order, the file holds `sample_count` × 16 little-endian f32, for samples `first_sample … first_sample + sample_count − 1`, starting at `byte_offset`. There is no header.
  - Total size: `byte_length` = Σ sample_count × 64 = 2,970,304 bytes.
- **Fields:** `px, py, pz, qx, qy, qz, qw, sx, sy, sz, dissolve, tone, ox, oy, oz, openness`.
  - The quaternion is in Bevy order and rotates mesh local axes into effect space.
  - World transform = T · R · S, with scale along the mesh's local axes.
  - Quaternions are hemisphere-continuous within each element's range.
- **Existence:** an element exists only for `first_sample/rate ≤ t ≤ (first_sample + sample_count − 1)/rate`; outside that, do not draw it.
  - The first and last stored samples are pads at scale 0 with dissolve 1, where the timeline allows. Births, deaths and subdivision swaps are therefore seamless under interpolation.
  - Every element has ≥ 2 samples.
- **Interpolation inside the range:** linear for position, scale, dissolve, tone, open_dir and openness; nlerp for rotation. Global time is clamped to [0, 14.0].
- **Per-element manifest fields:**
  - `kind` (`puff` | `spike`), `mesh` (index into `meshes`), `lobe` (null for spikes), `lod_rank`, `lod_scale` {medium, low};
  - `generation`: 0 = original; 1 or 2 = the subdivision phase the element was born in. Phase 1 splits originals; phase 2 splits phase-1 children and originals that did not split in phase 1, so a generation-2 child's parent can be generation 0 or 1.
  - `parent`: id or null.
  - `birth_s`, `death_s`, `first_sample`, `sample_count`, `byte_offset`.
- **Observed counts:** 349 elements (166 generation-0 puffs and spikes, 90 generation-1 children, 93 generation-2 children).
  - Peak alive: 172 puffs (cap 200) and 30 spikes (cap 32).
  - Peak visible triangles: 118,452 (budget 120,000).

### Subdivision

- A split happens at a sample boundary. The parent's range ends on a zero-scale pad, and its 2–4 children start there, at 0.5–0.6 × the parent's scale.
- Children sit on the parent's medium sub-lump centres (`meshes[].sub_lump_centres`, informational) and share the parent's orientation.
- No runtime logic is needed: draw whatever exists at `t`.

### Wind

- The effect is authored with wind along `[1.0, 0.0, 0.0]` (drift up to 0.08 m/s at the top).
- To follow the game's wind, the runtime may **only rotate the whole effect about +Y**. Everything else is baked.

## Meshes

- One mesh per `.glb` (single node, single mesh, single primitive): `…/file.glb#Mesh0/Primitive0`.
- Attributes are `POSITION`, `NORMAL` (smooth) and `COLOR_0` (float RGBA), in metres, Y-up, with CCW exterior winding.
- **Puffs:** 10 large variants (`puff_0..9`, ≤ 1,200 tris) and 6 small low-poly variants (`puff_small_0..5`, ≤ 300 tris, 3–5 sub-lumps).
  - The origin is the mesh centre, with bounding radius 0.5, so scale `s` means a diameter of `s` metres.
  - The small variants are used for generation-2 children and for small late-born puffs.
- **Spike:** `spike_0` has its base at the origin and runs along local +Y with length 1 (≤ 128 tris). Scale Y sets the length.
- **`COLOR_0` channels:**
  - R = `self_ao`, the mesh's baked ambient occlusion, used as a lighting term.
  - G = dissolve noise. For spikes it is a tip→base gradient mixed with noise. For puffs it is unused, but valid.
  - B = 0 and A = 1.

## dissolve, tone, open_dir, openness

- **`dissolve`:** puffs are never discarded, and `dissolve` is 0 for every visible puff sample.
  - Spikes are discarded where `COLOR_0.G < dissolve`; this is the onset retraction, tip first.
  - Dead pad samples carry dissolve 1.
- **`tone`:** 0 = initial light grey-brown palette, 1 = settled dark brown. Blend the palettes by `tone`.
- **`open_dir` / `openness`:** `open_dir` is the element's least-occluded direction (unit vector). `openness` runs from 0 to 1, where 1 is fully exposed.

## Shading (copy exactly)

Inputs:
- `N_local`, the mesh normal;
- `N_macro = normalize(P − lobe_centroid)` for puffs (the element's lobe, from `lobe_centroids[sample][lobe]`, interpolated linearly), or the mesh normal for spikes;
- `L`; `open_dir` and `openness`; `self_ao = COLOR_0.R`;
- `h = P.y / mass_height`, with `mass_height` per sample in the manifest (interpolated linearly);
- `tone`.

Back faces are culled, so the meshes render single-sided. There are no outlines or ink lines.

```
N    = normalize(mix(N_local, N_macro, 0.4))
m    = dot(normalize(mix(N_local, N_macro, 0.8)), L)
d    = dot(N, L)
band = m > 0.0 ? mid : shadow
if (band == mid && d > 0.72) band = lit
c    = (1 - openness) * saturate(-dot(N_local, open_dir));  if (c > 0.08) band = shadow
if (self_ao < 0.65) band = one band darker
if (h < 0.1) band = shadow  else if (h < 0.24) band = min(band, mid)
spikes: band = min(band, mid)
colour = mix(initial[band], settled[band], tone)
```

Palettes (sRGB; convert to linear before blending):
- settled: shadow `#2A2420`, mid `#3D332D`, lit `#5A4C44`;
- initial: shadow `#3F3833`, mid `#574D46`, lit `#776A61`.

The review renders use a ground of `#7A8A5A` and a background of `#808080`.

## LOD

- `lod_presets` = full 136 / medium 60 / low 25.
- draw puffs whose lod_rank < preset (subdivision children carry their root parent's rank) and multiply their scale by elements[].lod_scale[preset]; spikes always drawn.

## Regenerate

From the authoring folder (`~/Desktop/VFX_1/houdini/cartoon_explosion_01/`):

```
hython build_cartoon_explosion.py --seed N --stage hip          # scene
python3 render_farm.py --sets contact,seq,dissolve              # Karma reference renders (resumable)
./export_all.sh <this folder>                                   # meshes, track, manifest, reference, validation, provenance
```

## Known limitations and deviations

- **Schema id.** The schema id stays `maybraid.cartoon-explosion.v1`, but the contract follows Revisions 01–04: 16 floats per sample, lobes instead of groups, a 14 s sparse element-major track, `generation` / `parent`, small variants, and no erosion block.
- **Export steps.** The export runs as three steps behind `export_all.sh`: hython for the meshes, then plain Python for the data and the validation. The original brief asked for a single build script.
- **0–4.8 s vs v3.** The first 4.8 s match the approved v3 motion up to 4.667 s. In 4.7–4.8 s, v3 had already started eroding a few puffs, and Revision 04 forbids puff erosion.
- **Area rise.** During the loosen, about 4.8–7 s, the silhouette area rises by about 17–20% before the shrink-outs reduce it.
- **Subdivision swaps.** The pieces keep the parent's top. In each review camera, the leading child is raised to reach the parent's screen-space extent, so a split never lowers the visible top.
- **Small pieces.** The pieces under 0.12 m appear only in the last 0.28 s of their own shrink-out.
