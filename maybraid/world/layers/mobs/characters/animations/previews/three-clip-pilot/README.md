# Three-clip integration pilot

This pilot ports exactly three independently authored clips onto main baseline
`75eaa5d187a81a6855f223121dccc5973795281c`. The source branches contain older shared
infrastructure, so only the selected features are ported. Original pull requests
remain open until the pilot lands; this document does not mark them resolved.

| Source | Exact refreshed head | Authorship and adaptation | Disposition |
| --- | --- | --- | --- |
| [VictoryWave #980](https://github.com/ramate-io/maybraid/pull/980) | `3fca38ba0ed1c3e9b8dd854e89fa7caf2511ff56` | Original feature commits by Cursor Agent; refreshed head by Liam Makena Monninger. Current semantic pose adapter, both-sided production-bind geometry, fixed idle endpoints and a relaxed opposite arm. | Feature port; original stays open pending landing. |
| [HandClap #1043](https://github.com/ramate-io/maybraid/pull/1043) | `aed4ab44e8c0a861f63838112d0ca1e07c3ce088` | Cursor Agent. Adapt bilateral chest contact and pulses to current scaled rig geometry, fixed idle recovery and visible-pose transitions. | Feature port; original stays open pending landing. |
| [QuadrupedRunStop #1317](https://github.com/ramate-io/maybraid/pull/1317) | `6431d56e21616d0883974b21bb66db7bb13579d6` | Cursor Agent. Current quadruped scratch blend and captured-pose transition; current production player and world-player playground own automatic trot-to-still selection. | Feature port including automatic trot stopping; original stays open pending landing. |

The gesture clips are clamped one-shots over progress `[0,1]`, with identity
armature effects. Both gestures start and end at `Idle::default()` phase 0, where
the scratch envelope is zero, so they recover to hanging arms. This adapts the
source clips' raw bind endpoints after inspecting real-character renders.
`VictoryWave` uses a selected side and leaves the other arm relaxed; `HandClap`
is bilateral. Body-skin remap calibrates `rig.forearm_endpoints` from the actual
fully weighted forearm cap: Standard and Full bodies have different visible
terminal lengths. Contact uses the calibrated local points through full affine
FK, rather than an inferred half-meter segment. Missing/unsupported geometry has
an explicit `+Y * 0.5` fallback; that fallback does not establish visible contact.
`QuadrupedRunStop` blends the authored run phase 0 to idle phase 0 over the same
normalized range, with default playback speed 3.5 (about 0.286 seconds). The mailbox
captures the visible arbitrary gait phase on a clip change, so entry and movement
or jump interruption use the existing crossfade. Production player and the
world-player playground select the stop after a grounded trot crosses below the
walk threshold, keep it until completion, and return to idle without restarting it. Gallop stopping is outside
this trot transition's scope.

## Repeatable real-character previews

`characters-playground` uses the production fixed character assembly, meshes,
clothing and shaders. It supports `braidman` and `epiphant`, the `victory-wave`,
`hand-clap` and `quadruped-run-stop` animation names, sided wave selection, paused
progress scrubbing, and front / side / three-quarter camera commands.

```sh
cargo run -p characters-playground --example animation_capture -- \
  --output-dir /tmp/animation-pilot-captures
```

The example accepts repeated `--clip` and `--view` filters. Manual progress
overrides sample the exact requested coordinate, including Still, so idle
endpoint comparisons do not include ordinary crowd phase desynchronization. It captures real assets
through Bevy screenshot readback. On a headless Linux machine, run under Xvfb with
the session's Vulkan software renderer configured. Output images, contact sheets,
videos and logs belong outside git. Inspect all three views, both wave sides, hand
contact/open pulses, recovery near .98/.99, and the stop's entry/midpoint/idle
endpoint. Test output and joint tables alone do not establish a visual pass.

The integration review inspected 123 real-character images across front, side
and three-quarter views: both wave sides, clap opening/contact/recovery, and the
quadruped stop. Visible clap terminals meet, the opposite wave arm remains
relaxed, and the stop endpoints match the independent run/idle references. Pose
sidecars confirmed all 16 quadruped animation bones at both endpoints in all
three views. Captures and contact sheets remain outside git; the integration
review/PR records the run artifacts and test evidence.
