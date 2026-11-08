# Catalog validation

Validated 2026-11-07 (America/Los_Angeles) against upstream
[Durop `6334668`](https://github.com/ramate-io/durop/commit/6334668b340b82bc60aab52f21ac8f80991f34d4)
with Rust 1.97.1, FFmpeg/ffprobe and Pillow on Linux.

- All 26 clip records pass Durop's `schemas/clip-v1.schema.json`.
- Durop's actual `load_local` accepts all 26 clips and unique stable identities.
- `CatalogCache::refresh(PackSource::LocalDirectory(...))` prepares all clips;
  the resulting pin retains the same revision. Cached media and icon bytes match
  the source files, and `load_local` successfully reloads the cached pack offline.
- All 26 media files fully decode through FFmpeg; all 26 PNG icons decode through
  Pillow. Thumbnails are 320 pixels wide; visual frames retain their aspect ratio
  and audio waveforms are 320 × 180. No title bars or card padding are baked in.
  The contact sheet was visually inspected.
- All 11 visual records exactly match their original templates: `bottom-right`,
  `widthFraction: 0.25`, `fit: contain`. This includes the audio-only level-up clip
  and all five replacement videos.
- `SHA256SUMS` verifies the 52 bundled media/icon files. Media totals 37,761,923
  bytes. No media or icon is a Git LFS pointer; `.gitattributes` disables filters
  for the pack so these files remain ordinary Git blobs.
- Upstream `cargo test -p durop-catalog --locked`: 17 passed, 0 failed,
  2 network acceptance tests ignored by upstream defaults.
- `git diff --check` passes.

The pack retains Durop's 10-second playback cap, once-only playback and restart
on retrigger. Longer source files remain intact; the host limits playback.
Catalog validation does not test macOS audio output, native video codecs,
green-screen compositing, or live overlay placement on a recording display.
The Pexels alternatives contain stock reactions rather than the original meme
dialogue, music or political figures; see [SOURCES.md](SOURCES.md).

To verify downloaded bytes from this directory:

```sh
sha256sum -c SHA256SUMS
```

To run the upstream catalog checks from a Durop checkout:

```sh
cargo test -p durop-catalog --locked
```

For playback acceptance on the recording machine, launch Durop with the absolute
path to this `pack/`, trigger each clip, and inspect the lower-right overlay.

Thumbnail styling updated 2026-11-08 (America/Los_Angeles) to match Durop’s base
reactions pack. Regenerated all 26 icons with `generate_icons.py`, refreshed
checksums, decoded all PNGs, and reloaded the catalog through Durop.

## Airhorn and title-swallow additions — 2026-11-08

Merged main at `9f9fbf658` (including PR #1563). The catalog now contains 28
clips: the previous 26, DJ airhorn from Pixabay, and the six-second title swallow
copied byte-for-byte from `maybraid/art/promo/title-swallow.mov`. The title uses
its upstream fullscreen center / width 1 / cover placement and 6500 ms cap;
the eleven original bottom-right defaults remain unchanged.

All 28 records pass the upstream schema and match their placement templates.
All 28 PNGs decode at 320 pixels wide. FFmpeg fully decodes both added media
files; the other 26 media files retain their previously validated bytes. Durop's
Rust loader, cache preparation, pinning, unchanged cached bytes, and offline
reload pass with all 28 clips. The 56 media/icon checksums pass; media totals
59,156,199 bytes. `git diff --check` passes. All files remain ordinary Git blobs.

The source title MOV is 3840 × 2160, 60 fps, six seconds, encoded with Apple's
`hevc_videotoolbox`. It is bundled unchanged to preserve its alpha encoding.
Linux FFmpeg validates the HEVC color stream; macOS AVFoundation alpha playback
and the live fullscreen swallow still require acceptance on the recording Mac.
The title thumbnail samples the visible title card at 3.9 seconds.
