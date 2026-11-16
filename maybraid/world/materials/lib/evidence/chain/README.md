# Mythical chain evidence ([#1145](https://github.com/ramate-io/maybraid/pull/1145))

Stills and video for the painterly rewrite. Captured on lavapipe (software
Vulkan) with:

```text
export CXX=g++-13 CC=gcc-13 LIBRARY_PATH=/usr/lib/gcc/x86_64-linux-gnu/13
export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json
export WGPU_BACKEND=vk
xvfb-run -a cargo run -p world-materials-playground --release -- chain capture
xvfb-run -a cargo run -p world-materials-playground --release -- chain video /tmp/chain-video
ffmpeg -y -framerate 30 -i /tmp/chain-video/frames/f%04d.png \
  -c:v libx264 -pix_fmt yuv420p mythical_chain_flow.mp4
```

Video cadence is **30 Hz**. After shader warmup the playground inserts
`TimeUpdateStrategy::ManualDuration(1/30 s)` so simulation time and frame
index stay locked. The 10.13 s / 304-frame reel is **fixed damage, no hits**.
Hits are applied after that reel so they do not strobe the transit. Stills
freeze virtual time after warmup so every damage plate shares one flow phase.
Do not infer smoothness from a sparser dump.

Energy/halo rebalance (recipe, not shader hard-codes): glow
`(0.02, 0.90, 0.78)`, `glow_intensity 3.6`, `conduit_frequency 1.6`,
`flow.baseline 0.18`, `flow.peak 1.0`, `halo_width_m 0.016`,
`halo_gain 0.78`. Halo presence is `authored gain × one energy`; additive
RGB is compressed so overlapping links stay turquoise.

## Stills

| File | What it shows |
|------|----------------|
| `damage_00_day.png` … `damage_100_day.png` | Same seed, camera, lighting, flow phase; 0 / 25 / 50 / 75 / 100% local structural loss |
| `damage_*_day_no_emit.png` | Same strip with emission and halo off — missing sectors still read |
| `pair_night.png` | Night at gameplay distance with a wood cube beside the chain |
| `pair_day_sharp.png` / `pair_day_retro.png` / `pair_day_antediluvian.png` | Real `VisualStylePlugin` offscreen path (`FollowCamera` + nearest-neighbor blit). Window resize is not used. |
| `released_day.png` | Fade 1 removes body and halo on the left chain |
| `flow_transit.png` | Mid-reel still from the 30 Hz capture (same locked dt) |
| `hit_link6.png` | After three settled 0.22 hits on left link 6; right chain unchanged |
| `mythical_chain_flow.mp4` | 10.13 s / 304 frames / 30 Hz. Fixed damage, no hits. Chain + tube, bent and moving. |

## Cost (lavapipe, not Apple GPU)

| Item | This rewrite | Prior HEAD on this PR |
|------|----------------|------------------------|
| Scene | 2 × 14-link chains + 1 continuous tube + 1 skinned tube + 1 cube | 2 × 14-link chains |
| Window | 1280×720 | 1280×720 |
| Halo sync after settle | 0 material asset rewrites (`settled_chains_stop_modifying_halo_assets`) | halo cloned / rewritten every `Update` |
| GPU | lavapipe ICD | lavapipe ICD |

Timings on lavapipe are not comparable to Metal. The useful CPU change is
that settled chains no longer rewrite material assets every frame.

## Limitations

- Discarded sectors have no solid interior faces (double-sided darkened backsides).
- Placeholder-chain `flow_s` is rest-Y projection, not a torus geodesic.
- Lavapipe is a software renderer; Retro / Antediluvian stills are the
  play-distance readability evidence, not Apple GPU cost.
- Halo stays a glancing Fresnel shell. It brightens with the surge and is
  quiet at baseline; thin tubes still pick up a rim at grazing angles.
