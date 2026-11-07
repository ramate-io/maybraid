# Maybraid title swallow pack

Local [durop](https://github.com/ramate-io/durop) drop pack for the promo title
sequence. Durop's catalog loader expects `durop.json`, clip JSON, a
same-basename PNG icon, and a nonempty media file named by `asset`.

## Produce the overlay

From the Maybraid repository root:

```bash
./maybraid/promo/title/capture.sh \
  --label "Maybraid" \
  --width 1920 \
  --height 1080 \
  --fps 60 \
  --output /tmp/title-sequence.mp4 \
  --overlay maybraid/promo/title/durop-pack/title-swallow.mov
```

`title-swallow.mov` is ProRes 4444 (`-qscale:v 13`) with straight-alpha swallow.
It is generated, not committed: even the smaller qscale is ~200 MiB at 1080p60,
over GitHub's blob limit, and durop rejects unresolved Git LFS pointers. It
does fit durop's 256 MiB per-file catalog cap.

Load this folder as a **local directory** pack in durop's Catalog menu (or
`--pack /path/to/maybraid/promo/title/durop-pack`). Visual placement is
fullscreen (`center` / `widthFraction` 1 / `cover`) so the circle can eat the
screen.

## Why ProRes 4444

See the crate [README](../README.md#durop-overlay-format). Durop's measured
alpha movie is HEVC-with-alpha; this pack ships ProRes 4444 because that is the
allowlisted MOV this capture can encode with a real alpha plane. On a Mac,
transcode to `hevc_videotoolbox` if you want the measured codec, and point
`asset` at that file instead.
