# Promo title sequence

Scripted, deterministic ~6 s title card for promotional videos and as a
fullscreen [durop](https://github.com/ramate-io/durop) overlay transition.

1. Background is a cosmos-forward mix of the existing furniture `rockadder` and `cosmos` (chest-cosimo) shaders. Rockadder is shown at a larger tile scale; nebula washes and a twinkling star field sit on top of the blend.
2. The Maybraid HUD mark flies a high meteor arc with a curved, energy-pulsing tail and spark motes, lands in the bottom-right with a bounce, then spins at the loading-icon rate (`LOADING_SPIN_SECS`).
3. A centered rounded card (`HudTextCard` chrome, `MENU_LIST_RADIUS`) fades in and pulses. The string is the `--label` / `PROMO_TITLE_LABEL` value.
4. A circular swallow expands from the label center and eats the frame. Overlay pixels inside the circle are straight-alpha transparent so whatever is underneath (the desktop, a game, a checkerboard) shows through.

Timeline math lives in [`src/timeline.rs`](src/timeline.rs) and is unit-tested. Capture advances a frozen clock by `1/fps` per submitted frame, dumps `frame_XXXXX.png`, encodes an H.264 preview, punches swallow alpha, then encodes a ProRes 4444 overlay.

## Command

From the repository root, inside `nix develop`:

```bash
./maybraid/promo/title/capture.sh \
  --label "Maybraid" \
  --width 1920 \
  --height 1080 \
  --fps 60 \
  --output title-sequence.mp4
```

That is the only command you need. It builds `promo-title` if missing, runs it under Xvfb + lavapipe (or the current Vulkan device), and writes:

| Path | Contents |
|---|---|
| `title-sequence.mp4` | Six-second H.264 preview (opaque RGB, swallow not punched) |
| `title-sequence-checkerboard.mp4` | Same timeline composited over a checkerboard so the swallow is visible |
| `title-sequence.mov` | ProRes 4444 overlay with swallow alpha (durop asset) |
| `title-sequence_stills/01_background_blend.png` | Shader blend, stars, no fallen star |
| `title-sequence_stills/02_mid_arc_tail.png` | Curved tail + energy motes |
| `title-sequence_stills/03_landing_bounce.png` | Bounce at the bottom-right |
| `title-sequence_stills/04_label_visible.png` | Centered rounded label |
| `title-sequence_stills/05_swallow_mid.png` | Mid-expansion hole composited over a checkerboard |
| `title-sequence_stills/06_swallow_clear.png` | Final fully-transparent frame over a checkerboard |

Equivalent cargo invocation (same flags, no Xvfb wrapper). Prefer this on
macOS: `capture.sh` forces Linux software GL/Vulkan and panics with `Unable to
find a GPU` unless you override `WGPU_BACKEND=metal`.

```bash
cargo run -p promo-title --release -- \
  --label "Maybraid" \
  --width 1920 --height 1080 --fps 60 \
  --output title-sequence.mp4
```

Omit `--output` to play the sequence looping in a window.

Capture writes `{stem}_frames/frame_XXXXX.png`, encodes, then **deletes that
directory** unless `--keep-frames`. Re-encode from the ProRes `.mov`, not from
`frame_%05d.png` in the repo root.

| Flag / env | Default | Role |
|---|---|---|
| `--label` / `PROMO_TITLE_LABEL` | `Maybraid` | Centered card copy |
| `--width` / `PROMO_TITLE_WIDTH` | `1920` | Pixels |
| `--height` / `PROMO_TITLE_HEIGHT` | `1080` | Pixels |
| `--fps` / `PROMO_TITLE_FPS` | `60` | Timeline and encode rate |
| `--output` / `PROMO_TITLE_OUTPUT` | unset | H.264 preview path; also enables capture |
| `--overlay` / `PROMO_TITLE_OVERLAY` | `{stem}.mov` | ProRes 4444 overlay path |
| `--ffmpeg` / `PROMO_TITLE_FFMPEG` | `ffmpeg` | Encoder binary |
| `--keep-frames` | off | Leave `*_frames/frame_XXXXX.png` (RGBA after encode) |
| `--warmup-frames` | `48` | GPU settle before frame 0 |

Software rendering is supported (`WGPU_BACKEND=vulkan` + lavapipe, or `WGPU_BACKEND=gl`).

```bash
cargo test -p promo-title
```

## Durop overlay format

Durop packs are a `durop.json` plus per-clip JSON (`id`, `title`, `asset`, optional `tags` / `playback` / `visual`) and a same-basename PNG icon. The v1 allowlist accepts `mp4`, `mov`, `webm`, and `gif` (among audio types). That is an authoring extension list, not a decoder guarantee.

Native overlay playback on macOS is AVFoundation. The measured alpha fixture in durop is **HEVC-with-alpha MOV** (`AVVideoCodecType.hevcWithAlpha`, `alpha-hevc.mov`). GIF also carries alpha, via a separate decoder. Linux native A/V is unsupported (GIF only).

This capture emits **ProRes 4444 MOV** (`prores_ks` + `yuva444p10le` + `-qscale:v 13`) because that is the alpha-capable allowlisted format the Linux encoder can actually preserve:

- **HEVC-with-alpha** is what durop measured. `libx265` on Linux rejects `yuva420p`; Apple `hevc_videotoolbox` is not available there. Transcode the ProRes file on a Mac (next section).
- **VP9 WebM** is allowlisted, but ffmpeg 6.1.1 `libvpx-vp9` on this VM muxes `yuva420p` as opaque `yuv420p` (decoded center alpha is 255). Unusable as an overlay until that pipeline is proven.
- **H.264 MP4** has no alpha plane; keep it as the standalone preview.
- **GIF** has alpha and works on Linux durop, but 1080p60 of this sequence is the wrong tool. Durop's GIF decoder caps composited RGBA at 128 MiB / 1000 frames (~15 frames of 1080p).

ProRes 4444 is intra-frame 10-bit 4:4:4 plus a full alpha plane on every pixel. At 1920×1080 / 60 fps / 6 s that is typically **200–400 MiB** — often over durop's **256 MiB** per-file catalog cap, and always over GitHub's blob limit. Do not commit it. The swallow punch does not shrink the file: alpha is stored on opaque frames too.

The overlay clip should use `visual.anchor: center`, `widthFraction: 1.0`, `fit: cover` so the swallow fills the screen. Pack authoring files live in [`durop-pack/`](durop-pack/README.md).

## HEVC-with-alpha on a Mac

Durop's catalog allowlists `.mov`; native playback is AVFoundation. The measured alpha fixture is HEVC-with-alpha (`AVVideoCodecType.hevcWithAlpha`, `alpha-hevc.mov`), not ProRes. After capture, transcode the overlay MOV — it already has straight-alpha swallow punched in:

```bash
ffmpeg -y -i title-sequence.mov \
  -c:v hevc_videotoolbox -pix_fmt bgra -alpha_quality 0.75 -tag:v hvc1 \
  title-swallow.mov
```

Paths follow `--output` / `--overlay`. A capture written under `maybraid/art/promo/` looks like:

```bash
ffmpeg -y -i maybraid/art/promo/title-sequence.mov \
  -c:v hevc_videotoolbox -pix_fmt bgra -alpha_quality 0.75 -tag:v hvc1 \
  maybraid/art/promo/title-swallow.mov
```

Point the pack `asset` at that HEVC file (see [`durop-pack/title-swallow.json`](durop-pack/title-swallow.json)) and load the folder as a local durop pack. `.gitignore` already drops `durop-pack/*.mov`; keep generated movies out of git.

Need the PNG sequence for some other encode? Recapture with `--keep-frames`. The frame directory is `{stem}_frames/` next to the MP4, not the current working directory.
