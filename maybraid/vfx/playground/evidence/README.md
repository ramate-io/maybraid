# Playground evidence stills

Capture with the command drawer hidden (`VFX_CAPTURE=1`). The playground driver
writes [`SonicWaveState`](../../lib/src/drive.rs) snapshots; the effect only
renders them.

```text
# God Beam: full 7 m × 96 m column, then character-scale close-up
VFX_CAPTURE=1 VFX_FREEZE=1.6 VFX_FRAME=full  cargo run -p vfx-playground -- god-beam --width 7 --height 96 --lifetime 10 --seed 2
VFX_CAPTURE=1 VFX_FREEZE=1.6 VFX_FRAME=base  cargo run -p vfx-playground -- god-beam --width 7 --height 96 --lifetime 10 --seed 2

# Sonic: three rings leaving a mouth-height placeholder head
# Boss (20 m) side + 3/4
# thickness = 2 * WavefrontSpec.half_depth (authored 0.35 → 0.70)
VFX_CAPTURE=1 VFX_FREEZE=0.72 VFX_FRAME=side            cargo run -p vfx-playground -- sonic-wave --range 20 --speed 22 --count 3 --radius 0.85 --thickness 0.70 --interval 0.28 --seed 2
VFX_CAPTURE=1 VFX_FREEZE=0.72 VFX_FRAME=three-quarter   cargo run -p vfx-playground -- sonic-wave --range 20 --speed 22 --count 3 --radius 0.85 --thickness 0.70 --interval 0.28 --seed 2

# Ordinary scale side + 3/4
VFX_CAPTURE=1 VFX_FREEZE=0.42 VFX_FRAME=side            cargo run -p vfx-playground -- sonic-wave --range 8 --speed 16 --count 3 --radius 0.4 --thickness 0.40 --interval 0.18 --seed 2
VFX_CAPTURE=1 VFX_FREEZE=0.42 VFX_FRAME=three-quarter   cargo run -p vfx-playground -- sonic-wave --range 8 --speed 16 --count 3 --radius 0.4 --thickness 0.40 --interval 0.18 --seed 2
```

Checked-in stills (console hidden):

- `god_beam_7m_full.png` — 7 m × 96 m column, ground to top
- `god_beam_7m_scale.png` — same beam with the 1.8 m stand-in
- `sonic_boss_20m_side.png` / `sonic_boss_20m_three_quarter.png` — three rings from a boss-scale mouth
- `sonic_ordinary_8m_side.png` / `sonic_ordinary_8m_three_quarter.png` — same burst at ordinary scale

`VFX_CANCEL_AT` stills the God Beam fade-from-cancel path.

## Motion clips

Short headless videos (lavapipe + xvfb). Requires `mesa-vulkan-drivers`,
`libxkbcommon-x11-0`, and `ffmpeg`:

```bash
./maybraid/vfx/playground/evidence/capture-motion.sh
```

Checked-in `.mp4` files cover boss/ordinary sonic (side and 3/4, day and night),
sequential-wave cancel (`VFX_CANCEL_AT`), and God Beam 7 m column / scale reference.
