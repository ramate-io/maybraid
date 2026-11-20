# VFX Playground

Interactive previews for the reusable `maybraid-vfx` library. See the [usage specification](../README.md) for game integration, input contracts and limits.

Run from the repository root. The commands below use the optimized local profile:

```sh
cargo run --locked --profile perf --config 'profile.perf.debug=1' -p vfx-playground -- show combined-landing --seed 13
```

Replace the arguments after `--` with an entry below. Press `/` in the window to enter commands without restarting.

## Effects

| Purpose | Command arguments |
|---|---|
| Landing | `show cartoon-landing --seed 13` / `show combined-landing --seed 13` |
| RollingWave | `show cartoon-wave --seed 13` / `show combined-wave --seed 13` |
| Landing with contact | `show cartoon-landing-contact --seed 13` / `show combined-landing-contact --seed 13` |
| Wave with contact | `show cartoon-wave-contact --seed 13` / `show combined-wave-contact --seed 13` |
| Herd Trail | `herd-dust --mode trail --style cartoon --count 10 --seed 13 --mixed-heights` |
| Herd Cloud | `herd-dust --mode cloud --style combined --count 10 --seed 13 --mixed-heights` |
| Cartoon smoke explosion | `show cartoon-explosion --heading 0 --seed 6 --lod full` |
| Combined smoke explosion | `show combined-explosion --heading 0 --seed 6 --quality high` |
| Legacy baked dust | `show dust` |
| Fire explosion / individual layers | `fiery-explosion` / `show flash` / `show fireball` / `show smoke` / `show sparks` |

Herd supports either `--style cartoon` or `--style combined` in both modes. Styles never substitute for each other. `combined-dust` aliases `combined-wave`; `rolling-wave` retains its older meaning of baked `dust`.

## Controls

- WASD, Space/Shift and mouse look move the camera; `orbit` enables orbit inspection.
- `pause`, `play`, `freeze AGE`, `replay-at AGE`, `reset`, `cancel`, `noloop` control playback. One-shot `show` previews repeat by default; this is a host feature, not an authored loop.
- `inspect main|side|top|close|inside` selects dust views. Interior inspection requires a live Combined body.
- `inspect explosion-front|explosion-three-quarter|explosion-close|explosion-inside|explosion-depth` selects smoke-explosion views.
- `--scale`, `--playback`, `--seed` configure one-shot previews where supported. Contact requires scale 1. Negative numeric show options use `=`, for example `--heading=-90`.
- `screenshot /absolute/path.png` saves a frame. Keep captures outside the source tree. Warm once and reset before comparing onset frames.

## Contact regions

```sh
cargo run --locked --profile perf --config 'profile.perf.debug=1' -p vfx-playground -- contact-regions --effect combined-wave-contact --position -12 8 18 --yaw 37 --count 2 --spacing 12 --seed 13
```

Select any of the four `*-contact` effects. `--spacing 1.5` tests overlapping regions; `--cancel-first-at 2.3` tests independent cancellation. `--close` / `--side` choose relative cameras. The cyan box is the fixed local flow region.

In origin previews, `interaction --scenario cross|stop|fast|grazing|static|none` selects actor motion. Arrow keys and Page Up/Down move the interaction proxy; WASD still controls the camera. Region placements remain fixed after spawn. Floors are horizontal; scale, pitch/roll and moving roots are unsupported.

## Travelling Herd

The default travel course starts at `(20,4,-15)`, heading 37 degrees. It crosses slopes and a smoothed drop, turns, stops and resumes without recycling actors.

```sh
cargo run --locked --profile perf --config 'profile.perf.debug=1' -p vfx-playground -- herd-dust --mode cloud --style combined --count 20 --herds 2 --mixed-heights --diagnostics --teleport-at 22
```

- `--start X Y Z`, `--heading DEGREES` place the course.
- `--herds 1|2` creates independent groups; counts are per herd.
- `--course flat` selects the fixed flat-ground compatibility course.
- `--settling-accel` / `--settling-speed` control gentle main-body settling (defaults 0.12 in effect-age units).
- `herd-control off|on` stops/resumes births; released dust continues independently.
- `herd-control cartoon|combined` changes new births only.
- `herd-control cancel-first` removes one herd.
- `inspect herd` / `herd-control overview` restores the following camera; manual fly movement stops following.

The cyan field moves with active emission coverage; dust stays in world space. Orange boxes show birth-tile ownership. Real game callers must supply movement and terrain through the library interfaces; these proxies are demonstrations, not animal AI or foot events.

## Style comparisons and tuning

`show landing-compare`, `wave-compare`, `landing-contact-compare`, and `wave-contact-compare` play matched sequential styles. `show explosion-compare --heading 0` displays both smoke explosions together.

`combined --layer all|body|clouds|grains --quality default|low` configures dust diagnostics. New Landing/contact recipes also support `dust-appearance --body-concentration 1 --fine-concentration 1 --cloud-rate 5 --grain-rate 3`.

Herd supports `--body-density`, `--fine-density`, `--coverage`, `--budget`, `--cloud-budget`, and `--grain-budget`. `--spacing`, `--size`, and `--lifetime` tune Trail; Cloud retains its measured formation hierarchy.

Smoke explosions use `--lod full|medium|low` for Cartoon and `--quality high|medium|low` for Combined. `--diagnostic main|fine|spikes|section|composed` isolates Combined density.

## Validation

```sh
cargo test --locked -p maybraid-vfx -p vfx-playground
```

Resource and rendering limits are listed in the [usage specification](../README.md#budgets-and-limits). Multiple contact/Herd regions and high-tier smoke volumes can exceed a 60 FPS budget. Performance captures and authoring projects are not required to build or run these previews.
