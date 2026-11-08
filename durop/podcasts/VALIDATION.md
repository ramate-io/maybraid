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
  Pillow and have dimensions 512 × 512. The contact sheet was visually inspected.
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
