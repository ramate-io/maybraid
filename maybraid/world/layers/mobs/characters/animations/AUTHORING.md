# Authoring character motion

V0 clips sample a semantic pose. The resolver maps those fields onto the imported parent-space swing / flex / twist compose (`R(swing) * R(twist) * R(flex) * rest`). Field names are the authoring API. They are **not** yet a character-space anatomical contract: hip flexion is still femur swing about parent Y, elbow flexion is still forearm flex about parent Z, and walk lean is still spine twist about parent X.

Do not put a minus sign on the right knee. Right shin and right forearm already use `−Z` flex.

Character space is the armature’s local frame: **+X** right, **+Y** up, **+Z** fight-forward. Bevy’s world stays **+Y** up and **−Z** camera-forward.

`/character joint --flexion` drives the same V0 channels as clips: flexion is flex, lateral is swing, axial is twist.

Inspected `humanoid_rig.glb` (production biped bodies share these binds): T-pose `shoulder.L` is `Rz(−90°)`, `shoulder.R` is `Rz(+90°)`, humerus/forearm rest identity, `pelvis.L` is `(-0.5,-0.5,-0.5,0.5)`, femur `Rx(−90°)` at 0.25 m from the pelvis, shin identity at 0.50 m from the femur. Femur length is that shin-origin distance (0.50 m), not the pelvis offset. Parent **Y** (DEFAULT swing) is the previous flap / arm-pump stroke (T-pose +Y sweeps in XZ). Parent **Z** (DEFAULT flex, right forearm `−Z`) is the elbow hinge. Character **X** is the arm length and would only twist those joints.

Inspected `quadruped_rig.glb` (production quadruped bodies share these binds): `anterior_mid_back` `(0, s2, s2, 0)` parents `upper_back`; `shoulder.L` is `Rz(+90°)`; `anterior_thigh` is `Rx(−90°)`; shin identity. The V0 definition still parents `upper_back` to `back_ridge`; judge the live fold as `mid_back * shoulder * thigh * shin`. The old shin hinge is parent **Z** (right `−Z`).

Inspected `forelimbed_rig.glb`: `shoulder.L` is the same `Rz(−90°)` as the humanoid T-pose. Fin sweep is DEFAULT parent-Y swing.

## V0 channels

Angles are radians. These are the knobs the adapter writes:

| Semantic field | Compose channel | Typical imported axis |
| --- | --- | --- |
| `shoulder_forward`, `hip_flexion`, stride | swing | parent Y, right femur `−Y` |
| `elbow_flexion`, `knee_flexion`, hinge, `shoulder_lift` | flex | parent Z, right shin/forearm `−Z` |
| spine `forward_bend`, nod, pelvis flexion | twist | parent X |

Walk lean uses `add_root_forward` → twist so the torso pitches. That is an intentional correction of the old walk yaw. Held and looping squat share `add_stack_forward` (0.28, 0.26, 0.24, 0.22). Jab waist is `add_waist_forward` (0.40, 0.35, 0.15, 0.10). `add_turn` yaws lumbar, mid-back, and upper back.

A jab aim is a character-space direction on the humerus, plus a roll. The elbow stays V0 forearm flex.

Flapping keeps the previous parent-Y front/back stroke. A vertical wing beat would be a separate authoring change.

## What a sample may depend on

A sample reads the clip definition, the effective rest, the clip parameters, and progress. Sampling again does not accumulate. Bones the clip does not write stay at rest. `ArmatureOffset::IDENTITY` means the armature is not visually offset. Gameplay locomotion is not that offset.

Identity rest is not a production T-pose. Clip tests that need bind geometry use `HumanoidV0Rig::for_clip_test()` / `QuadrupedV0Rig::for_clip_test()`, which seed the inspected local translations and rotations.

Quadruped animation locals omit the fixed front/rear midback branches. Their typed FK coordinates are not world coordinates. Support/height tests must include those branches and species proportions; the shared [Epiphant test fixture](src/rigs/quadruped/production_fixture.rs) reproduces the captured LIVE affines. Use world Y for height and inspect the rendered fold before accepting angle-only assertions.

Segment lengths are joint-to-joint. Femur length is the shin origin’s translation. When no distal joint or calibrated mesh terminal is available, the explicit family fallback is 0.5 m. A fallback does not prove visible mesh contact.

## Playground

After `/`:

- `/character assemble --species braidman --animation think-again` (or `jab`, `still`, `walk`, `run`, …) spawns the default clothed Braidman preview (parts + character shaders) through the runtime fixed-assembly path. Sided gestures take `--side left|right`. Tanz clips: `jab`, `think-again`, `power-strafe --side left|right`, `power-dash`.
- `/character animate think-again --side right` switches the mailbox clip without respawning. The sample clock restarts at 0 so a one-shot reads from the first frame.
- `/character playback --pause` freezes the clock. `--resume` starts it. `--speed 0.5` scales time. `--progress 0.4` scrubs to 0.4 seconds. `--once` plays a one-shot through and holds the end (`think-again` is four clip-seconds at 2×, so about two seconds; other one-shots stop at one second). `--looping` (the default) wraps `think-again` and other clamp clips so they repeat. `--phase 0.5` samples the opposite gait phase. `--leg-scale 0.75` or `--leg-scale 1.25` scales femur and shin rest translations before sampling. Femur length follows the scaled shin origin.
- `/character camera front`, `side`, or `three-quarter` frames the rig around its hip-centered origin. Defaults use distance 5 for Braidman and 7 for Epiphant. Override framing with, for example, `/character camera side --distance 8 --height 0.2`; height is the target offset above the assembled character origin.
- `/character joint femur.L` draws bone-local axes (bright RGB), parent-local axes (shorter, dimmer), and character-space axes on the rig. The opposite bone is drawn dimmer so the two sides can be compared. A yellow line is the effective-rest length direction. A white cross marks the posed segment end. `--hide-rest` drops the rest line.
- `/character joint femur.L --flexion 40 --lateral 0 --axial 0` replaces that bone's clip rotation with the V0 flex / swing / twist channels. `--clear` returns the bone to the clip.

To inspect a new one-shot (salute, shrug), add the clip to `AnimationMode` and map it in `anim_clip` — `/character animate` and playback/joint then work without a respawn.


## Three-clip pilot

`VictoryWave`, `HandClap`, and `QuadrupedRunStop` are clamped one-shots. Their
current source mapping and repeatable real-character capture command are in
[`previews/three-clip-pilot`](previews/three-clip-pilot/README.md).
`/character assemble --species epiphant --animation quadruped-run-stop` previews
the quadruped using the same fixed-assembly path as Braidman. For the wave,
`--animation victory-wave --side left|right` selects the arm; `hand-clap` is
bilateral. Scrub normalized progress with `/character playback --progress 0.54 --pause`.

For sided anatomical assertions, check `Side::sign()` and the composed imported
bind joints. In the current humanoid bind, the left authored arm extends toward
+X and the right toward −X. The generic character-frame +X label above does not
establish which way an anatomical left/right limb must move. For visible hand contact, use calibrated `rig.forearm_endpoints`: the body skin
remap measures the actual fully weighted distal cap in each forearm's bind-local
space. `+Y * 0.5` is only the explicit missing/unsupported-geometry fallback; it
is not a validated mesh endpoint. The forearm origin's parent offset is also not
the distal segment length.

When comparing quaternions near rest, normalize them and compare the absolute dot
product with a tolerance; `angle_between` can report a small nonzero angle for
numerically identical imported bind rotations. For a fast recovery, compare
60 Hz endpoint steps with 120/240 Hz samples: shrinking steps distinguish peak
speed from a discontinuity. Render the actual character before judging the motion.

The pilot gestures use `Idle::default().sample_pose(0.0)` as their fixed neutral
pose (the idle scratch envelope is zero there), rather than the imported T-pose.
For contact gestures, test the full affine distal segment under actual proportion
scales: Braidman includes an ancestor scale of 0.8 and humerus scale (0.8, 1, 0.8).
A rotation-only normalized bone direction loses those lengths and can report
contact while the rendered hands remain apart.

An explicit `AnimProgress` uses that exact sample coordinate, including Still.
Manual playback scrubbing therefore compares authored idle endpoints consistently;
automatic crowd idle retains its per-entity phase offset.

Body skin calibration is invalidated when its source skin is replaced or its
mesh/inverse-bind asset changes. Pending assets retain the existing remap retry;
missing or unsupported geometry keeps the documented fallback. Contact sampling
uses full affine transforms through `RigDefinition::affine_in_character`, so
ancestor scale and shear participate in the actual visible endpoint.
