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

Capture writes **ProRes 4444** (`-qscale:v 13`) with straight-alpha swallow.
That file is generated, not committed: 1080p60 is ~200–400 MiB, over GitHub's
blob limit, and often over durop's 256 MiB per-file catalog cap. Durop also
rejects unresolved Git LFS pointers.

On a Mac, transcode the ProRes overlay to the codec durop actually measured
(HEVC-with-alpha). Do not point ffmpeg at `frame_%05d.png`: capture deletes
that directory unless you passed `--keep-frames`.

```bash
ffmpeg -y -i title-swallow.mov \
  -c:v hevc_videotoolbox -pix_fmt bgra -alpha_quality 0.75 -tag:v hvc1 \
  title-swallow-hevc.mov
mv title-swallow-hevc.mov title-swallow.mov
```

If capture wrote a different overlay path (for example
`maybraid/art/promo/title-sequence.mov`), use that as `-i` and write
`title-swallow.mov` here so it matches [`title-swallow.json`](title-swallow.json)
`asset`.

Load this folder as a **local directory** pack in durop's Catalog menu (or
`--pack /path/to/maybraid/promo/title/durop-pack`). Visual placement is
fullscreen (`center` / `widthFraction` 1 / `cover`) so the circle can eat the
screen.

## Why ProRes 4444, then HEVC

See the crate [README](../README.md#hevc-with-alpha-on-a-mac). Linux capture
cannot emit HEVC-with-alpha (`libx265` rejects `yuva420p`; Videotoolbox is
Apple-only), so the recorder keeps ProRes. Durop's allowlist includes `.mov`;
native decode is AVFoundation, and the proven alpha fixture is
`AVVideoCodecType.hevcWithAlpha`. Point `asset` at the transcoded file.
