# Authoring character motion

Clips sample an anatomical pose. The rig resolves that pose into bone-local rotations from the effective rest. Authors do not pick imported skeleton axes, and they do not put a minus sign on the right knee to mirror the left one.

Character space is the armature’s local frame: **+X** right, **+Y** up, **+Z** fight-forward. Bevy’s world stays **+Y** up and **−Z** camera-forward. Convert positions and directions with the character `GlobalTransform`. Do not send a direction through point conversion.

Clip evaluation does **not** conjugate that triad through every joint. The previous working clips composed imported [`RiggedAxis`](../../rigs/src/lib.rs) swing / flex / twist in the bone’s parent space (`R(swing) * R(twist) * R(flex) * rest`). Semantic pose fields are an authoring adapter onto those knobs. `/character joint --flexion` still previews a character-space frame for gizmos; that preview is not the clip solver.

Inspected `humanoid_rig.glb` (production biped bodies share these binds): T-pose `shoulder.L` is `Rz(−90°)`, `shoulder.R` is `Rz(+90°)`, humerus/forearm rest identity, `pelvis.L` is `(-0.5,-0.5,-0.5,0.5)`, femur `Rx(−90°)`, shin identity. Parent **Y** (DEFAULT swing) is the previous flap / arm-pump stroke (T-pose +Y sweeps in XZ). Parent **Z** (DEFAULT flex, right forearm `−Z`) is the elbow hinge. Character **X** is the arm length and would only twist those joints.

Inspected `quadruped_rig.glb` (production quadruped bodies share these binds): `anterior_mid_back` `(0, s2, s2, 0)` parents `upper_back`; `shoulder.L` is `Rz(+90°)`; `anterior_thigh` is `Rx(−90°)`; shin identity. The V0 definition still parents `upper_back` to `back_ridge`; judge the live fold as `mid_back * shoulder * thigh * shin`. The old shin hinge is parent **Z** (right `−Z`).

Inspected `forelimbed_rig.glb`: `shoulder.L` is the same `Rz(−90°)` as the humanoid T-pose. Fin sweep is DEFAULT parent-Y swing.

## Channels

Angles are radians.

| Control | Axis | Positive motion |
| --- | --- | --- |
| Forward bend, hip flexion, knee flexion, nod | +X | A +Y bone tips toward fight-forward +Z |
| Side bend, abduction, side tilt | +Z | A +Y bone tips toward −X |
| Turn, axial rotation | +Y | Fight-forward +Z tips toward −X |

The same positive knee flexion closes both knees. Gait phase is `Side::phase_offset` (0 on the left, half a cycle on the right). Opposite `arm_down` and hip-lift signs are hang and hike bias. They are not axis mirrors.

Spine helpers:

- `add_root_forward` puts the whole angle on the root. Walk lean uses this, so the torso tips forward instead of yawing.
- `add_stack_forward` splits a squat fold across root, lumbar, mid-back, and upper back with weights 0.28, 0.26, 0.24, 0.22. Held and looping squat both use it.
- `add_waist_forward` is the jab waist fold (0.40, 0.35, 0.15, 0.10).
- `add_turn` yaws lumbar, mid-back, and upper back (0, 0.35, 0.40, 0.25). Root lean is not that yaw.

A jab aim is a character-space direction on the humerus, plus a roll. The elbow stays a bone-local flexion.

## What a sample may depend on

A sample reads the clip definition, the effective rest, the clip parameters, and progress. Sampling again does not accumulate. Bones the clip does not write stay at rest. `ArmatureOffset::IDENTITY` means the armature is not visually offset. Gameplay locomotion is not that offset.

Identity rest is not a production T-pose. Clip tests that need bind geometry use `HumanoidV0Rig::for_clip_test()`, which seeds the inspected rotations above.

Segment lengths come from the effective-rest translation. A length near zero keeps the family default (0.5 m for a humanoid femur or shin). Editing rest refreshes the calibrated frames and those lengths.

## Playground

After `/`:

- `/character playback --pause` freezes the clock. `--resume` starts it. `--speed 0.5` scales time. `--progress 0.4` scrubs to 0.4 seconds. `--once` stops at one second. `--looping` keeps running. `--phase 0.5` samples the opposite gait phase. `--leg-scale 0.75` or `--leg-scale 1.25` shortens or lengthens the femur and shin before sampling.
- `/character camera front`, `side`, or `three-quarter` frames the rig.
- `/character joint femur.L` draws bone-local axes (bright RGB), parent-local axes (shorter, dimmer), and character-space axes on the rig. The opposite bone is drawn dimmer so the two sides can be compared. A yellow line is the effective-rest length direction. A white cross marks the posed segment end. `--hide-rest` drops the rest line.
- `/character joint femur.L --flexion 40 --lateral 0 --axial 0` replaces that bone's clip rotation with those anatomical degrees. `--clear` returns the bone to the clip.

Positive flexion should move the green bone axis toward the blue character axis. Hand and foot target markers are not drawn; a jab aim is a character-space direction on the humerus, checked by the geometric tests.
