# Promo title sequence

Scripted, deterministic ~5 s title card for promotional videos.

1. Background is a mix of the existing furniture `rockadder` and `cosmos` (chest-cosimo) shaders (not a reimplementation).
2. The Maybraid HUD mark flies in as a fallen star with a sampled tail, lands in the bottom-right with a bounce, then spins at the loading-icon rate (`LOADING_SPIN_SECS`).
3. A pulsing menu header (Barlow Black, `HEADER_FONT_SIZE`, `TEXT_YELLOW`) fades in. The string is the `--label` / `PROMO_TITLE_LABEL` value.

Timeline math lives in [`src/timeline.rs`](src/timeline.rs) and is unit-tested. Capture advances a frozen clock by `1/fps` per submitted frame, dumps `frame_XXXXX.png`, then encodes with `ffmpeg`.

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
| `title-sequence.mp4` | Five-second H.264 video |
| `title-sequence_stills/01_background_blend.png` | Both shaders, star still off-screen |
| `title-sequence_stills/02_mid_flight_star.png` | Star + tail |
| `title-sequence_stills/03_landing_bounce.png` | Bounce at the bottom-right |
| `title-sequence_stills/04_label_visible.png` | Pulsing header |

Equivalent cargo invocation (same flags, no Xvfb wrapper):

```bash
cargo run -p promo-title --release -- \
  --label "Maybraid" \
  --width 1920 --height 1080 --fps 60 \
  --output title-sequence.mp4
```

Omit `--output` to play the sequence looping in a window.

| Flag / env | Default | Role |
|---|---|---|
| `--label` / `PROMO_TITLE_LABEL` | `Maybraid` | Header copy |
| `--width` / `PROMO_TITLE_WIDTH` | `1920` | Pixels |
| `--height` / `PROMO_TITLE_HEIGHT` | `1080` | Pixels |
| `--fps` / `PROMO_TITLE_FPS` | `60` | Timeline and encode rate |
| `--output` / `PROMO_TITLE_OUTPUT` | unset | MP4 path; also enables capture |
| `--ffmpeg` / `PROMO_TITLE_FFMPEG` | `ffmpeg` | Encoder binary |
| `--keep-frames` | off | Leave `*_frames/frame_XXXXX.png` |
| `--warmup-frames` | `48` | GPU settle before frame 0 |

Software rendering is supported (`WGPU_BACKEND=vulkan` + lavapipe, or `WGPU_BACKEND=gl`).

```bash
cargo test -p promo-title
```
