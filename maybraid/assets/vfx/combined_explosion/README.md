# combined_explosion

Combined (semi-volume) appearance of the cartoon explosion: a filled 3D density body that
follows the cartoon puffs exactly, a restrained analytic fine-dust layer, thin volumetric spike
jets and a small grain burst, composed as one capped optical medium. It is an independent
effect; it does not change or merge into the dust `combined`.

| File | Content |
| --- | --- |
| `fields/<variant>.ktx2` | 16 static linear R16Float rest-density grids (10 × 64³ large puffs, 6 × 32³ small puffs), Zstandard-supercompressed |
| `fields/<variant>.volume.json` | Volume manifests (same conventions as `dust/bodies/*.volume.json`: column-major index-to-local voxel-centre matrix, bounds including the half-voxel border) |
| `combined_explosion.combined.json` | References to the cartoon manifest/track, the field per mesh index, and every appearance rule and constant |
| `combined_explosion.grains.json` | Explicit grain birth table (portable hash, no RNG to reproduce) and its integrator rules |
| `reference.json` | Density probes (world points × times; main / fine / composed) for Rust tests |
| `combined_explosion.provenance.json` | Input and source hashes, encoder settings, output checksums |

The 16 KTX2 files occupy 1388162 bytes on disk; their R16Float GPU representation is
5636096 bytes. Grid values are linear density in the puff's normalised rest frame
(bounding radius 0.5), not colour.

Motion is not stored here. Every element's position, orientation, scale, birth, death and
dissolve come from the cartoon track in `../cartoon_explosion/`; the rules index that track by
element id and add only appearance (noise frames, concentration, fine patches, spikes, optics).

Runtime outline, per frame:

1. For each alive puff, map world samples to rest space with the track transform, sample its
   field and apply the remap, noise frame and concentration `c(t)`.
2. Add the fine patches (attached, or released after their parent splits) and the spike jets.
3. Sum all contributions uncapped, apply the single smooth cap, then integrate with thickness 3
   and the directional light model, recomputed from the current density.

The authoring renders, validation and open questions are in the Checkpoint A review of the
Houdini project recorded in the provenance file.
