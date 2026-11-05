# Dust asset packaging

These exporters repackage existing artwork without Houdini. Runtime files live in
`maybraid/assets/vfx/dust`; the full offline simulation and authoring delivery is
an explicit optional download, never a build dependency.

## Tools

Use Python 3 and [KTX-Software 4.4.2](https://github.com/KhronosGroup/KTX-Software/releases/tag/v4.4.2).
Put its `ktx` and `toktx` executables on `PATH`, or pass their paths to the
exporters. Source downloads also require an authenticated GitHub CLI where the
source repository requires access. Exporting already downloaded inputs is offline.

## Eight-view flipbook

From the repository root:

```sh
python3 bin/vfx/export_flipbook.py /tmp/rolling-wave-source --fetch
```

This fetches only the pinned manifest and eight PNGs listed in
`rolling_wave.sources.json`, verifies their hashes, and produces:

- `rolling_wave.ktx2`: eight 952×952 array layers, each containing the same 7×7
  arrangement of 49 time samples at 15 Hz;
- `rolling_wave.flipbook.json`: the small typed runtime contract;
- `rolling_wave.provenance.json`: source identity, encoder settings, checksum,
  and separate disk/decoded/GPU byte counts.

The format is UASTC with Zstandard supercompression, sRGB color and straight
linear alpha. UASTC is an intermediate encoding; Bevy transcodes to a supported
GPU format. BC7 residency is 7,250,432 bytes, while an RGBA8 fallback requires
29,001,728 bytes. The exported file is 231,916 bytes. These are storage sizes,
not frame-time measurements. The checked-in Bevy decoder features support this
array path.

The export uses one mip level. Ordinary atlas-wide mip generation would blend
adjacent animation frames; adding mipmaps requires per-frame filtering, suitable
gutters, and an explicit safe LOD limit. View order, pivot, capture geometry,
frame timing, and alpha convention stay unchanged. Compression is lossy: a
comparison after RGBA8 transcode found a maximum alpha difference of 14/255 and
a maximum per-view mean alpha difference below 0.051/255. Review thin edges on
target devices when changing encoder settings.

## Static density volumes

```sh
python3 bin/vfx/fetch_reference.py --group reusable --destination /tmp/dust-source
python3 bin/vfx/export_volumes.py --source-dir /tmp/dust-source/Combined/Assets
```

Only reusable body and cloud inputs are fetched by the `reusable` group. The
export preserves X-fastest half-float values in linear R16Float 3D KTX2 images,
plus typed volume manifests retaining each voxel-center transform. Body meshes,
deformation data, and motion rules remain available for subsequent renderer
integration. Identical Cartoon/Combined body data is stored once.

The source inventory and optional groups are pinned in
`maybraid/assets/vfx/dust/reference.lock.json`. Inspect the fetcher's `--help`
before explicitly requesting authoring projects or recordings; neither is needed
to run the flipbook playground. Downloaded inputs belong outside the runtime asset
directory. Export validation extracts the KTX2 volume slices and compares their
bytes against the original grids. Storage compression does not reduce the GPU
residency of an R16Float volume.

## Ownership and scope

The runtime uses Bevy AssetServer/AssetLoader dependencies and shared image
handles. Source provenance and optional downloads are separate from runtime
manifests. The dust renderer previews the supplied multiview flipbook; the static
volume exports do not implement the separate Houdini dynamic simulation or its
lighting model.

## Validation

```sh
python3 -m unittest discover -s bin/vfx -p test_export_flipbook.py
KTX=ktx python3 -m unittest discover -s bin/vfx -p test_volumes.py
cargo check --locked -p maybraid-vfx -p vfx-playground
cargo test --locked -p maybraid-vfx -p vfx-playground
```

Validation for this delivery passed 7 flipbook-export tests, 10 volume tests,
42 VFX tests and 5 playground tests. The Rust tests decode the real assets
through Bevy, check shared handles and failure propagation, and exercise
concurrent playback, pause, cancellation and completion. Independent exports
reproduced the checked-in artifacts. Shader validation covered all eight
HDR/depth-prepass/MSAA configurations, followed by a Linux Lavapipe render
smoke with depth prepass and a screenshot from GPU readback.

Metal and Windows visual checks remain outstanding. The software GPU smoke is
not a performance benchmark. Review the playground's documented camera and cold
pipeline limitations when evaluating the artwork.

Related work: [flipbook export #1552](https://github.com/ramate-io/maybraid/issues/1552),
[volume packaging #1553](https://github.com/ramate-io/maybraid/issues/1553),
[typed loading #1554](https://github.com/ramate-io/maybraid/issues/1554), and
[runtime integration #1555](https://github.com/ramate-io/maybraid/issues/1555).
