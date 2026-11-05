# Packaged dust

Runtime assets exported from the supplied [eight-view showcase](https://github.com/ramate-io/maybraid/pull/1342)
and [dust authoring delivery](https://github.com/ramate-io/maybraid/pull/1483).
Source commits, checksums and encoding settings are recorded beside the assets.

| Asset | Runtime representation | Storage |
| --- | --- | --- |
| `rolling_wave.ktx2` | Eight sRGB UASTC texture-array layers, 49 frames at 15 Hz, one mip level | 231,916 bytes on disk; 7,250,432 bytes as BC7 |
| `clouds/*.ktx2` | Four static linear R16Float 64³ grids | 2,097,152 bytes decoded in total |
| `bodies/*.ktx2` | Three static linear R16Float 128×112×112 grids | 9,633,792 bytes decoded in total |

The seven volume KTX2 files together occupy 1,239,469 bytes on disk. Zstandard
reduces their storage size; their R16Float GPU representation remains 11,730,944
bytes. The body GLBs, deformation coefficients and authored motion rules are
also retained. Static volumes are available to consumers but are not needed by
the multiview flipbook renderer.

`*.flipbook.json` and `*.volume.json` are versioned manifests loaded by
`maybraid_vfx::packaged::PackagedVfxPlugin`. Texture paths resolve relative to
the manifest through Bevy's AssetServer. Volume matrices are column-major and
map integer indices to local voxel centers; bounds include the half-voxel border.
Grid values are linear density, not color. Flipbook frame zero starts at time
zero and the last authored sample is at 3.2 seconds; sampling clamps at the ends.

Preview the shared runtime from the repository root:

```sh
cargo run -p vfx-playground -- show dust
```

See the [playground commands](../../../vfx/playground/README.md) and
[reproducible export instructions](../../../../bin/vfx/README.md).
The runtime uses the existing asset root; it does not decode source PNGs, repack
texture arrays, or fetch authoring material at startup.

`reference.lock.json` pins the optional original delivery. Its approximately
453 MiB compressed recording is intentionally absent here. The explicit fetch
tool verifies downloaded sizes and SHA-256 hashes. The original source remains
at its pinned commit; this package does not rewrite or remove either source PR.

UASTC is lossy. Target-device review should check thin alpha edges, frame/view
transitions, ground intersections, camera proximity and overlapping effects.
The eight-view bake was authored around a 20° camera elevation; it does not
provide arbitrary-view volumetric lighting or reproduce the separate dynamic
Houdini simulation.
