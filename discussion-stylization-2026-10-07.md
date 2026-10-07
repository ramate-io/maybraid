# Animation stylization proposal: Spring arm whip, Idle weight shift, Upright Leap land stagger

**Daily naturalness review — 2026-10-07**

This proposal follows [#965](https://github.com/ramate-io/maybraid/pull/965) (merged anatomical authoring) and deliberately avoids clips already covered by recent stylization work ([#983](https://github.com/ramate-io/maybraid/pull/983) walk knee timing; jab snap and land arms from the same daily batch) and open animation PRs (Fall arm stagger [#996](https://github.com/ramate-io/maybraid/pull/996), crouch-walk blend [#1002](https://github.com/ramate-io/maybraid/pull/1002), walk→run Mix [#981](https://github.com/ramate-io/maybraid/pull/981), prone descent [#1001](https://github.com/ramate-io/maybraid/pull/1001)).

**Clips reviewed:** `Spring`, `Idle`, `UprightLeap`, `Tuck` (humanoid). Quadruped clips were skipped to avoid overlap with gait/perf sibling routines.

**Validation path:** Posed-joint sampling on `HumanoidV0Rig::for_clip_test()` (imported GLB rest from `humanoid_rig.glb`). Playground visual check was **not** available in the cloud agent environment.

**Mirroring check:** Left/right signs verified against `apply_arm` / `apply_leg` in `rigs/humanoid/apply.rs` — positive `shin_flex` and `forearm_flex` on both sides; right shin/forearm `−Z` flex is handled in the resolver (`HUMANOID_RIGHT_SHIN_AXIS`, `HUMANOID_RIGHT_FLEX_AXIS`), not by negating authored values.

---

## PR #965 infrastructure present on `main`

All of the following from [#965](https://github.com/ramate-io/maybraid/pull/965) are on `main` as of `aca6487f`:

| Component | Location |
|-----------|----------|
| `AUTHORING.md` | `maybraid/world/layers/mobs/characters/animations/AUTHORING.md` |
| Authoring module (`HumanoidPose`, `QuadrupedPose`, `ForelimbedPose`, binding, buffer, frame, space) | `maybraid/world/layers/mobs/characters/rigs/src/authoring/` |
| Clip migration to semantic pose sampling | `animations/src/rigs/{humanoid,quadruped,forelimbed,mix,transition}/` |
| `for_clip_test()` imported-rest fixtures | `rigs/src/rigs/{humanoid_v0,quadruped_v0}.rs` |
| Playground scrub/phase/joint-axis/leg-scale | `playground/src/commands/character.rs`, `playground/src/animation.rs` |
| Regression posed-joint tests | `animations/src/rigs/regression.rs` |
| Walk lean routed through `add_root_forward` (twist), not yaw | `apply_root` + `SpinePose::add_root_forward` |

---

## Recommendation 1 — Spring: arms lead leg extension (implemented in draft PR)

**Clip / code:** `Spring` — `animations/src/animations/spring.rs`, rig `rigs/humanoid/spring.rs`. Used in `TwoFootedJump` / `TwoFootedTuckedFlip` spring segment and `Transition::from_pose` blends.

**Issue:** During the squat→extension spring, arms and legs share one `extend_amount` envelope. At mid-extension the knees are still visibly bent while the backward arm sweep is only ~80% complete, so the takeoff reads as “legs pushing” rather than a coordinated whip.

**Change:** Decouple `arm_amount` so arms saturate at 82% of leg extension:

```rust
const ARM_FULL_AT_LEG_EXTENSION: f32 = 0.82;

pub fn arm_amount(&self, progress: f32) -> f32 {
    let leg = self.extend_amount(progress);
    (leg / ARM_FULL_AT_LEG_EXTENSION).min(1.0)
}
```

Leg/root channels unchanged (`femur_swing`, `shin_flex`, `root_swing` still use `extend_amount` only).

**Transitions / mirroring / offsets:** Symmetric on both arms; no `ArmatureOffset` change. Spring→Fall blend in jump unchanged (still samples end pose at `progress = 1.0`).

**Validation:** `cargo test -p character-animations spring`. Posed-joint at `progress = 0.55`, `for_clip_test()`:

| Metric | Before | After |
|--------|--------|-------|
| `forearm.R` tip Z (character space) | −0.382 | −0.433 |
| `shin.L` posed angle | 0.318 | 0.318 (unchanged) |
| `shoulder_swing` semantic | −0.439 | −0.535 |

**Perf:** One extra divide per sample; negligible.

**Evidence:** **Posed-joint numeric** (confirmed). Playground not run.

---

## Recommendation 2 — Idle: phase-opposed hip weight shift

**Clip / code:** `Idle` — `animations/src/animations/idle.rs`, rig `rigs/humanoid/idle.rs`.

**Issue:** Hip shift (`HIP_FREQ = 0.8`, amplitude `0.025`) oscillates independently of arm sway (`ARM_FREQ = 1.0`). Posed pelvis lateral at two quiet phases differs by only ~0.005 rad while shoulders move ~0.013 rad — weight does not visibly follow the arm hang/sway.

**Change:** Drive hip lateral from the negated arm oscillator (keep amplitude cap):

```rust
// In sample_pose:
let arm_phase = (TAU * (progress * Idle::ARM_FREQ)).sin();
let hip = -arm_phase * idle.hip_shift; // replace independent HIP_FREQ sine
```

Optionally bump `DEFAULT_HIP_SHIFT` from `0.025` → `0.035` if still too subtle after phase lock.

**Transitions / mirroring:** `apply_idle_hips` already uses `side.sign()` on pelvis lateral; opposing the arm phase preserves L/R mirroring. No armature offset.

**Validation:** Posed `pelvis.L` / `pelvis.R` should anti-correlate with `shoulder.L` swing at `progress = SCRATCH_DURATION + 0.2`. Playground: `/character playback --pause --progress …` on idle loop.

**Perf:** None.

**Evidence:** **Posed-joint numeric** (hip motion exists but is decoupled). **Code hypothesis** for improvement appeal.

---

## Recommendation 3 — Upright Leap: stagger land knee absorb (lead leg first)

**Clip / code:** `UprightLeap` land segment — `rigs/humanoid/leap.rs` `land()`, parameters in `animations/upright_leap.rs`.

**Issue:** Land absorb applies the same `sin(π·u)` envelope to both shins (`shin * 0.95` on right only). At `progress ≈ 0.86` both knees flex ~0.75 rad symmetrically — readable but flat; a one-frame stagger would sell weight acceptance without touching the standalone `Land` clip (already stylized elsewhere).

**Change:** Offset the right (lead) absorb by ~0.08 normalized land-phase:

```rust
fn land(&self, u: f32) -> LeapPose {
    let absorb_lead = ((u + 0.08) * PI).sin().max(0.0);
    let absorb_trail = (u * PI).sin();
    // apply absorb_lead to right_shin, absorb_trail to left_shin
}
```

Keep femur absorb matched or slightly lead on the same side for consistency with takeoff stride (right lead).

**Transitions / mirroring:** Lead is authorship convention (right = lead throughout `UprightLeap`); signs checked via `apply_leg` symmetric positive flex. No gameplay displacement.

**Validation:** Posed `shin.R` angle > `shin.L` at `progress = 0.86` by ≥0.05 rad; end pose (`progress = 1.0`) still zero. Playground: upright leap clip, scrub 0.8–0.95, side camera.

**Perf:** One extra `sin` per sample.

**Evidence:** **Posed-joint numeric** at symmetric peak; stagger benefit is **code hypothesis** until playground review.

---

## Likely PR conflicts

| Open PR | Overlap risk |
|---------|----------------|
| [#996](https://github.com/ramate-io/maybraid/pull/996) Fall arm stagger | None (Spring segment only) |
| [#963](https://github.com/ramate-io/maybraid/pull/963) jump sampling cache | Low — may touch `two_footed_jump.rs` wiring, not spring pose |
| [#1002](https://github.com/ramate-io/maybraid/pull/1002) crouch-walk | None |
| [#981](https://github.com/ramate-io/maybraid/pull/981) walk/run Mix | None |

---

## Playground checklist (human)

1. `nix develop` → run playground.
2. `/character` → select humanoid → `TwoFootedJump` or isolated `Spring`.
3. `/character playback --pause --progress 0.55 --speed 0.25` — arms should sit further back than knees straighten.
4. Compare idle hip shift (rec 2, if implemented) at quiet vs sway peak.
5. Upright leap land at `--progress 0.86` (rec 3).
