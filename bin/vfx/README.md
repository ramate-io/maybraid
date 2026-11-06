# Dust asset packaging

Pack newly authored assets from local exports before committing the runtime
results. No initial GitHub push or previous commit is required for that workflow.
Runtime files live in `maybraid/assets/vfx/dust`; large authoring projects and
offline recordings are archived separately, never fetched by an ordinary build.

The Python scripts here currently reproduce a specific committed dust delivery.
They validate its pinned hashes and layout, including when reading local files.
They are not yet general exporters for newly authored assets. The sections below
distinguish the local authoring workflow from those migration commands.

## Tools

Use Python 3 and [KTX-Software 4.4.2](https://github.com/KhronosGroup/KTX-Software/releases/tag/v4.4.2).
Put its `ktx` and `toktx` executables on `PATH`, or pass their paths to the
exporters. Packing local images or grids does not require Houdini or GitHub access;
Houdini is used to author and render the inputs. The committed flipbook migration
uses GitHub CLI for downloads; the volume fetcher uses pinned GitHub raw URLs.

## Packing new

For an artist who has just authored an effect in Houdini:

1. Export the rendered views as local **PNG atlases with alpha**. JPEG does not
   preserve transparency. Keep every view's frame dimensions, grid, gutters and
   frame order consistent. Preserve the intended color space and alpha convention.
2. Record the export contract: frame count and timing, atlas dimensions and
   gutters, view directions, capture size, pivot and bounds. Keep this metadata
   with the local source images. The current capture-plane renderer expects uniformly
   timed frames and an evenly spaced ring of views at one elevation, with array
   layers in consecutive azimuth order.
3. Pack those local atlases with KTX-Software into a KTX2 texture array, one layer
   per captured view. The current runtime supports sRGB RGBA/UASTC flipbooks;
   use UASTC with Zstandard for the existing compressed path and start with one
   mip level to avoid cross-frame bleeding.
4. Create the matching `.flipbook.json` runtime manifest and record source hashes
   and encoder settings in separate provenance metadata. See the
   [manifest types](../../maybraid/vfx/lib/src/packaged/manifest.rs) and the
   [existing flipbook manifest](../../maybraid/assets/vfx/dust/rolling_wave.flipbook.json)
   for the required fields. New source hashes describe the new local inputs;
   they do not require a Git commit to exist first.
5. Wire the new manifest into a `FlipbookPart` / `Dust` recipe and preview it in
   `vfx-playground`. Check frame order, alpha edges, view transitions, pivot,
   timing and ground intersections. The current `show dust` preset loads
   `rolling_wave.flipbook.json`; it does not automatically discover new filenames.
6. Commit the validated KTX2, runtime manifest and provenance together. Archive
   Houdini projects, source renders and simulation caches separately as needed.
   A commit or archive reference can be added to provenance once it exists.

**Current tooling boundary:** steps 3–4 use KTX-Software directly and require
authoring the runtime metadata. There is no generic local-input CLI in this
directory yet. `export_flipbook.py` hardcodes the original delivery's layout,
output names and hashes; omitting `--fetch` only disables downloading. New renders
will still fail its validation. `export_volumes.py` likewise validates the pinned
reusable volume delivery. Do not push new images just to work around those checks.
A general local packer should derive names/layout/timing from an input manifest
and compute provenance from the local files, with fetching kept separate.

For newly authored static volumes, export the density grids with their dimensions,
voxel order, bounds and voxel-center transforms. Pack linear half-float density
as R16Float 3D KTX2 and provide matching `.volume.json` metadata. Verify that raw
extraction reproduces the original grid bytes. These assets are separate from
rendered PNG flipbooks; the existing dust preset does not render arbitrary volumes.

## Migrating from committed

Use this less-common workflow to reproduce or migrate the existing delivery from
its immutable GitHub source. The pins provide reproducibility for already
committed assets; they are not a prerequisite for authoring new ones.

### Eight-view flipbook

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

If those exact inputs are already present locally, omit `--fetch` to run offline:

```sh
python3 bin/vfx/export_flipbook.py /tmp/rolling-wave-source
```

Both forms enforce the same pinned source hashes and fixed delivery contract.

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

### Static density volumes

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
