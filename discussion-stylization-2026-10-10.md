# Stylization review 2026-10-10: idle weight, leap land arms, flapping downstroke

Daily Maybraid stylization / naturalness pass on `origin/main` @ `18f4986d` (post [#1080](https://github.com/ramate-io/maybraid/pull/1080) animation playground). Evidence labels: **visual** = playground/silhouette confirmed; **posed-joint** = `HumanoidV0Rig::for_clip_test()` / `imported()` numeric samples; **code** = read resolver/masking only.

## PR #965 authoring infrastructure on `main`

**Present (merged [#965](https://github.com/ramate-io/maybraid/pull/965)):**

- [`animations/AUTHORING.md`](maybraid/world/layers/mobs/characters/animations/AUTHORING.md): V0 channel table, inspected GLB bind notes, playground commands, non-accumulating sampling rules.
- Clip + rig pairing refresh across humanoid/quadruped/forelimbed clips (`upright_walk`, `jab`, `land`, `spring`, `tuck`, transitions, etc.).
- Humanoid resolver apply paths under `rigs/humanoid/*` aligned with semantic pose fields (still parent-space swing/flex/twist — not a full anatomical retarget).
- Shared helpers (`mix`, `transition`, tuck profiles) and downstream consumers (`hold`, `stance`).

**Not missing for this review:** no need to re-land #965 frameworks. Playground wiring for `/character assemble` / playback landed via [#1080](https://github.com/ramate-io/maybraid/pull/1080).

**Overlap avoided:** open draft PRs on walk/run transitions, jab snap ([#1042](https://github.com/ramate-io/maybraid/pull/1042)), land/leap blends ([#1044](https://github.com/ramate-io/maybraid/pull/1044), [#1081](https://github.com/ramate-io/maybraid/pull/1081)), fall/spring/run lean, gestures, and gait `Mix` ([#981](https://github.com/ramate-io/maybraid/pull/981)). Walk swing-knee timing from prior dailies is already on `main` via `gait_knee.rs`.

---

## Recommendation 1 — Idle pelvis shift phases with arm sway neutral (implemented in draft PR)

| | |
|---|---|
| **Clip / location** | `Idle` — [`animations/idle.rs`](maybraid/world/layers/mobs/characters/animations/src/animations/idle.rs), sampling in [`rigs/humanoid/idle.rs`](maybraid/world/layers/mobs/characters/animations/src/rigs/humanoid/idle.rs) |
| **Issue / intent** | Idle weight bob reads mushy when pelvis lateral and shoulder sway peak together. Shift pelvis peak to arm-neutral for stylized contralateral weight (not mocap). |
| **Change** | `HIP_FREQ` 0.8→1.0, `HIP_WEIGHT_PHASE = -0.25` cycles, `hip_shift` 0.025→0.032 rad. Hip oscillator uses `sin(TAU * (progress * HIP_FREQ + HIP_WEIGHT_PHASE))`. |
| **Transitions / mirroring** | Clip-only; `idle_write_mask` unchanged. `pelvis_lateral` still `amount * side.sign()` — no extra L/R inversion. `SquatDescent` / idle-walk blends inherit new idle phase when they sample `Idle`. |
| **Validation / perf** | New `for_clip_test` assertions; zero per-frame cost. **Visual:** not performed in agent VM. |
| **Evidence** | **posed-joint:** before `main` @ progress 0.25/0.5 — `pelvis.L` 0.0147 / 0.0238, `shoulder.L` 0.0600 at 0.25; after — 0.0000 / 0.0320 at same samples. |

```rust
// animations/idle.rs
pub const HIP_FREQ: f32 = 1.0;
pub const HIP_WEIGHT_PHASE: f32 = -0.25;
const DEFAULT_HIP_SHIFT: f32 = 0.032;

// rigs/humanoid/idle.rs
let hip = (TAU * (progress * Idle::HIP_FREQ + Idle::HIP_WEIGHT_PHASE)).sin();
```

Playground: `/character assemble --animation idle`, `--playback --pause`, scrub `--progress 0.25` vs `0.5`, `/character camera three-quarter`, `/character joint pelvis.L`.

---

## Recommendation 2 — Upright leap land arm counterbalance peaks with leg absorb

| | |
|---|---|
| **Clip / location** | `UprightLeap::land` — [`rigs/humanoid/leap.rs`](maybraid/world/layers/mobs/characters/animations/src/rigs/humanoid/leap.rs), knobs in [`animations/upright_leap.rs`](maybraid/world/layers/mobs/characters/animations/src/animations/upright_leap.rs) |
| **Issue / intent** | Land segment uses `arm = air_arm * 0.4 * (1 - u)` so humerus drive **weakens** as `sin(π u)` leg absorb peaks — silhouette collapses at the punchiest frame. |
| **Change** | Add `land_arm` (or scale) and multiply by `absorb = sin(π u)` so arms punch forward/down at mid-land, decaying into recovery. Keep `Leap::from_leap` gather scaling on leg fields only unless we want arms tied to gather. |
| **Transitions / mirroring** | One-shot `Leap` only; no root motion. Left/right humerus already both negative-forward with slight asymmetry (`-arm` vs `-arm * 0.8`) — verify signs after change. **Note:** draft [#1044](https://github.com/ramate-io/maybraid/pull/1044) touches `leap.rs`; land absorb should stay coordinated with jump land blends. |
| **Validation / perf** | Extend `land_absorbs_then_recovers` with `for_clip_test` humerus at `progress ≈ 0.86`. Playground: `/character animate leap` (if wired) or assemble leap, scrub land third. **Visual:** not performed. |
| **Evidence** | **code** + **posed-joint** on `imported()` at `progress 0.86` femur absorb (existing test); humerus magnitude at absorb mid not yet asserted. |

---

## Recommendation 3 — Flapping downstroke snap (asymmetric stroke envelope)

| | |
|---|---|
| **Clip / location** | `Flapping::flap_amount` — [`animations/flapping.rs`](maybraid/world/layers/mobs/characters/animations/src/animations/flapping.rs), wings via [`rigs/humanoid/flapping.rs`](maybraid/world/layers/mobs/characters/animations/src/rigs/humanoid/flapping.rs) + `wing` helpers |
| **Issue / intent** | Pure `sin` stroke is symmetric; stylized flight reads snappier with a faster downstroke and softer upstroke (still parent-Y flap per AUTHORING). |
| **Change** | Replace raw sine with a biased envelope (e.g. `sin` remapped through `smoothstep` on negative half, or `flap_amount = sin(θ) - k * sin(2θ)` with small `k`) — **clip constants only**, keep `speed` / `range`. |
| **Transitions / mirroring** | Wings use shared `apply_flight_wings`; L/R should remain mirrored through resolver. Re-check `for_clip_test` forearm tip Z sweep test after remapping. |
| **Validation / perf** | Update `flapping_sweeps_t_pose_wings_in_z`; playground bird assembly if available. **Visual:** not performed. |
| **Evidence** | **posed-joint** half-cycle shoulder separation on `imported()` today; asymmetry **code hypothesis** until envelope lands. |

---

## Agent implementation note

Draft PR implements **recommendation 1** only (clip tuning, no resolver edits).
